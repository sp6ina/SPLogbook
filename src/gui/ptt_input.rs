// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny, fail-closed dispatcher intencji PTT (F-17).
//!
//! Ten moduł jest jedynym miejscem, w którym warstwa GUI zamienia **intencję** użytkownika
//! (żądanie nadawania / żądanie odbioru) na wywołanie kanału nadawczego. Warstwa GUI nie
//! wykonuje żadnego `SetPtt(true)` bezpośrednio: wszystkie interakcyjne wejścia PTT (skrót
//! klawiaturowy, przycisk ekranowy, paleta poleceń) przechodzą przez [`dispatch`].
//!
//! # Model bezpieczeństwa
//!
//! * **Brak niejawnego przełącznika.** Istnieją dwie rozłączne intencje: [`PttIntent::RequestOn`]
//!   i [`PttIntent::RequestOff`]. Zdarzenie nigdy nie zmienia kierunku w zależności od stanu.
//! * **Brak pojedynczego klawisza.** Moduł nie zna klawiszy — bramką jest
//!   [`crate::gui::shortcuts::ShortcutGates`] wyliczana raz na klatkę.
//! * **Fail-closed.** Każda niejednoznaczność (release, repeat, brak fokusu okna, kompozycja IME,
//!   otwarty modal, fokus tekstowy, nieodczytany stan) kończy się kontrolowaną odmową i **nie**
//!   wywołuje kanału nadawczego.
//! * **Autoryzacja F-15 dla PTT ON.** Żądanie nadawania jest przekazywane dalej wyłącznie wtedy,
//!   gdy moduł F-15 potwierdzi upoważnienie (`HilAuthorizationOutcome::authorizes_hardware_ptt`).
//!   Wszystkie pozostałe wyniki (w tym `NotEvaluated` i `DryRunOnly`) są kontrolowaną odmową.
//!   Żądanie **wyłączenia** nadawania nigdy nie jest blokowane autoryzacją — blokada OFF mogłaby
//!   pozostawić nadajnik w stanie kluczowania.
//! * **Brak optymistycznego stanu GUI.** Wynik `RequestedOn`/`RequestedOff` oznacza wyłącznie
//!   „żądanie przekazane do `TxArbiter`”, nigdy „potwierdzony TX/RX”. Stan w GUI pochodzi
//!   z telemetrii i stanu arbitra, nie z tego modułu.
//! * **Determinizm.** Brak czasu ściennego, brak `sleep`, brak ponowień.

use crate::cat::hil::authorization::HilAuthorizationOutcome;
use crate::cat::tx_arbiter::TxArbiterState;
use crate::gui::shortcuts::ShortcutGates;
use eframe::egui;

/// Źródło intencji PTT. Wszystkie warianty są produkcyjne — nie istnieje wariant testowy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PttActionSource {
    /// Skrót klawiaturowy z co najmniej jednym modyfikatorem.
    KeyboardShortcut,
    /// Jawny przycisk ekranowy (wymaga świadomej interakcji).
    ScreenButton,
    /// Jawna komenda z palety poleceń.
    CommandPalette,
    /// Kontrolowane zwolnienie nadawania po utracie fokusu okna (wyłącznie intencja OFF).
    FocusLossRelease,
}

impl PttActionSource {
    /// Stabilna nazwa źródła (audyt, logi, testy).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeyboardShortcut => "keyboard_shortcut",
            Self::ScreenButton => "screen_button",
            Self::CommandPalette => "command_palette",
            Self::FocusLossRelease => "focus_loss_release",
        }
    }
}

/// Rozłączna intencja PTT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PttIntent {
    /// Żądanie załączenia nadawania (PTT ON) — wymaga autoryzacji F-15.
    RequestOn,
    /// Żądanie wyłączenia nadawania (PTT OFF) — nigdy nie wymaga autoryzacji.
    RequestOff,
}

impl PttIntent {
    /// Stabilna nazwa intencji (audyt, logi, testy).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestOn => "request_on",
            Self::RequestOff => "request_off",
        }
    }
}

/// Projekcja [`TxArbiterState`] na stan istotny dla decyzji PTT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PttTransmitState {
    /// Arbiter jest bezczynny — brak lokalnego właściciela nadajnika.
    Idle,
    /// Trwa żądanie załączenia nadawania, brak potwierdzenia.
    Requesting,
    /// Nadajnik jest potwierdzony jako lokalnie nadający.
    Owned,
    /// Trwa żądanie wyłączenia nadawania.
    Releasing,
    /// Stan niejednoznaczny — wymagana resynchronizacja.
    Unknown,
}

impl PttTransmitState {
    /// Stabilna nazwa stanu (audyt, logi, testy).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Requesting => "requesting",
            Self::Owned => "owned",
            Self::Releasing => "releasing",
            Self::Unknown => "unknown",
        }
    }
}

/// Projekcja stanu [`TxArbiterState`] na [`PttTransmitState`].
///
/// Funkcja nie odczytuje żadnego innego stanu — dzięki temu decyzja PTT jest deterministyczna.
#[must_use]
pub fn project_tx_state(state: &TxArbiterState) -> PttTransmitState {
    match state {
        TxArbiterState::Idle => PttTransmitState::Idle,
        TxArbiterState::Requesting { .. } => PttTransmitState::Requesting,
        TxArbiterState::Owned { .. } => PttTransmitState::Owned,
        TxArbiterState::Releasing { .. } => PttTransmitState::Releasing,
        TxArbiterState::Unknown { .. } => PttTransmitState::Unknown,
    }
}

/// Błąd kanału nadawczego, zgłoszony synchronicznie przez [`PttTransmitPort`].
///
/// Błąd jest jawnie raportowany jako porażka — nigdy jako sukces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttPortError {
    /// Brak aktywnego celu nadawczego (brak radia lub brak potwierdzenia właściciela TX).
    NoActiveRig,
    /// Kanał wykonawczy jest zamknięty (runtime CAT zatrzymany).
    RequestChannelClosed,
}

impl PttPortError {
    /// Stabilna nazwa błędu (audyt, logi, testy).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveRig => "no_active_rig",
            Self::RequestChannelClosed => "request_channel_closed",
        }
    }
}

/// Kanał wykonawczy PTT.
///
/// Produkcyjna implementacja żyje przy stanie CAT aplikacji i wywołuje wyłącznie
/// `CatRuntime::request_tx` / `CatRuntime::release_tx`, czyli pełną ścieżkę `TxArbiter`.
/// Implementacje testowe istnieją wyłącznie w zakresie testowym (`#[cfg(test)]` oraz `tests/`).
pub trait PttTransmitPort {
    /// Przekazuje żądanie załączenia nadawania (PTT ON) do arbitra.
    fn request_on(&mut self) -> Result<(), PttPortError>;

    /// Przekazuje żądanie wyłączenia nadawania (PTT OFF) do arbitra.
    fn request_off(&mut self) -> Result<(), PttPortError>;
}

/// Jawny wynik decyzji PTT.
///
/// Wszystkie warianty poza [`Self::RequestedOn`] gwarantują brak wywołania kanału nadawczego
/// w kierunku ON. [`Self::RequestedOn`]/[`Self::RequestedOff`] oznaczają wyłącznie
/// „żądanie przekazane do `TxArbiter`” — nie potwierdzony stan TX/RX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttDispatchOutcome {
    /// Zdarzenie `release` — nigdy nie tworzy intencji PTT.
    IgnoredRelease,
    /// Powtórzenie klawisza (autorepeat) — zero nowych intencji.
    IgnoredRepeat,
    /// Aktywne pole tekstowe lub framework żąda wejścia klawiatury.
    IgnoredTextEditing,
    /// Aktywna kompozycja IME.
    IgnoredImeActive,
    /// Okno aplikacji nie ma fokusu systemowego.
    IgnoredWindowUnfocused,
    /// Otwarte okno modalne.
    IgnoredModalOpen,
    /// Naciśnięcie nie spełnia dokładnego kontraktu modyfikatorów albo kombinacja
    /// źródło/intencja jest niedozwolona.
    ShortcutMismatch,
    /// Nadajnik już nadaje — żądanie ON jest ignorowane (brak podwójnego ON).
    AlreadyTransmitting,
    /// Nadajnik nie nadaje — żądanie OFF jest ignorowane (brak zbędnego release).
    NotTransmitting,
    /// Arbiter jest zajęty inną operacją.
    RefusedBusy,
    /// Stan arbitra jest niejednoznaczny — wymagana resynchronizacja.
    RefusedUnknownTxState,
    /// Brak autoryzacji F-15 dla żądania nadawania.
    AuthorizationDenied(HilAuthorizationOutcome),
    /// Żądanie nadawania przekazane do arbitra (nie: potwierdzony TX).
    RequestedOn,
    /// Żądanie wyłączenia nadawania przekazane do arbitra (nie: potwierdzony RX).
    RequestedOff,
    /// Kanał nadawczy zwrócił błąd — jawna porażka, nigdy sukces.
    Failed(PttPortError),
}

impl PttDispatchOutcome {
    /// Czy żądanie zostało przekazane do arbitra.
    #[must_use]
    pub const fn is_request(self) -> bool {
        matches!(self, Self::RequestedOn | Self::RequestedOff)
    }

    /// Czy wynik jest kontrolowaną odmową (brak żądania).
    #[must_use]
    pub const fn is_denial(self) -> bool {
        !self.is_request()
    }

    /// Stabilny kod wyniku (audyt, logi, interfejs, testy).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IgnoredRelease => "ignored_release",
            Self::IgnoredRepeat => "ignored_repeat",
            Self::IgnoredTextEditing => "ignored_text_editing",
            Self::IgnoredImeActive => "ignored_ime_active",
            Self::IgnoredWindowUnfocused => "ignored_window_unfocused",
            Self::IgnoredModalOpen => "ignored_modal_open",
            Self::ShortcutMismatch => "shortcut_mismatch",
            Self::AlreadyTransmitting => "already_transmitting",
            Self::NotTransmitting => "not_transmitting",
            Self::RefusedBusy => "refused_busy",
            Self::RefusedUnknownTxState => "refused_unknown_tx_state",
            Self::AuthorizationDenied(_) => "authorization_denied",
            Self::RequestedOn => "requested_on",
            Self::RequestedOff => "requested_off",
            Self::Failed(_) => "failed",
        }
    }
}

/// Migawka stanu wejścia PTT w jednej klatce.
///
/// Wszystkie pola są wyliczane raz na klatkę z produkcyjnego wejścia GUI i przekazywane
/// niezmienione do [`dispatch`], dzięki czemu decyzja jest deterministyczna i testowalna
/// bez okna systemowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PttInputSnapshot {
    /// Źródło intencji (skrót / przycisk / paleta).
    pub source: PttActionSource,
    /// Rozłączna intencja.
    pub intent: PttIntent,
    /// Czy zdarzenie jest naciśnięciem (`false` = zwolnienie).
    pub pressed: bool,
    /// Czy zdarzenie jest powtórzeniem klawisza.
    pub repeat: bool,
    /// Czy naciśnięcie spełnia dokładny kontrakt modyfikatorów (dla przycisków: zawsze `true`).
    pub modifiers_ok: bool,
    /// Bramki wejściowe tej klatki.
    pub gates: ShortcutGates,
}

impl PttInputSnapshot {
    /// Migawka dla jawnego przycisku ekranowego lub komendy palety.
    ///
    /// Interakcja wskaźnikiem nie jest zdarzeniem klawiatury, więc nie ma modyfikatora,
    /// powtórzenia ani zwolnienia klawisza — pozostają wyłącznie bramki kontekstowe.
    #[must_use]
    pub fn pointer(source: PttActionSource, intent: PttIntent, gates: ShortcutGates) -> Self {
        Self {
            source,
            intent,
            pressed: true,
            repeat: false,
            modifiers_ok: true,
            gates,
        }
    }
}

/// Deterministyczny detektor aktywnej kompozycji IME.
///
/// egui 0.36.2 nie przechowuje stanu kompozycji, więc jest on wyprowadzany z sekwencji
/// [`egui::Event::Ime`]. Detektor jest **fail-closed**: każda klatka, w której wystąpiło
/// jakiekolwiek zdarzenie IME, zamyka bramkę PTT, a niepusta kompozycja (`Preedit` z tekstem)
/// zamyka ją także w kolejnych klatkach, aż do jawnego zakończenia.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PttImeTracker {
    composing: bool,
}

impl PttImeTracker {
    /// Tworzy detektor w stanie spoczynku (brak kompozycji).
    #[must_use]
    pub const fn new() -> Self {
        Self { composing: false }
    }

    /// Czy kompozycja IME jest aktualnie aktywna.
    #[must_use]
    pub const fn is_composing(&self) -> bool {
        self.composing
    }

    /// Zeruje stan kompozycji (utrata fokusu okna, zamknięcie aplikacji).
    pub fn reset(&mut self) {
        self.composing = false;
    }

    /// Przetwarza zdarzenia klatki i zwraca stan bramki IME dla tej klatki.
    ///
    /// Zwracana wartość jest `true`, gdy kompozycja jest aktywna **lub** w klatce wystąpiło
    /// jakiekolwiek zdarzenie IME. Dzięki temu PTT nie może wykonać się „po commitcie”
    /// w tej samej klatce.
    pub fn observe_frame(&mut self, events: &[egui::Event]) -> bool {
        let mut saw_ime = false;
        for event in events {
            let egui::Event::Ime(ime_event) = event else {
                continue;
            };
            saw_ime = true;
            match ime_event {
                egui::ImeEvent::Preedit { text, .. } => {
                    self.composing = !text.is_empty();
                }
                egui::ImeEvent::DeleteSurrounding { .. } => {
                    // Usuwanie otoczenia kursora występuje wyłącznie w kontekście kompozycji.
                    self.composing = true;
                }
                #[allow(deprecated)]
                egui::ImeEvent::Enabled => {
                    // Wariant przestarzały i nieemitowany przez egui-winit; fail-closed.
                    self.composing = true;
                }
                // `Commit` oraz przestarzały `Disabled` kończą kompozycję.
                #[allow(deprecated)]
                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => {
                    self.composing = false;
                }
            }
        }
        saw_ime || self.composing
    }
}

/// Strażnik kontrolowanego zwolnienia PTT po utracie fokusu okna (F-17).
///
/// Emituje **dokładnie jedną** intencję [`PttIntent::RequestOff`] na przejście
/// „okno miało fokus → okno utraciło fokus”, i tylko wtedy, gdy arbiter potwierdza lokalne
/// nadawanie. Brak ponowień, brak czasu ściennego, brak zależności od zdarzenia key-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PttFocusReleaseGuard {
    was_focused: bool,
    pending: bool,
}

impl Default for PttFocusReleaseGuard {
    fn default() -> Self {
        Self {
            was_focused: true,
            pending: false,
        }
    }
}

impl PttFocusReleaseGuard {
    /// Tworzy strażnika zakładającego, że okno ma fokus.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            was_focused: true,
            pending: false,
        }
    }

    /// Czy oczekuje na wykonanie kontrolowanego zwolnienia.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.pending
    }

    /// Obserwuje stan fokusu okna i stan nadajnika; zwraca najwyżej jedną intencję OFF.
    pub fn observe(
        &mut self,
        window_focused: bool,
        tx_state: PttTransmitState,
    ) -> Option<PttIntent> {
        let lost_focus = self.was_focused && !window_focused;
        self.was_focused = window_focused;

        if lost_focus && tx_state == PttTransmitState::Owned {
            self.pending = true;
        }

        if !window_focused && self.pending {
            // Dokładnie jedna próba na jedno przejście fokusu — bez ponowień.
            self.pending = false;
            return Some(PttIntent::RequestOff);
        }

        if window_focused {
            self.pending = false;
        }
        None
    }
}

/// Centralna decyzja PTT: zamienia migawkę wejścia na jawny wynik i — wyłącznie dla prawidłowego
/// żądania — wywołuje kanał nadawczy.
///
/// Kolejność bramek jest deterministyczna i fail-closed: każde odrzucenie wejścia następuje
/// **przed** odczytem stanu arbitra i przed wywołaniem [`PttTransmitPort`].
///
/// Wyjątkiem jest [`PttActionSource::FocusLossRelease`]: to wewnętrzna akcja bezpieczeństwa,
/// a nie wejście użytkownika, więc bramki wejściowe jej nie dotyczą — ale wolno jej wyłącznie
/// żądać [`PttIntent::RequestOff`].
pub fn dispatch(
    snapshot: &PttInputSnapshot,
    tx_state: PttTransmitState,
    authorization: HilAuthorizationOutcome,
    port: &mut dyn PttTransmitPort,
) -> PttDispatchOutcome {
    if snapshot.source == PttActionSource::FocusLossRelease {
        // Kontrolowane zwolnienie po utracie fokusu nigdy nie może żądać nadawania.
        if snapshot.intent != PttIntent::RequestOff {
            return PttDispatchOutcome::ShortcutMismatch;
        }
    } else {
        // 1. Zwolnienie klawisza nigdy nie tworzy intencji.
        if !snapshot.pressed {
            return PttDispatchOutcome::IgnoredRelease;
        }

        // 2. Powtórzenie klawisza nigdy nie tworzy nowej intencji.
        if snapshot.repeat {
            return PttDispatchOutcome::IgnoredRepeat;
        }

        // 3. Dokładny kontrakt modyfikatorów (dla skrótu klawiaturowego).
        if !snapshot.modifiers_ok {
            return PttDispatchOutcome::ShortcutMismatch;
        }

        // 4. Fokus okna.
        if !snapshot.gates.window_focused {
            return PttDispatchOutcome::IgnoredWindowUnfocused;
        }

        // 5. Kompozycja IME.
        if snapshot.gates.ime_composing {
            return PttDispatchOutcome::IgnoredImeActive;
        }

        // 6. Okno modalne.
        if snapshot.gates.modal_open {
            return PttDispatchOutcome::IgnoredModalOpen;
        }

        // 7. Fokus edycji tekstu.
        if snapshot.gates.text_editing {
            return PttDispatchOutcome::IgnoredTextEditing;
        }
    }

    match snapshot.intent {
        PttIntent::RequestOff => match tx_state {
            PttTransmitState::Owned => match port.request_off() {
                Ok(()) => PttDispatchOutcome::RequestedOff,
                Err(error) => PttDispatchOutcome::Failed(error),
            },
            // Brak potwierdzonego nadawania — nie ma czego zwalniać.
            PttTransmitState::Idle | PttTransmitState::Requesting => {
                PttDispatchOutcome::NotTransmitting
            }
            // Zwolnienie już trwa — nie duplikujemy release.
            PttTransmitState::Releasing => PttDispatchOutcome::RefusedBusy,
            PttTransmitState::Unknown => PttDispatchOutcome::RefusedUnknownTxState,
        },
        PttIntent::RequestOn => match tx_state {
            // Brak podwójnego ON.
            PttTransmitState::Owned => PttDispatchOutcome::AlreadyTransmitting,
            PttTransmitState::Requesting | PttTransmitState::Releasing => {
                PttDispatchOutcome::RefusedBusy
            }
            PttTransmitState::Unknown => PttDispatchOutcome::RefusedUnknownTxState,
            PttTransmitState::Idle => {
                // 8. F-15: jedyna bramka, która może upoważnić logiczne PTT ON.
                // Pytamy moduł F-15 o upoważnienie — GUI nie nazywa wariantu „authorized”
                // i nie może go skonstruować.
                if authorization.authorizes_hardware_ptt() {
                    match port.request_on() {
                        Ok(()) => PttDispatchOutcome::RequestedOn,
                        Err(error) => PttDispatchOutcome::Failed(error),
                    }
                } else {
                    PttDispatchOutcome::AuthorizationDenied(authorization)
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejestrujący kanał nadawczy — wyłącznie w zakresie testowym.
    #[derive(Debug, Default)]
    struct RecordingPort {
        on_calls: usize,
        off_calls: usize,
        fail_on: Option<PttPortError>,
        fail_off: Option<PttPortError>,
    }

    impl PttTransmitPort for RecordingPort {
        fn request_on(&mut self) -> Result<(), PttPortError> {
            self.on_calls += 1;
            self.fail_on.map_or(Ok(()), Err)
        }

        fn request_off(&mut self) -> Result<(), PttPortError> {
            self.off_calls += 1;
            self.fail_off.map_or(Ok(()), Err)
        }
    }

    fn open() -> ShortcutGates {
        ShortcutGates {
            text_editing: false,
            ime_composing: false,
            window_focused: true,
            modal_open: false,
        }
    }

    fn shortcut(intent: PttIntent, gates: ShortcutGates) -> PttInputSnapshot {
        PttInputSnapshot {
            source: PttActionSource::KeyboardShortcut,
            intent,
            pressed: true,
            repeat: false,
            modifiers_ok: true,
            gates,
        }
    }

    const NOT_EVALUATED: HilAuthorizationOutcome = HilAuthorizationOutcome::NotEvaluated;
    const DRY_RUN_ONLY: HilAuthorizationOutcome = HilAuthorizationOutcome::DryRunOnly;

    // Pozytywna kontrola PTT ON z upoważnieniem F-15 żyje wyłącznie w `tests/p1_f17_ptt_gating.rs`:
    // warstwa GUI nie może nazwać wariantu „authorized”, co jest pilnowane przez `security_guard`.

    #[test]
    fn f17_on_without_authorization_never_reaches_the_port() {
        for authorization in [
            HilAuthorizationOutcome::NotEvaluated,
            HilAuthorizationOutcome::DryRunOnly,
            HilAuthorizationOutcome::HardwareExecutionDenied,
            HilAuthorizationOutcome::EvidenceMissing,
            HilAuthorizationOutcome::EvidenceExpired,
            HilAuthorizationOutcome::EvidenceInvalid,
            HilAuthorizationOutcome::EvidenceScopeMismatch,
            HilAuthorizationOutcome::EvidenceReplayed,
            HilAuthorizationOutcome::IndependentApprovalMissing,
            HilAuthorizationOutcome::OperatorAttestationMissing,
            HilAuthorizationOutcome::ConfigurationInvalid,
        ] {
            let mut port = RecordingPort::default();
            let outcome = dispatch(
                &shortcut(PttIntent::RequestOn, open()),
                PttTransmitState::Idle,
                authorization,
                &mut port,
            );
            assert_eq!(
                outcome,
                PttDispatchOutcome::AuthorizationDenied(authorization)
            );
            assert_eq!(port.on_calls, 0, "{authorization} must not key PTT");
            assert_eq!(port.off_calls, 0);
            assert!(!outcome.is_request());
        }
    }

    #[test]
    fn f17_off_never_requires_authorization() {
        let mut port = RecordingPort::default();
        let outcome = dispatch(
            &shortcut(PttIntent::RequestOff, open()),
            PttTransmitState::Owned,
            NOT_EVALUATED,
            &mut port,
        );
        assert_eq!(outcome, PttDispatchOutcome::RequestedOff);
        assert_eq!(port.off_calls, 1);
        assert_eq!(port.on_calls, 0);
    }

    #[test]
    fn f17_every_closed_gate_denies_on_and_off() {
        let mut port = RecordingPort::default();
        let closed = [
            (
                ShortcutGates {
                    text_editing: true,
                    ..open()
                },
                PttDispatchOutcome::IgnoredTextEditing,
            ),
            (
                ShortcutGates {
                    ime_composing: true,
                    ..open()
                },
                PttDispatchOutcome::IgnoredImeActive,
            ),
            (
                ShortcutGates {
                    window_focused: false,
                    ..open()
                },
                PttDispatchOutcome::IgnoredWindowUnfocused,
            ),
            (
                ShortcutGates {
                    modal_open: true,
                    ..open()
                },
                PttDispatchOutcome::IgnoredModalOpen,
            ),
            // `fail_closed` zamyka wszystkie bramki; jako pierwsza rozstrzyga bramka fokusu okna.
            (
                ShortcutGates::fail_closed(),
                PttDispatchOutcome::IgnoredWindowUnfocused,
            ),
        ];
        for (gates, expected) in closed {
            for (intent, tx_state) in [
                (PttIntent::RequestOn, PttTransmitState::Idle),
                (PttIntent::RequestOff, PttTransmitState::Owned),
            ] {
                let outcome =
                    dispatch(&shortcut(intent, gates), tx_state, NOT_EVALUATED, &mut port);
                assert_eq!(outcome, expected, "{intent:?} with {gates:?}");
            }
        }
        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }

    #[test]
    fn f17_release_and_repeat_are_rejected_before_everything_else() {
        let mut port = RecordingPort::default();
        let mut snapshot = shortcut(PttIntent::RequestOn, open());
        snapshot.pressed = false;
        assert_eq!(
            dispatch(&snapshot, PttTransmitState::Idle, NOT_EVALUATED, &mut port),
            PttDispatchOutcome::IgnoredRelease
        );

        let mut snapshot = shortcut(PttIntent::RequestOn, open());
        snapshot.repeat = true;
        assert_eq!(
            dispatch(&snapshot, PttTransmitState::Idle, NOT_EVALUATED, &mut port),
            PttDispatchOutcome::IgnoredRepeat
        );

        let mut snapshot = shortcut(PttIntent::RequestOn, open());
        snapshot.modifiers_ok = false;
        assert_eq!(
            dispatch(&snapshot, PttTransmitState::Idle, NOT_EVALUATED, &mut port),
            PttDispatchOutcome::ShortcutMismatch
        );

        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }

    #[test]
    fn f17_tx_state_matrix_is_explicit() {
        let mut port = RecordingPort::default();
        let cases = [
            (
                PttIntent::RequestOn,
                PttTransmitState::Owned,
                PttDispatchOutcome::AlreadyTransmitting,
            ),
            (
                PttIntent::RequestOn,
                PttTransmitState::Requesting,
                PttDispatchOutcome::RefusedBusy,
            ),
            (
                PttIntent::RequestOn,
                PttTransmitState::Releasing,
                PttDispatchOutcome::RefusedBusy,
            ),
            (
                PttIntent::RequestOn,
                PttTransmitState::Unknown,
                PttDispatchOutcome::RefusedUnknownTxState,
            ),
            (
                PttIntent::RequestOff,
                PttTransmitState::Idle,
                PttDispatchOutcome::NotTransmitting,
            ),
            (
                PttIntent::RequestOff,
                PttTransmitState::Requesting,
                PttDispatchOutcome::NotTransmitting,
            ),
            (
                PttIntent::RequestOff,
                PttTransmitState::Releasing,
                PttDispatchOutcome::RefusedBusy,
            ),
            (
                PttIntent::RequestOff,
                PttTransmitState::Unknown,
                PttDispatchOutcome::RefusedUnknownTxState,
            ),
        ];
        for (intent, tx_state, expected) in cases {
            let outcome = dispatch(
                &shortcut(intent, open()),
                tx_state,
                NOT_EVALUATED,
                &mut port,
            );
            assert_eq!(outcome, expected, "{intent:?} / {tx_state:?}");
            assert!(!outcome.is_request());
        }
        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }

    #[test]
    fn f17_port_failure_is_reported_as_failure_never_as_success() {
        // Kanał OFF nie wymaga autoryzacji, więc błąd kanału jest tu w pełni obserwowalny.
        // Błąd kanału ON (po upoważnieniu) jest weryfikowany w `tests/p1_f17_ptt_gating.rs`.
        let mut off_port = RecordingPort {
            fail_off: Some(PttPortError::RequestChannelClosed),
            ..RecordingPort::default()
        };
        let outcome = dispatch(
            &shortcut(PttIntent::RequestOff, open()),
            PttTransmitState::Owned,
            NOT_EVALUATED,
            &mut off_port,
        );
        assert_eq!(
            outcome,
            PttDispatchOutcome::Failed(PttPortError::RequestChannelClosed)
        );
        assert!(outcome.is_denial(), "an OFF error must never be a success");
        assert_eq!(outcome.code(), "failed");
        assert_eq!(off_port.off_calls, 1);
        assert_eq!(off_port.on_calls, 0);
    }

    #[test]
    fn f17_pointer_snapshots_have_no_keyboard_artefacts() {
        let snapshot =
            PttInputSnapshot::pointer(PttActionSource::ScreenButton, PttIntent::RequestOn, open());
        assert!(snapshot.pressed);
        assert!(!snapshot.repeat);
        assert!(snapshot.modifiers_ok);
        assert_eq!(snapshot.source, PttActionSource::ScreenButton);
    }

    #[test]
    fn f17_ime_tracker_starts_closed_on_preedit_and_reopens_on_commit() {
        let mut tracker = PttImeTracker::new();
        assert!(!tracker.observe_frame(&[]));

        let preedit = egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "に".to_string(),
            active_range_chars: None,
        });
        assert!(tracker.observe_frame(std::slice::from_ref(&preedit)));
        assert!(tracker.is_composing());
        // Kolejna klatka bez zdarzeń pozostaje zamknięta (kompozycja trwa).
        assert!(tracker.observe_frame(&[]));

        let commit = egui::Event::Ime(egui::ImeEvent::Commit("に".to_string()));
        assert!(tracker.observe_frame(std::slice::from_ref(&commit)));
        assert!(!tracker.is_composing());
        // Klatka po commitcie jest już otwarta.
        assert!(!tracker.observe_frame(&[]));
    }

    #[test]
    fn f17_ime_tracker_treats_empty_preedit_as_dismissal() {
        let mut tracker = PttImeTracker::new();
        let preedit = egui::Event::Ime(egui::ImeEvent::Preedit {
            text: String::new(),
            active_range_chars: None,
        });
        // Klatka z jakimkolwiek zdarzeniem IME jest zamknięta, ale stan nie jest „composing”.
        assert!(tracker.observe_frame(std::slice::from_ref(&preedit)));
        assert!(!tracker.is_composing());
        assert!(!tracker.observe_frame(&[]));
    }

    #[test]
    fn f17_ime_tracker_fails_closed_on_delete_surrounding() {
        let mut tracker = PttImeTracker::new();
        let event = egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
            before_chars: 1,
            after_chars: 0,
        });
        assert!(tracker.observe_frame(std::slice::from_ref(&event)));
        assert!(tracker.is_composing());
    }

    #[test]
    fn f17_focus_release_guard_emits_exactly_one_off_per_transition() {
        let mut guard = PttFocusReleaseGuard::new();
        // Fokus jest, nadawanie potwierdzone.
        assert_eq!(guard.observe(true, PttTransmitState::Owned), None);
        // Utrata fokusu przy potwierdzonym nadawaniu → dokładnie jedna intencja OFF.
        assert_eq!(
            guard.observe(false, PttTransmitState::Owned),
            Some(PttIntent::RequestOff)
        );
        // Kolejne klatki bez fokusu nie duplikują release.
        assert_eq!(guard.observe(false, PttTransmitState::Owned), None);
        assert_eq!(guard.observe(false, PttTransmitState::Owned), None);
        // Powrót fokusu zamyka cykl.
        assert_eq!(guard.observe(true, PttTransmitState::Owned), None);
    }

    #[test]
    fn f17_focus_release_guard_is_silent_without_confirmed_transmit() {
        for tx_state in [
            PttTransmitState::Idle,
            PttTransmitState::Requesting,
            PttTransmitState::Releasing,
            PttTransmitState::Unknown,
        ] {
            let mut guard = PttFocusReleaseGuard::new();
            assert_eq!(guard.observe(true, tx_state), None);
            assert_eq!(guard.observe(false, tx_state), None, "{tx_state:?}");
            assert!(!guard.is_pending());
        }
    }

    #[test]
    fn f17_focus_release_reaches_the_port_despite_closed_input_gates() {
        // Kontrolowane zwolnienie po utracie fokusu nie jest wejściem użytkownika:
        // bramki wejściowe (w tym fokus okna i fokus tekstowy) go nie blokują.
        let mut port = RecordingPort::default();
        let snapshot = PttInputSnapshot {
            source: PttActionSource::FocusLossRelease,
            intent: PttIntent::RequestOff,
            pressed: true,
            repeat: false,
            modifiers_ok: true,
            gates: ShortcutGates::fail_closed(),
        };
        let outcome = dispatch(&snapshot, PttTransmitState::Owned, NOT_EVALUATED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::RequestedOff);
        assert_eq!(port.off_calls, 1);
        assert_eq!(port.on_calls, 0);
    }

    #[test]
    fn f17_focus_release_can_never_request_transmit() {
        let mut port = RecordingPort::default();
        for authorization in [NOT_EVALUATED, DRY_RUN_ONLY] {
            let snapshot = PttInputSnapshot {
                source: PttActionSource::FocusLossRelease,
                intent: PttIntent::RequestOn,
                pressed: true,
                repeat: false,
                modifiers_ok: true,
                gates: open(),
            };
            let outcome = dispatch(&snapshot, PttTransmitState::Idle, authorization, &mut port);
            assert_eq!(outcome, PttDispatchOutcome::ShortcutMismatch);
        }
        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }

    #[test]
    fn f17_project_tx_state_covers_every_arbiter_state() {
        use crate::cat::routing::{RigCommandId, RigId};
        let rig = RigId::hub();
        let command = RigCommandId::new(1);
        assert_eq!(
            project_tx_state(&TxArbiterState::Idle),
            PttTransmitState::Idle
        );
        assert_eq!(
            project_tx_state(&TxArbiterState::Requesting {
                rig_id: rig.clone(),
                command_id: command,
            }),
            PttTransmitState::Requesting
        );
        assert_eq!(
            project_tx_state(&TxArbiterState::Owned {
                rig_id: rig.clone(),
                command_id: command,
            }),
            PttTransmitState::Owned
        );
        assert_eq!(
            project_tx_state(&TxArbiterState::Releasing {
                rig_id: rig,
                command_id: command,
            }),
            PttTransmitState::Releasing
        );
        assert_eq!(
            project_tx_state(&TxArbiterState::Unknown {
                rig_id: None,
                reason: crate::cat::tx_arbiter::TxUnknownReason::Timeout,
            }),
            PttTransmitState::Unknown
        );
    }

    #[test]
    fn f17_outcome_codes_are_stable_and_unique() {
        let codes = [
            PttDispatchOutcome::IgnoredRelease.code(),
            PttDispatchOutcome::IgnoredRepeat.code(),
            PttDispatchOutcome::IgnoredTextEditing.code(),
            PttDispatchOutcome::IgnoredImeActive.code(),
            PttDispatchOutcome::IgnoredWindowUnfocused.code(),
            PttDispatchOutcome::IgnoredModalOpen.code(),
            PttDispatchOutcome::ShortcutMismatch.code(),
            PttDispatchOutcome::AlreadyTransmitting.code(),
            PttDispatchOutcome::NotTransmitting.code(),
            PttDispatchOutcome::RefusedBusy.code(),
            PttDispatchOutcome::RefusedUnknownTxState.code(),
            PttDispatchOutcome::AuthorizationDenied(NOT_EVALUATED).code(),
            PttDispatchOutcome::RequestedOn.code(),
            PttDispatchOutcome::RequestedOff.code(),
            PttDispatchOutcome::Failed(PttPortError::NoActiveRig).code(),
        ];
        let unique: std::collections::HashSet<&str> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len());
    }
}
