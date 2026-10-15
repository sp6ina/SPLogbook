// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Bezpieczny programowy arbiter nadawania TX dla stacji wieloradiowej (NATIVE-6A).
//!
//! Gwarantuje maksymalnie jednego lokalnego właściciela nadajnika ([`TxArbiterState::Owned`])
//! w całej stacji. Zapobiega współbieżnemu kluczowaniu dwóch radii, realizuje dwufazowy
//! protokół PTT oraz wymusza bezpieczny stan awaryjny [`TxArbiterState::Unknown`]
//! w przypadku niejednoznaczności, zerwania transportu lub przekroczenia limitu czasu (timeout).
//!
//! # Ograniczenie odpowiedzialności (Safety Notice)
//! Niniejszy arbiter programowy stanowi logiczną blokadę w warstwie aplikacji SPLogbook.
//! Nie zastępuje fizycznego, sprzętowego interlocku PTT (np. mikroprocesorowej lub przekaźnikowej
//! blokady sprzętowej na liniach PTT/Inhibit), wymaganego w instalacjach dużej mocy lub SO2R.

use std::fmt;

use crate::cat::routing::{RigCommandId, RigId};

/// Zastępuje wrażliwe dane w komunikatach diagnostycznych (hasła, tokeny, tajemnice).
pub fn redact_sensitive_info(msg: &str) -> String {
    let mut out = msg.to_string();
    for pattern in &[
        "password=",
        "password:",
        "token=",
        "token:",
        "secret=",
        "secret:",
        "client_id=",
        "pass=",
        "bearer ",
    ] {
        let mut search_from = 0;
        while search_from < out.len() {
            let lower = out[search_from..].to_lowercase();
            if let Some(pos) = lower.find(pattern) {
                let abs_pos = search_from + pos;
                let start = abs_pos + pattern.len();
                let end = out[start..]
                    .find(|c: char| {
                        c == '&'
                            || c == ' '
                            || c == ';'
                            || c == ','
                            || c == '"'
                            || c == '\''
                            || c == '\n'
                    })
                    .map(|p| start + p)
                    .unwrap_or(out.len());
                let replacement = "[REDACTED]";
                out.replace_range(start..end, replacement);
                search_from = start + replacement.len();
            } else {
                break;
            }
        }
    }
    out
}

/// Wynik wykonania polecenia PTT przez backend CAT (ETAP 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttCommandOutcome {
    /// Fizycznie potwierdzony stan nadawania (TX).
    ConfirmedTx,
    /// Fizycznie potwierdzony stan odbioru (RX).
    ConfirmedRx,
    /// Polecenie zostało odrzucone przez urządzenie lub backend.
    Rejected,
    /// Polecenie zostało wysłane lub przyjęte, ale stan fizyczny TX/RX nie jest jeszcze potwierdzony.
    Unconfirmed,
}

/// Przyczyny przejścia arbitra do stanu niejednoznaczności [`TxArbiterState::Unknown`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxUnknownReason {
    /// Przekroczono limit czasu oczekiwania na potwierdzenie załączenia PTT.
    Timeout,
    /// Przekroczono limit czasu komendy PTT (synonim [`Self::Timeout`]).
    CommandTimeout,
    /// Utrata transportu (zamknięcie gniazda TCP, zerwanie portu COM) w trakcie żądania lub nadawania.
    TransportLoss,
    /// Błąd transportu komunikacyjnego (synonim [`Self::TransportLoss`]).
    TransportError,
    /// Zamknięcie kanału komunikacyjnego z aktorem sesji.
    ChannelClosed,
    /// Przekroczono limit czasu podczas próby wyłączenia nadawania (release timeout).
    ReleaseTimeout,
    /// Backend radiowy zwrócił błąd protokołu podczas operacji PTT.
    BackendError(String),
    /// Stan telemetrii radia nie potwierdza jednoznacznie odbioru (RX).
    UnconfirmedState,
    /// Wymagana ręczna lub programowa resynchronizacja po awarii.
    ManualResyncRequired,
}

impl fmt::Display for TxUnknownReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout | Self::CommandTimeout => {
                write!(
                    f,
                    "przekroczono limit czasu oczekiwania na potwierdzenie TX"
                )
            }
            Self::TransportLoss | Self::TransportError => {
                write!(f, "utrata transportu podczas operacji nadawczej")
            }
            Self::ChannelClosed => write!(f, "zamknięto kanał aktora sesji"),
            Self::ReleaseTimeout => {
                write!(
                    f,
                    "przekroczono limit czasu podczas wyłączania PTT (release)"
                )
            }
            Self::BackendError(msg) => {
                write!(f, "błąd backendu radiowego: {}", redact_sensitive_info(msg))
            }
            Self::UnconfirmedState => {
                write!(f, "telemetria radia nie potwierdza jednoznacznie stanu RX")
            }
            Self::ManualResyncRequired => write!(f, "wymagana jawna resynchronizacja"),
        }
    }
}

/// Stany maszyny arbitrażu nadawania TX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxArbiterState {
    /// Żadne radio nie nadaje ani nie oczekuje na autoryzację TX. Dopuszczalne nowe żądania.
    Idle,
    /// Zgłoszono żądanie `SetPtt(true)` dla wskazanego radia. Oczekiwanie na potwierdzenie backendu.
    Requesting {
        /// Radio żądające prawa do nadawania.
        rig_id: RigId,
        /// Identyfikator polecenia PTT.
        command_id: RigCommandId,
    },
    /// Radio uzyskało potwierdzenie nadawania i jest jedynym lokalnym właścicielem TX.
    Owned {
        /// Radio posiadające wyłączność na nadawanie.
        rig_id: RigId,
        /// Identyfikator polecenia, które przyznało stan Owned.
        command_id: RigCommandId,
    },
    /// Właściciel zgłosił `SetPtt(false)`. Oczekiwanie na potwierdzenie przejścia radia w stan RX.
    Releasing {
        /// Radio zwalniające nadawanie.
        rig_id: RigId,
        /// Identyfikator polecenia zwalniającego.
        command_id: RigCommandId,
    },
    /// Stan niejednoznaczny (np. po timeout, utracie transportu). Nowe żądania TX są bezwzględnie zablokowane.
    Unknown {
        /// Opcjonalne radio powiązane z niejednoznacznością stanu.
        rig_id: Option<RigId>,
        /// Przyczyna wejścia w stan Unknown.
        reason: TxUnknownReason,
    },
}

impl fmt::Display for TxArbiterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Requesting { rig_id, command_id } => {
                write!(f, "Requesting({rig_id}, {command_id})")
            }
            Self::Owned { rig_id, command_id } => write!(f, "Owned({rig_id}, {command_id})"),
            Self::Releasing { rig_id, command_id } => {
                write!(f, "Releasing({rig_id}, {command_id})")
            }
            Self::Unknown { rig_id, reason } => {
                if let Some(id) = rig_id {
                    write!(f, "Unknown({id}: {reason})")
                } else {
                    write!(f, "Unknown({reason})")
                }
            }
        }
    }
}

/// Błędy operacji arbitrażu nadawania.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxArbiterError {
    /// Inne radio oczekuje obecnie na autoryzację nadawania lub nadaje.
    Busy { current_owner: RigId },
    /// Inne radio oczekuje obecnie na autoryzację nadawania.
    AlreadyRequesting { current_rig: RigId },
    /// Inne radio jest już właścicielem nadajnika.
    AlreadyOwned { owner: RigId },
    /// Radio próbujące wykonać operację nie jest aktualnym właścicielem TX.
    NotOwner { rig_id: RigId, owner: Option<RigId> },
    /// Niezgodność właściciela nadajnika.
    OwnerMismatch {
        expected: RigId,
        actual: Option<RigId>,
    },
    /// Arbiter znajduje się w stanie Unknown – wymagana resynchronizacja.
    TxStateUnknown {
        rig_id: Option<RigId>,
        reason: TxUnknownReason,
    },
    /// Arbiter znajduje się w stanie Unknown.
    StateUnknown { reason: TxUnknownReason },
    /// Niedozwolone przejście maszyny stanów.
    InvalidTransition { from: String, to: String },
    /// Resynchronizacja nie powiodła się.
    ResyncFailed(String),
}

impl fmt::Display for TxArbiterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy { current_owner } => {
                write!(f, "nadajnik jest zajęty przez radio '{current_owner}'")
            }
            Self::AlreadyRequesting { current_rig } => {
                write!(f, "radio '{current_rig}' oczekuje na autoryzację TX")
            }
            Self::AlreadyOwned { owner } => {
                write!(f, "nadajnik jest już zablokowany przez radio '{owner}'")
            }
            Self::NotOwner { rig_id, owner } => write!(
                f,
                "radio '{rig_id}' nie jest właścicielem TX (aktualny właściciel: {owner:?})"
            ),
            Self::OwnerMismatch { expected, actual } => write!(
                f,
                "niezgodność właściciela TX: oczekiwano '{expected}', aktualny '{actual:?}'"
            ),
            Self::TxStateUnknown { rig_id, reason } => write!(
                f,
                "stan TX jest nieznany (Unknown, radio: {rig_id:?}, przyczyna: {reason})"
            ),
            Self::StateUnknown { reason } => {
                write!(f, "stan TX jest nieznany (Unknown: {reason})")
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "niedozwolone przejście stanu arbitra: {from} -> {to}")
            }
            Self::ResyncFailed(msg) => write!(f, "resynchronizacja TX nie powiodła się: {msg}"),
        }
    }
}

impl std::error::Error for TxArbiterError {}

/// Główny arbiter nadawania TX dla stacji wieloradiowej.
#[derive(Debug, Clone)]
pub struct TxArbiter {
    state: TxArbiterState,
    unknown_since: Option<tokio::time::Instant>,
    generation: u64,
}

impl Default for TxArbiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TxArbiter {
    /// Tworzy nowy arbiter w bezpiecznym stanie początkowym [`TxArbiterState::Idle`].
    pub fn new() -> Self {
        Self {
            state: TxArbiterState::Idle,
            unknown_since: None,
            generation: 0,
        }
    }

    /// Monotoniczny znacznik czasu wejścia w stan Unknown, o ile arbiter aktualnie w nim przebywa.
    pub fn unknown_since(&self) -> Option<tokio::time::Instant> {
        self.unknown_since
    }

    /// Monotoniczny numer generacji stanu arbitra.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Aktualny stan maszyny arbitra.
    pub fn state(&self) -> &TxArbiterState {
        &self.state
    }

    /// Identyfikator aktualnego właściciela nadajnika, jeśli stan to [`TxArbiterState::Owned`].
    pub fn current_owner(&self) -> Option<&RigId> {
        match &self.state {
            TxArbiterState::Owned { rig_id, .. } => Some(rig_id),
            _ => None,
        }
    }

    /// Identyfikator radia zaangażowanego w operację nadawania (`Requesting`, `Owned`, `Releasing`).
    pub fn active_target(&self) -> Option<&RigId> {
        match &self.state {
            TxArbiterState::Requesting { rig_id, .. }
            | TxArbiterState::Owned { rig_id, .. }
            | TxArbiterState::Releasing { rig_id, .. } => Some(rig_id),
            _ => None,
        }
    }

    /// Sprawdza, czy arbiter znajduje się w stanie spoczynku ([`TxArbiterState::Idle`]).
    pub fn is_idle(&self) -> bool {
        matches!(self.state, TxArbiterState::Idle)
    }

    /// Sprawdza, czy wskazane radio jest aktualnym właścicielem TX.
    pub fn is_owned_by(&self, rig_id: &RigId) -> bool {
        match &self.state {
            TxArbiterState::Owned { rig_id: owner, .. } => owner == rig_id,
            _ => false,
        }
    }

    /// Sprawdza, czy wskazane radio może ubiegać się o nadawanie.
    pub fn can_request(&self, rig_id: &RigId) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Idle => Ok(()),
            TxArbiterState::Requesting {
                rig_id: current, ..
            } => Err(TxArbiterError::Busy {
                current_owner: current.clone(),
            }),
            TxArbiterState::Owned { rig_id: owner, .. } => {
                if owner == rig_id {
                    // Już jest właścicielem
                    Ok(())
                } else {
                    Err(TxArbiterError::Busy {
                        current_owner: owner.clone(),
                    })
                }
            }
            TxArbiterState::Releasing {
                rig_id: current, ..
            } => Err(TxArbiterError::Busy {
                current_owner: current.clone(),
            }),
            TxArbiterState::Unknown { reason, .. } => Err(TxArbiterError::StateUnknown {
                reason: reason.clone(),
            }),
        }
    }

    /// Rozpoczyna procedurę żądania nadawania: przejście `Idle -> Requesting`.
    ///
    /// # Błędy
    /// Zwraca błąd, jeżeli arbiter nie znajduje się w stanie `Idle`.
    pub fn begin_request(
        &mut self,
        rig_id: RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Idle => {
                self.state = TxArbiterState::Requesting { rig_id, command_id };
                Ok(())
            }
            TxArbiterState::Requesting {
                rig_id: current, ..
            } => Err(TxArbiterError::Busy {
                current_owner: current.clone(),
            }),
            TxArbiterState::Owned { rig_id: owner, .. } => Err(TxArbiterError::Busy {
                current_owner: owner.clone(),
            }),
            TxArbiterState::Releasing {
                rig_id: current, ..
            } => Err(TxArbiterError::Busy {
                current_owner: current.clone(),
            }),
            TxArbiterState::Unknown { reason, .. } => Err(TxArbiterError::StateUnknown {
                reason: reason.clone(),
            }),
        }
    }

    /// Pomocnicza metoda żądania nadawania (synonim [`Self::begin_request`]).
    pub fn request_tx(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        self.begin_request(rig_id.clone(), command_id)
    }

    /// Potwierdza pomyślne załączenie PTT przez backend: przejście `Requesting -> Owned`.
    ///
    /// Wymaga, aby potwierdzenie dotyczyło dokładnie tego samego radia i tego samego polecenia.
    pub fn confirm_owned(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Requesting {
                rig_id: req_rig,
                command_id: req_cmd,
            } if req_rig == rig_id && *req_cmd == command_id => {
                self.state = TxArbiterState::Owned {
                    rig_id: rig_id.clone(),
                    command_id,
                };
                Ok(())
            }
            other => Err(TxArbiterError::InvalidTransition {
                from: other.to_string(),
                to: format!("Owned({rig_id}, {command_id})"),
            }),
        }
    }

    /// Pomocnicza metoda potwierdzenia nadawania (synonim [`Self::confirm_owned`]).
    pub fn confirm_tx(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        self.confirm_owned(rig_id, command_id)
    }

    /// Odrzuca żądanie nadawania (np. odmowa backendu): przejście `Requesting -> Idle`.
    pub fn reject_request(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Requesting {
                rig_id: req_rig,
                command_id: req_cmd,
            } if req_rig == rig_id && *req_cmd == command_id => {
                self.state = TxArbiterState::Idle;
                Ok(())
            }
            other => Err(TxArbiterError::InvalidTransition {
                from: other.to_string(),
                to: "Idle".to_string(),
            }),
        }
    }

    /// Rozpoczyna zwalnianie nadawania przez właściciela: przejście `Owned -> Releasing`.
    pub fn begin_release(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Owned { rig_id: owner, .. } if owner == rig_id => {
                self.state = TxArbiterState::Releasing {
                    rig_id: rig_id.clone(),
                    command_id,
                };
                Ok(())
            }
            TxArbiterState::Owned { rig_id: owner, .. } => Err(TxArbiterError::OwnerMismatch {
                expected: owner.clone(),
                actual: Some(rig_id.clone()),
            }),
            other => Err(TxArbiterError::OwnerMismatch {
                expected: match other {
                    TxArbiterState::Requesting { rig_id: r, .. }
                    | TxArbiterState::Releasing { rig_id: r, .. } => r.clone(),
                    _ => rig_id.clone(),
                },
                actual: Some(rig_id.clone()),
            }),
        }
    }

    /// Pomocnicza metoda rozpoczęcia zwalniania nadawania (synonim [`Self::begin_release`]).
    pub fn request_release(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        self.begin_release(rig_id, command_id)
    }

    /// Potwierdza zakończenie nadawania i powrót radia do stanu RX: przejście `Releasing -> Idle`.
    pub fn confirm_release(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Releasing {
                rig_id: rel_rig,
                command_id: rel_cmd,
            } if rel_rig == rig_id && *rel_cmd == command_id => {
                self.state = TxArbiterState::Idle;
                Ok(())
            }
            other => Err(TxArbiterError::InvalidTransition {
                from: other.to_string(),
                to: "Idle".to_string(),
            }),
        }
    }

    /// Aplikuje wynik polecenia PTT z zachowaniem rygorów bezpieczeństwa (ETAP 5).
    pub fn apply_ptt_outcome(
        &mut self,
        rig_id: &RigId,
        command_id: RigCommandId,
        outcome: PttCommandOutcome,
    ) -> Result<(), TxArbiterError> {
        match (&self.state, outcome) {
            (
                TxArbiterState::Requesting {
                    rig_id: req_rig,
                    command_id: req_cmd,
                },
                PttCommandOutcome::ConfirmedTx,
            ) if req_rig == rig_id && *req_cmd == command_id => {
                self.state = TxArbiterState::Owned {
                    rig_id: rig_id.clone(),
                    command_id,
                };
                self.generation += 1;
                Ok(())
            }
            (
                TxArbiterState::Requesting {
                    rig_id: req_rig,
                    command_id: req_cmd,
                },
                PttCommandOutcome::Rejected,
            ) if req_rig == rig_id && *req_cmd == command_id => {
                self.state = TxArbiterState::Idle;
                self.generation += 1;
                Ok(())
            }
            (
                TxArbiterState::Requesting {
                    rig_id: req_rig,
                    command_id: req_cmd,
                },
                PttCommandOutcome::Unconfirmed,
            ) if req_rig == rig_id && *req_cmd == command_id => {
                self.set_unknown(Some(rig_id.clone()), TxUnknownReason::UnconfirmedState);
                Err(TxArbiterError::StateUnknown {
                    reason: TxUnknownReason::UnconfirmedState,
                })
            }
            (
                TxArbiterState::Releasing {
                    rig_id: rel_rig,
                    command_id: rel_cmd,
                },
                PttCommandOutcome::ConfirmedRx,
            ) if rel_rig == rig_id && *rel_cmd == command_id => {
                self.state = TxArbiterState::Idle;
                self.generation += 1;
                Ok(())
            }
            (
                TxArbiterState::Releasing {
                    rig_id: rel_rig,
                    command_id: rel_cmd,
                },
                PttCommandOutcome::Unconfirmed,
            ) if rel_rig == rig_id && *rel_cmd == command_id => {
                self.set_unknown(Some(rig_id.clone()), TxUnknownReason::UnconfirmedState);
                Err(TxArbiterError::StateUnknown {
                    reason: TxUnknownReason::UnconfirmedState,
                })
            }
            (other, _) => Err(TxArbiterError::InvalidTransition {
                from: other.to_string(),
                to: format!("Outcome({outcome:?})"),
            }),
        }
    }

    /// Wymusza przejście do stanu `Unknown` w przypadku timeoutu, zerwania transportu lub błędu I/O.
    pub fn set_unknown(&mut self, rig_id: Option<RigId>, reason: TxUnknownReason) {
        let redacted_reason = match reason {
            TxUnknownReason::BackendError(msg) => {
                TxUnknownReason::BackendError(redact_sensitive_info(&msg))
            }
            other => other,
        };
        self.state = TxArbiterState::Unknown {
            rig_id,
            reason: redacted_reason,
        };
        self.unknown_since = Some(tokio::time::Instant::now());
        self.generation += 1;
    }

    /// Oznacza stan jako nieznany (synonim [`Self::set_unknown`]).
    pub fn mark_unknown(&mut self, rig_id: Option<RigId>, reason: TxUnknownReason) {
        self.set_unknown(rig_id, reason);
    }

    /// Sprawdza, czy arbiter znajduje się w stanie Unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self.state, TxArbiterState::Unknown { .. })
    }

    /// Obsługuje awarię lub rozłączenie sesji wskazanego radia.
    ///
    /// Jeżeli uszkodzona sesja brała udział w procedurze TX (`Requesting`, `Owned`, `Releasing`),
    /// arbiter natychmiast przechodzi w stan `Unknown`, blokując pozostałe sesje do czasu resynchronizacji.
    pub fn handle_session_failure(&mut self, failed_rig_id: &RigId, reason_desc: &str) {
        let involved = match &self.state {
            TxArbiterState::Requesting { rig_id, .. }
            | TxArbiterState::Owned { rig_id, .. }
            | TxArbiterState::Releasing { rig_id, .. } => rig_id == failed_rig_id,
            TxArbiterState::Unknown {
                rig_id: Some(rig_id),
                ..
            } => rig_id == failed_rig_id,
            _ => false,
        };

        if involved {
            self.set_unknown(
                Some(failed_rig_id.clone()),
                TxUnknownReason::BackendError(reason_desc.to_string()),
            );
        }
    }

    /// Resynchronizuje arbiter bezpośrednio do stanu Idle.
    pub fn resync_to_idle(&mut self) {
        self.state = TxArbiterState::Idle;
        self.unknown_since = None;
        self.generation += 1;
    }

    /// Resynchronizuje arbiter ze stanu `Unknown` do `Idle` z weryfikacją świeżości pomiaru RX (ETAP 6).
    ///
    /// Przejście jest dozwolone wyłącznie wtedy, gdy:
    /// 1. Radio jest osiągalne i dostarczyło świeży, jednoznaczny stan RX (`rx_confirmed == true`),
    /// 2. Jeżeli stan `Unknown` wskazywał konkretne radio, `confirmed_rig_id` musi odpowiadać temu radiu,
    /// 3. Pomiar RX (`observation_time`) nastąpił PO wejściu w stan `Unknown` (brak stale cache),
    /// 4. Brak potwierdzonego stanu TX na którymkolwiek radiu.
    ///
    /// W przypadku niespełnienia kryteriów stan `Unknown` pozostaje nienaruszony.
    pub fn resync_with_fresh_audit(
        &mut self,
        confirmed_rig_id: &RigId,
        rx_confirmed: bool,
        observation_time: Option<tokio::time::Instant>,
    ) -> Result<(), TxArbiterError> {
        match &self.state {
            TxArbiterState::Idle => Ok(()),
            TxArbiterState::Unknown {
                rig_id: maybe_target,
                ..
            } => {
                if let Some(target) = maybe_target {
                    if target != confirmed_rig_id {
                        return Err(TxArbiterError::ResyncFailed(format!(
                            "resynchronizacja dotyczy radia '{confirmed_rig_id}', a stan Unknown zarejestrowano dla '{target}'"
                        )));
                    }
                }

                if !rx_confirmed {
                    return Err(TxArbiterError::ResyncFailed(
                        "brak jednoznacznego potwierdzenia stanu RX dla radia".to_string(),
                    ));
                }

                // Weryfikacja świeżości pomiaru: observation_time musi być PO wejściu w Unknown
                if let (Some(unknown_t), Some(obs_t)) = (self.unknown_since, observation_time) {
                    if obs_t < unknown_t {
                        return Err(TxArbiterError::ResyncFailed(
                            "pomiar RX wykonany przed wejściem w stan Unknown (nieświeży stan)"
                                .to_string(),
                        ));
                    }
                } else if self.unknown_since.is_some() && observation_time.is_none() {
                    return Err(TxArbiterError::ResyncFailed(
                        "brak znacznika czasu pomiaru RX po wejściu w stan Unknown".to_string(),
                    ));
                }

                self.state = TxArbiterState::Idle;
                self.unknown_since = None;
                self.generation += 1;
                Ok(())
            }
            other => Err(TxArbiterError::ResyncFailed(format!(
                "resynchronizacja ze stanu '{other}' nie jest dozwolona (dozwolona wyłącznie z Unknown)"
            ))),
        }
    }

    /// Resynchronizuje arbiter ze stanu `Unknown` do `Idle` z weryfikacją stanu RX.
    pub fn resync_with_audit(
        &mut self,
        confirmed_rig_id: &RigId,
        rx_confirmed: bool,
    ) -> Result<(), TxArbiterError> {
        self.resync_with_fresh_audit(
            confirmed_rig_id,
            rx_confirmed,
            Some(tokio::time::Instant::now()),
        )
    }

    /// Zwraca radio, które należy bezpiecznie rozłączyć (wyłączyć PTT) w trakcie procedury shutdown.
    ///
    /// Zwraca `Some(RigId)` wyłącznie wtedy, gdy arbiter jest w stanie `Owned`.
    /// W stanach `Idle` oraz `Unknown` procedura shutdown nie wysyła nadmiarowych komend PTT false.
    pub fn shutdown_release_target(&self) -> Option<RigId> {
        match &self.state {
            TxArbiterState::Owned { rig_id, .. } => Some(rig_id.clone()),
            _ => None,
        }
    }
}
