// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Typy identyfikacyjne, adresowania i routingu komend oraz zdarzeń CAT (NATIVE-6A).
//!
//! Zapewnia silnie typowane identyfikatory radiostacji ([`RigId`]), odbiorników ([`ReceiverId`]),
//! celów operacji ([`RigTarget`], [`ActiveRxTarget`], [`ActiveTxTarget`]), monotoniczne
//! identyfikatory poleceń ([`RigCommandId`]) oraz struktury przekazywania komend i zdarzeń
//! ([`RoutedRigCommand`], [`RoutedRigEvent`]).

use std::fmt;
use std::num::NonZeroU64;

use crate::cat::native::NativeRigEvent;
use crate::cat::rig::RigCommand;
use crate::cat::tx_arbiter::TxArbiterState;

/// Silnie typowany, unikalny identyfikator instancji radiostacji w systemie CAT.
///
/// Identyfikator jest niezależny od kolejności rejestracji sesji, indeksów kolekcji
/// oraz nazwy wyświetlanej użytkownikowi. Nie może być pusty ani składać się wyłącznie
/// ze znaków białych.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RigId(String);

impl RigId {
    /// Tworzy nowy [`RigId`] po uprzedniej walidacji.
    ///
    /// # Błędy
    /// Zwraca [`RigIdError::Empty`], jeżeli ciąg jest pusty, lub
    /// [`RigIdError::WhitespaceOnly`], jeżeli składa się wyłącznie z białych znaków.
    pub fn new(id: impl Into<String>) -> Result<Self, RigIdError> {
        let raw = id.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            if raw.is_empty() {
                return Err(RigIdError::Empty);
            }
            return Err(RigIdError::WhitespaceOnly);
        }
        if raw.chars().any(|c| c.is_control()) {
            return Err(RigIdError::ContainsControlCharacters);
        }
        // Ochrona przed przypadkowym umieszczeniem haseł lub tokenów w ID
        if raw.contains("password=") || raw.contains("token=") || raw.contains("secret=") {
            return Err(RigIdError::ContainsSensitiveInfo);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Zwraca reprezentację tekstową identyfikatora.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Bezpieczny identyfikator logiczny centralnego dyspozytora (huba).
    pub fn hub() -> Self {
        Self("hub".to_string())
    }

    /// Bezpieczny identyfikator logiczny braku przypisania.
    pub fn none() -> Self {
        Self("none".to_string())
    }
}

impl fmt::Display for RigId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for RigId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RigId({:?})", self.0)
    }
}

/// Błędy walidacji identyfikatora [`RigId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigIdError {
    /// Identyfikator jest pusty.
    Empty,
    /// Identyfikator zawiera wyłącznie białe znaki.
    WhitespaceOnly,
    /// Identyfikator zawiera niedozwolone znaki kontrolne.
    ContainsControlCharacters,
    /// Identyfikator zawiera niedozwolone wzorce poświadczeń/tajemnic.
    ContainsSensitiveInfo,
}

impl fmt::Display for RigIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "RigId nie może być pusty"),
            Self::WhitespaceOnly => {
                write!(f, "RigId nie może składać się wyłącznie z białych znaków")
            }
            Self::ContainsControlCharacters => {
                write!(f, "RigId nie może zawierać znaków kontrolnych")
            }
            Self::ContainsSensitiveInfo => {
                write!(f, "RigId nie może zawierać tokenów ani poświadczeń")
            }
        }
    }
}

impl std::error::Error for RigIdError {}

/// Silnie typowany identyfikator odbiornika / VFO / slice.
///
/// Gwarantuje brak niepożądanego aliasowania pomiędzy tradycyjnym modelem VFO (Main, Sub, VfoA, VfoB)
/// a wieloodbiornikową architekturą SDR Multi-Slice (np. FlexRadio SmartSDR `Slice(u32)`).
/// Wartość `Slice(0)` nie jest równoważna ani `VfoA`, ani `Main`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReceiverId {
    /// Główny odbiornik klasycznego radia.
    Main,
    /// Odbiornik pomocniczy (Sub-receiver) klasycznego radia (np. dual watch, KRX3, FT-991A/FTDX101).
    Sub,
    /// Dedykowany VFO A w protokołach bazujących na VFO (np. Yaesu, Kenwood).
    VfoA,
    /// Dedykowany VFO B w protokołach bazujących na VFO.
    VfoB,
    /// Niezależny plaster odbiorczy SDR (FlexRadio SmartSDR Slice ID `0..=7`).
    Slice(u32),
    /// Odbiornik o niestandardowej tożsamości zdefiniowanej przez backend.
    BackendDefined(String),
}

impl ReceiverId {
    /// Tworzy niestandardowy identyfikator odbiornika specyficzny dla backendu.
    ///
    /// # Błędy
    /// Zwraca [`ReceiverIdError::EmptyBackendDefined`], jeśli ciąg jest pusty lub zawiera tylko spacje.
    pub fn backend_defined(id: impl Into<String>) -> Result<Self, ReceiverIdError> {
        let raw = id.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ReceiverIdError::EmptyBackendDefined);
        }
        Ok(Self::BackendDefined(trimmed.to_string()))
    }

    /// Tworzy identyfikator VFO-A.
    pub const fn vfo_a() -> Self {
        Self::VfoA
    }

    /// Tworzy identyfikator VFO-B.
    pub const fn vfo_b() -> Self {
        Self::VfoB
    }

    /// Tworzy identyfikator plastra SDR.
    pub const fn slice(id: u32) -> Self {
        Self::Slice(id)
    }

    /// Sprawdza, czy odbiornik jest klasycznym VFO (VFO-A lub VFO-B).
    pub fn is_vfo(&self) -> bool {
        matches!(self, Self::VfoA | Self::VfoB)
    }

    /// Zwraca indeks plastra SDR, jeśli odbiornik reprezentuje `Slice(u32)`.
    pub fn slice_index(&self) -> Option<u32> {
        self.as_slice_id()
    }

    /// Sprawdza, czy odbiornik jest plastrem SDR Multi-Slice.
    pub fn is_slice(&self) -> bool {
        matches!(self, Self::Slice(_))
    }

    /// Pobiera numer plastra SDR, jeśli odbiornik reprezentuje `Slice(u32)`.
    pub fn as_slice_id(&self) -> Option<u32> {
        match self {
            Self::Slice(id) => Some(*id),
            _ => None,
        }
    }
}

impl fmt::Display for ReceiverId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Main => write!(f, "main"),
            Self::Sub => write!(f, "sub"),
            Self::VfoA => write!(f, "vfo_a"),
            Self::VfoB => write!(f, "vfo_b"),
            Self::Slice(id) => write!(f, "slice_{id}"),
            Self::BackendDefined(s) => write!(f, "backend:{s}"),
        }
    }
}

/// Błędy walidacji identyfikatora [`ReceiverId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverIdError {
    /// Wariant `BackendDefined` nie może być pusty.
    EmptyBackendDefined,
}

impl fmt::Display for ReceiverIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBackendDefined => {
                write!(
                    f,
                    "identyfikator ReceiverId::BackendDefined nie może być pusty"
                )
            }
        }
    }
}

impl std::error::Error for ReceiverIdError {}

/// Cel polecenia lub fokus telemetrii wskazujący radio oraz opcjonalny odbiornik.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RigTarget {
    /// Identyfikator docelowego radiotelefonu.
    pub rig_id: RigId,
    /// Opcjonalny odbiornik w obrębie radia (np. VFO B lub Slice 1).
    pub receiver_id: Option<ReceiverId>,
}

impl RigTarget {
    /// Tworzy nowy cel wskazujący radio i opcjonalny odbiornik.
    pub fn new(rig_id: RigId, receiver_id: Option<ReceiverId>) -> Self {
        Self {
            rig_id,
            receiver_id,
        }
    }

    /// Tworzy cel wskazujący wyłącznie radio (polecenia globalne transceivera).
    pub fn rig_only(rig_id: RigId) -> Self {
        Self {
            rig_id,
            receiver_id: None,
        }
    }

    /// Tworzy cel wskazujący całe radio (synonim [`RigTarget::rig_only`]).
    pub fn entire_rig(rig_id: RigId) -> Self {
        Self::rig_only(rig_id)
    }

    /// Tworzy cel wskazujący konkretny odbiornik w radiu (synonim [`RigTarget::with_receiver`]).
    pub fn receiver(rig_id: RigId, receiver_id: ReceiverId) -> Self {
        Self::with_receiver(rig_id, receiver_id)
    }

    /// Tworzy cel wskazujący konkretny odbiornik w radiu.
    pub fn with_receiver(rig_id: RigId, receiver_id: ReceiverId) -> Self {
        Self {
            rig_id,
            receiver_id: Some(receiver_id),
        }
    }

    /// Sprawdza, czy cel wskazuje całe radio bez wybranego odbiornika.
    pub fn is_entire_rig(&self) -> bool {
        self.receiver_id.is_none()
    }

    /// Identyfikator radia docelowego.
    pub fn rig_id(&self) -> &RigId {
        &self.rig_id
    }

    /// Opcjonalny identyfikator odbiornika docelowego.
    pub fn receiver_id(&self) -> Option<&ReceiverId> {
        self.receiver_id.as_ref()
    }
}

impl fmt::Display for RigTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref rx) = self.receiver_id {
            write!(f, "{}:{}", self.rig_id, rx)
        } else {
            write!(f, "{}", self.rig_id)
        }
    }
}

/// Silnie typowany aktywny cel odbiorczy (Active RX).
///
/// Reprezentuje radio i odbiornik, na których operator aktualnie prowadzi nasłuch
/// lub do których kierowane są polecenia strojenia odbiornika.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActiveRxTarget(RigTarget);

impl ActiveRxTarget {
    /// Tworzy aktywny cel odbiorczy.
    pub fn new(target: RigTarget) -> Self {
        Self(target)
    }

    /// Referencja do wewnętrznego [`RigTarget`].
    pub fn target(&self) -> &RigTarget {
        &self.0
    }

    /// Identyfikator docelowego radia.
    pub fn rig_id(&self) -> &RigId {
        &self.0.rig_id
    }

    /// Opcjonalny identyfikator odbiornika.
    pub fn receiver_id(&self) -> Option<&ReceiverId> {
        self.0.receiver_id.as_ref()
    }

    /// Konsumuje wrapper, zwracając bazowy [`RigTarget`].
    pub fn into_inner(self) -> RigTarget {
        self.0
    }

    /// Konsumuje wrapper, zwracając bazowy [`RigTarget`].
    pub fn into_target(self) -> RigTarget {
        self.0
    }
}

/// Silnie typowany aktywny cel nadawczy (Active TX).
///
/// Reprezentuje radio i odbiornik/slice, do którego operator zamierza skierować
/// nadawanie w przypadku aktywacji PTT.
///
/// # Ważne zasady bezpieczeństwa:
/// - Zmiana `ActiveTxTarget` nigdy samoczynnie nie załącza PTT.
/// - Sam wpis w `ActiveTxTarget` nie oznacza przyznania prawa do nadawania (wymagany jest arbitraż [`crate::cat::tx_arbiter::TxArbiter`]).
/// - W czasie, gdy jakiekolwiek radio znajduje się w stanie `Owned`, zmiana `ActiveTxTarget` na inne radio jest zablokowana.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActiveTxTarget(RigTarget);

impl ActiveTxTarget {
    /// Tworzy aktywny cel nadawczy.
    pub fn new(target: RigTarget) -> Self {
        Self(target)
    }

    /// Referencja do wewnętrznego [`RigTarget`].
    pub fn target(&self) -> &RigTarget {
        &self.0
    }

    /// Identyfikator docelowego radia.
    pub fn rig_id(&self) -> &RigId {
        &self.0.rig_id
    }

    /// Opcjonalny identyfikator odbiornika/slice nadawczego.
    pub fn receiver_id(&self) -> Option<&ReceiverId> {
        self.0.receiver_id.as_ref()
    }

    /// Konsumuje wrapper, zwracając bazowy [`RigTarget`].
    pub fn into_inner(self) -> RigTarget {
        self.0
    }

    /// Konsumuje wrapper, zwracając bazowy [`RigTarget`].
    pub fn into_target(self) -> RigTarget {
        self.0
    }
}

/// Błędy walidacji identyfikatora [`RigCommandId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigCommandIdError {
    /// Wartość 0 jest zarezerwowana i niedozwolona w architekturze.
    ZeroReserved,
}

impl fmt::Display for RigCommandIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroReserved => write!(
                f,
                "wartość 0 jest zarezerwowana w architekturze RigCommandId"
            ),
        }
    }
}

impl std::error::Error for RigCommandIdError {}

/// Monotoniczny, bezkolizyjny identyfikator polecenia CAT w obrębie instancji [`crate::cat::hub::RigHub`].
///
/// Wartość 0 jest zarezerwowana (reprezentowana przez [`NonZeroU64`]).
/// Identyfikator nie zawiera żadnych danych poufnych użytkownika.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RigCommandId(NonZeroU64);

impl RigCommandId {
    /// Jawna deklaracja polityki: identyfikator o wartości 0 jest zarezerwowany.
    pub const ZERO_RESERVED: bool = true;

    /// Tworzy identyfikator z wartości niezerowej.
    pub const fn from_nonzero(id: NonZeroU64) -> Self {
        Self(id)
    }

    /// Bezpiecznie tworzy identyfikator z liczby u64 z jawną obsługą błędu zera (bez paniki).
    pub fn try_new(id: u64) -> Result<Self, RigCommandIdError> {
        NonZeroU64::new(id)
            .map(Self)
            .ok_or(RigCommandIdError::ZeroReserved)
    }

    /// Tworzy identyfikator z liczby u64 (dla kompatybilności w testach).
    /// W przypadku podania wartości 0 zwraca bezpieczny identyfikator minimalny bez paniki.
    pub fn new(id: u64) -> Self {
        Self::from_u64(id).unwrap_or(Self(NonZeroU64::MIN))
    }

    /// Tworzy identyfikator z liczby u64, o ile nie jest zerem.
    pub fn from_u64(id: u64) -> Option<Self> {
        NonZeroU64::new(id).map(Self)
    }

    /// Zwraca surową wartość liczbową u64.
    pub const fn as_u64(&self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for RigCommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cmd#{}", self.0)
    }
}

impl fmt::Debug for RigCommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RigCommandId({})", self.0)
    }
}

/// Polecenie zaadresowane do konkretnego radia i odbiornika wraz z monotonicznym identyfikatorem i terminem wykonania.
#[derive(Debug, Clone)]
pub struct RoutedRigCommand {
    /// Unikalny identyfikator polecenia.
    pub command_id: RigCommandId,
    /// Cel operacji.
    pub target: RigTarget,
    /// Domenowe polecenie CAT.
    pub command: RigCommand,
    /// Bezwzględny, monotoniczny termin, po którym wykonanie polecenia musi zostać przerwane.
    pub deadline: tokio::time::Instant,
}

impl RoutedRigCommand {
    /// Tworzy nowe zaadresowane polecenie.
    pub fn new(
        command_id: RigCommandId,
        target: RigTarget,
        command: RigCommand,
        deadline: tokio::time::Instant,
    ) -> Self {
        Self {
            command_id,
            target,
            command,
            deadline,
        }
    }

    /// Tworzy nowe zaadresowane polecenie z limitem czasu względnym (timeout).
    pub fn with_timeout(
        command_id: RigCommandId,
        target: RigTarget,
        command: RigCommand,
        timeout: std::time::Duration,
    ) -> Self {
        Self::new(
            command_id,
            target,
            command,
            tokio::time::Instant::now() + timeout,
        )
    }
}

/// Zdarzenia warstwy wykonawczej Multi-Radio oraz zdarzenia natywne transceiverów.
#[derive(Debug, Clone, PartialEq)]
pub enum RigHubEvent {
    /// Sesja radiowa została zarejestrowana w hubie.
    RigRegistered,
    /// Sesja radiowa została usunięta z huba.
    RigRemoved,
    /// Rozpoczęto nawiązywanie połączenia z radiem.
    RigConnecting,
    /// Trwa synchronizacja stanu początkowego radia.
    RigSynchronizing,
    /// Sesja radiowa jest w pełni połączona i gotowa do pracy.
    RigReady,
    /// Sesja działa w stanie ograniczonym (częściowa utrata funkcji).
    RigDegraded(String),
    /// Rozpoczęto kontrolowane rozłączanie sesji.
    RigDisconnecting,
    /// Sesja została rozłączona.
    RigDisconnected,
    /// Sesja uległa awarii lub zgłosiła błąd I/O.
    RigFailed(String),
    /// Zmieniono aktywny cel odbiorczy (Active RX).
    ActiveRxChanged(Option<RigTarget>),
    /// Zmieniono aktywny cel nadawczy (Active TX).
    ActiveTxChanged(Option<RigTarget>),
    /// Zmiana stanu arbitra nadawania TX.
    TxArbiterStateChanged(TxArbiterState),
    /// Odbiorca zdarzeń opóźnił się w kolejce (utracono określoną liczbę zdarzeń).
    EventLagged(u64),
    /// Natywne zdarzenie telemetrii lub transceive odebrane z transceivera.
    Radio(NativeRigEvent),
}

/// Zaadresowane zdarzenie emitowane przez warstwę wykonawczą lub transceiver.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutedRigEvent {
    /// Radio, z którego pochodzi zdarzenie.
    pub rig_id: RigId,
    /// Opcjonalny odbiornik w obrębie radia, którego dotyczy zdarzenie.
    pub receiver_id: Option<ReceiverId>,
    /// Treść zdarzenia.
    pub event: RigHubEvent,
}

impl RoutedRigEvent {
    /// Tworzy nowe zaadresowane zdarzenie.
    pub fn new(rig_id: RigId, receiver_id: Option<ReceiverId>, event: RigHubEvent) -> Self {
        Self {
            rig_id,
            receiver_id,
            event,
        }
    }
}

/// Błędy routingu i adresowania komend w warstwie Multi-Radio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    /// Błąd walidacji identyfikatora radia.
    InvalidRigId(RigIdError),
    /// Błąd walidacji identyfikatora odbiornika.
    InvalidReceiverId(ReceiverIdError),
    /// Radio o podanym identyfikatorze jest już zarejestrowane w hubie.
    RigAlreadyExists(RigId),
    /// Wskazane radio nie istnieje w rejestrze huba.
    RigNotFound(RigId),
    /// Wskazany odbiornik nie istnieje w danym radiu.
    ReceiverNotFound {
        rig_id: RigId,
        receiver_id: ReceiverId,
    },
    /// Dana operacja wymaga jawnego wskazania odbiornika.
    ReceiverRequired(RigId),
    /// Radio nie jest w stanie gotowości do przyjmowania komend.
    RigNotReady { rig_id: RigId, state: String },
    /// Operacja nie jest obsługiwana przez wskazane radio lub odbiornik.
    Unsupported { rig_id: RigId, operation: String },
    /// Kolejka komend wskazanego radia jest przepełniona.
    QueueFull(RigId),
    /// Przekroczono monotoniczny termin wykonania polecenia.
    DeadlineExceeded(RigId),
    /// Kanał odpowiedzi na polecenie został zamknięty przed odesłaniem wyniku.
    ResponseChannelClosed(RigId),
    /// Niejednoznaczny cel operacji (np. próba nadawania bez określonego odbiornika TX).
    AmbiguousTarget(String),
    /// Niezgodność właściciela (np. próba zwolnienia PTT przez radio niebędące właścicielem).
    OwnershipMismatch { target_rig: RigId, owner_rig: RigId },
    /// Hub jest w trakcie kontrolowanego zamykania i odrzuca nowe komendy.
    HubShuttingDown,
    /// Usługa wskazanego radia jest w trakcie zamykania.
    ServiceShuttingDown(RigId),
    /// Niedozwolone przejście cyklu życia usługi radiowej.
    InvalidLifecycleTransition { from: String, to: String },
    /// Nadajnik jest zajęty przez inną radiostację.
    TxBusy { current_owner: RigId },
    /// Stan nadajnika jest niejednoznaczny (blokada bezpieczeństwa Unknown).
    TxStateUnknown { reason: String },
    /// Niezgodność właściciela TX w operacji nadawania.
    TxOwnerMismatch {
        expected: RigId,
        actual: Option<RigId>,
    },
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRigId(e) => write!(f, "błąd identyfikatora radia: {e}"),
            Self::InvalidReceiverId(e) => write!(f, "błąd identyfikatora odbiornika: {e}"),
            Self::RigAlreadyExists(id) => write!(f, "radio '{id}' jest już zarejestrowane"),
            Self::RigNotFound(id) => write!(f, "radio '{id}' nie zostało odnalezione"),
            Self::ReceiverNotFound {
                rig_id,
                receiver_id,
            } => write!(
                f,
                "odbiornik '{receiver_id}' nie istnieje w radiu '{rig_id}'"
            ),
            Self::ReceiverRequired(id) => {
                write!(
                    f,
                    "operacja na radiu '{id}' wymaga jawnego wskazania odbiornika"
                )
            }
            Self::RigNotReady { rig_id, state } => {
                write!(f, "radio '{rig_id}' nie jest gotowe (stan: {state})")
            }
            Self::Unsupported { rig_id, operation } => {
                write!(
                    f,
                    "operacja '{operation}' nie jest wspierana przez radio '{rig_id}'"
                )
            }
            Self::QueueFull(id) => write!(f, "kolejka komend radia '{id}' jest przepełniona"),
            Self::DeadlineExceeded(id) => {
                write!(f, "przekroczono limit czasu komendy dla radia '{id}'")
            }
            Self::ResponseChannelClosed(id) => {
                write!(f, "kanał odpowiedzi dla radia '{id}' został zamknięty")
            }
            Self::AmbiguousTarget(desc) => write!(f, "niejednoznaczny cel operacji: {desc}"),
            Self::OwnershipMismatch {
                target_rig,
                owner_rig,
            } => write!(
                f,
                "niezgodność właściciela: target '{target_rig}', faktyczny właściciel '{owner_rig}'"
            ),
            Self::HubShuttingDown => write!(f, "RigHub jest w trakcie zamykania"),
            Self::ServiceShuttingDown(id) => {
                write!(f, "usługa radia '{id}' jest w trakcie zamykania")
            }
            Self::InvalidLifecycleTransition { from, to } => {
                write!(f, "niedozwolone przejście cyklu życia: {from} -> {to}")
            }
            Self::TxBusy { current_owner } => {
                write!(f, "nadajnik jest zajęty przez radio '{current_owner}'")
            }
            Self::TxStateUnknown { reason } => {
                write!(f, "stan nadajnika jest nieznany (Unknown): {reason}")
            }
            Self::TxOwnerMismatch { expected, actual } => {
                write!(
                    f,
                    "niezgodność właściciela TX: oczekiwano '{expected}', aktualny '{actual:?}'"
                )
            }
        }
    }
}

impl std::error::Error for RoutingError {}

impl From<RigIdError> for RoutingError {
    fn from(e: RigIdError) -> Self {
        Self::InvalidRigId(e)
    }
}

impl From<ReceiverIdError> for RoutingError {
    fn from(e: ReceiverIdError) -> Self {
        Self::InvalidReceiverId(e)
    }
}
