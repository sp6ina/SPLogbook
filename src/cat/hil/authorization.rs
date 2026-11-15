// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Niezależny model dowodowy i bramka autoryzacji sprzętowego wykonania HIL.
//!
//! Ten moduł usuwa programową samocertyfikację gotowości HIL (F-15). Rozdziela cztery
//! nieprzenikalne warstwy:
//!
//! 1. **Konfiguracja** ([`HilTestBenchManifest`]) — wyłącznie plan testu. Plan nie zawiera
//!    żadnego dowodu.
//! 2. **Oświadczenie operatora** ([`HilAttestationDocument`]) — deklaracja intencji i stanu
//!    zgłoszonego przez operatora. Oświadczenie **nie jest** obiektywnym dowodem sprzętowym.
//! 3. **Niezależny dowód** ([`HilIndependentEvidence`]) — pochodzi z osobnego, jawnego kanału
//!    (osobny dokument, osobny podmiot wydający) i jest związany skrótem z planem oraz
//!    oświadczeniem.
//! 4. **Decyzja autoryzacyjna** ([`HilAuthorizationDecision`]) — wynik walidatora. Domyślnie
//!    `NotEvaluated`; wszystkie stany pośrednie są traktowane jak odmowa.
//!
//! Zasady fail-closed:
//! - brak dokumentu dowodowego ⇒ [`HilAuthorizationOutcome::EvidenceMissing`],
//! - brak pola w dowodzie ⇒ odmowa (dowód musi być kompletny), a nie „true”,
//! - brak powiązania (`plan_sha256`, `scope`, `test_case_id`, `rig_id`, `session_id`) ⇒ odmowa,
//! - wygaśnięcie albo „jeszcze nieważny” ⇒ odmowa,
//! - błąd wydawcy dowodu, timeout, anulowanie ⇒ odmowa,
//! - zmierzone fakty sprzętowe są przenoszone wyłącznie przez [`HilIndependentEvidence`];
//!   plan testu **nie potrafi** ich wytworzyć (brak konstruktora w kodzie produkcyjnym),
//! - suchy przebieg (`DryRunOnly`) nigdy nie upoważnia PTT.
//!
//! Konstruktory dowodu są dostępne wyłącznie w zakresie testowym (`#[cfg(test)]`).
//! W kodzie produkcyjnym nie istnieje żaden dostawca dowodu, więc
//! [`HilHardwareExecutionAuthorizer::authorize_at`] zawsze zwraca odmowę dla `evidence = None`.
//! Nie istnieje produkcyjne CLI, GUI, konfiguracja, zmienna środowiskowa ani feature flag,
//! która mogłaby wybrać testowy dostawca dowodu.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::manifest::HilTestBenchManifest;

/// Wersja schematu dokumentu dowodowego i decyzji autoryzacyjnej (F-15).
pub const HIL_AUTHORIZATION_SCHEMA_VERSION: u32 = 2;

/// Wersja schematu dokumentu oświadczenia operatora (F-15).
pub const HIL_ATTESTATION_SCHEMA_VERSION: u32 = 2;

/// Kanoniczna domena skrótu SHA-256 dla planu testu HIL.
pub const HIL_PLAN_DIGEST_DOMAIN: &str = "SPLogbook-HIL-TestPlan-v2";

/// Kanoniczna domena skrótu SHA-256 dla oświadczenia operatora HIL.
pub const HIL_ATTESTATION_DIGEST_DOMAIN: &str = "SPLogbook-HIL-OperatorAttestation-v2";

/// Zakres uprawnień żądanych przez dowód.
///
/// Dowód wydany dla węższego zakresu nie może odblokować szerszego wykonania.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HilEvidenceScope {
    /// Wyłącznie walidacja konfiguracji i modelu offline; brak dostępu do sprzętu.
    ConfigurationOnly,
    /// Suchy przebieg bez PTT i bez otwierania portu sprzętowego.
    DryRunNoPtt,
    /// Autoryzacja fizycznego wykonania PTT na stanowisku laboratoryjnym.
    HardwarePtt,
}

impl HilEvidenceScope {
    /// Czy zakres obejmuje fizyczne wykonanie PTT.
    pub fn covers_hardware_ptt(self) -> bool {
        matches!(self, Self::HardwarePtt)
    }

    /// Stabilna nazwa zakresu (audyt, logi).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigurationOnly => "configuration_only",
            Self::DryRunNoPtt => "dry_run_no_ptt",
            Self::HardwarePtt => "hardware_ptt",
        }
    }
}

impl fmt::Display for HilEvidenceScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Zmierzone fakty sprzętowe wymagane przez politykę autoryzacji HIL.
///
/// Każdy fakt musi zostać zmierzony przez podmiot wydający dowód. Plan testu i oświadczenie
/// operatora nie mogą go wytworzyć.
pub const HIL_REQUIRED_MEASURED_FACTS: [&str; 20] = [
    "repository.working_tree_clean",
    "software.baseline_green",
    "repository.tested_commit_exists",
    "antenna.disconnected",
    "dummy_load.connected",
    "dummy_load.impedance_50_ohm",
    "dummy_load.power_rating",
    "amplifier.disabled_or_locked",
    "tuner.bypassed_or_disabled",
    "station.physical_emergency_stop",
    "station.rf_perimeter_secured",
    "station.operator_present",
    "station.operator_acknowledged",
    "station.abort_procedure_available",
    "operator.rules_and_privileges",
    "station.stop_procedure_briefed",
    "network.smartlink_and_unattended_tx_disabled",
    "network.endpoint_local_or_controlled",
    "station.interlock_engaged",
    "station.out_of_band_data_plane_inactive",
];

/// Błąd dowodu lub autoryzacji. Każdy wariant jest kontrolowaną odmową, nigdy sukcesem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HilEvidenceError {
    /// Brak dokumentu dowodowego.
    Missing,
    /// Dokument dowodowy jest niekompletny albo strukturalnie niepoprawny.
    Malformed(String),
    /// Nieobsługiwana wersja schematu dowodu.
    UnsupportedSchema(u32),
    /// Podmiot wydający nie jest jawnie dopuszczony przez politykę.
    UnsupportedIssuer(String),
    /// Dowód nie został jeszcze ważny.
    NotYetValid(String),
    /// Dowód wygasł.
    Expired(String),
    /// Dowód został wydany dla innego planu (niezgodny skrót).
    PlanMismatch,
    /// Dowód został wydany dla innego oświadczenia operatora.
    AttestationMismatch,
    /// Dowód został wydany dla innego zakresu.
    ScopeMismatch {
        /// Zakres wymagany.
        required: HilEvidenceScope,
        /// Zakres zadeklarowany w dowodzie.
        declared: HilEvidenceScope,
    },
    /// Dowód wskazuje inny test, inne radio albo inną sesję.
    BindingMismatch {
        /// Nazwa niezgodnego powiązania.
        field: &'static str,
        /// Wartość wymagana.
        expected: String,
        /// Wartość zadeklarowana w dowodzie.
        declared: String,
    },
    /// Brakuje wymaganego zmierzonego faktu.
    MissingMeasuredFact(&'static str),
    /// Fakt został zmierzony jako fałszywy.
    MeasuredFactFalse(&'static str),
    /// Ten sam dowód został już użyty (replay).
    Replayed {
        /// Identyfikator próby.
        attempt_id: String,
    },
    /// Dostawca dowodu zwrócił błąd.
    ProviderError(String),
    /// Dostawca dowodu przekroczył limit czasu.
    ProviderTimeout,
    /// Operacja została anulowana.
    Cancelled,
    /// Wystawianie dowodu jest niedostępne w kodzie produkcyjnym.
    IssuanceUnavailableInProduction,
}

impl fmt::Display for HilEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "independent HIL evidence document is missing"),
            Self::Malformed(reason) => write!(f, "independent HIL evidence is malformed: {reason}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported HIL evidence schema version: {version}")
            }
            Self::UnsupportedIssuer(issuer) => {
                write!(f, "unsupported HIL evidence issuer: '{issuer}'")
            }
            Self::NotYetValid(at) => write!(f, "HIL evidence is not valid before {at}"),
            Self::Expired(at) => write!(f, "HIL evidence expired at {at}"),
            Self::PlanMismatch => write!(
                f,
                "HIL evidence plan digest does not match the evaluated test plan"
            ),
            Self::AttestationMismatch => write!(
                f,
                "HIL evidence attestation digest does not match the supplied operator attestation"
            ),
            Self::ScopeMismatch { required, declared } => write!(
                f,
                "HIL evidence scope mismatch: required '{required}', declared '{declared}'"
            ),
            Self::BindingMismatch {
                field,
                expected,
                declared,
            } => write!(
                f,
                "HIL evidence binding mismatch for '{field}': expected '{expected}', declared '{declared}'"
            ),
            Self::MissingMeasuredFact(fact) => {
                write!(f, "HIL evidence is missing a measured fact: '{fact}'")
            }
            Self::MeasuredFactFalse(fact) => {
                write!(f, "HIL evidence measured '{fact}' as false")
            }
            Self::Replayed { attempt_id } => {
                write!(f, "HIL evidence replayed for attempt '{attempt_id}'")
            }
            Self::ProviderError(reason) => write!(f, "HIL evidence provider error: {reason}"),
            Self::ProviderTimeout => write!(f, "HIL evidence provider timed out"),
            Self::Cancelled => write!(f, "HIL evidence provider was cancelled"),
            Self::IssuanceUnavailableInProduction => write!(
                f,
                "HIL evidence issuance is unavailable in production builds"
            ),
        }
    }
}

impl std::error::Error for HilEvidenceError {}

impl From<HilEvidenceError> for super::error::HilError {
    fn from(err: HilEvidenceError) -> Self {
        use super::error::HilError;
        match err {
            HilEvidenceError::Missing
            | HilEvidenceError::Malformed(_)
            | HilEvidenceError::UnsupportedSchema(_)
            | HilEvidenceError::UnsupportedIssuer(_)
            | HilEvidenceError::PlanMismatch
            | HilEvidenceError::AttestationMismatch
            | HilEvidenceError::ScopeMismatch { .. }
            | HilEvidenceError::BindingMismatch { .. }
            | HilEvidenceError::MissingMeasuredFact(_)
            | HilEvidenceError::MeasuredFactFalse(_)
            | HilEvidenceError::IssuanceUnavailableInProduction => {
                HilError::NotHilReady(err.to_string())
            }
            HilEvidenceError::NotYetValid(_) | HilEvidenceError::Expired(_) => {
                HilError::SessionExpired(err.to_string())
            }
            HilEvidenceError::Replayed { .. }
            | HilEvidenceError::ProviderError(_)
            | HilEvidenceError::ProviderTimeout
            | HilEvidenceError::Cancelled => HilError::UnauthorizedSession(err.to_string()),
        }
    }
}

/// Pieczęć dowodu. Pole prywatne — struktury nie da się zbudować literałem strukturalnym poza
/// tym modułem, a konstruktory są dostępne wyłącznie w zakresie testowym (`#[cfg(test)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct EvidenceSeal(());

/// Niezależny dokument dowodowy upoważniający wykonanie sprzętowe HIL.
///
/// Dowód jest osobnym dokumentem od planu testu i od oświadczenia operatora. Jest związany
/// skrótem SHA-256 z planem **oraz** z oświadczeniem operatora, wskazuje test, radio, sesję
/// i próbę oraz ma jawny zakres i okno ważności.
///
/// Dowód nie jest podpisem kryptograficznym: `plan_sha256` to skrót integralności,
/// a wiarygodność zapewnia jawnie wskazany podmiot wydający spoza planu testu
/// ([`HilIndependentEvidence::issuer`]) oraz brak jakiegokolwiek produkcyjnego dostawcy
/// dowodu w tej bramce.
///
/// Typ **nie** implementuje `Deserialize`: dokumentu dowodowego nie można wczytać z pliku,
/// konfiguracji, CLI ani zmiennej środowiskowej. Dowód może powstać wyłącznie w kodzie
/// posiadającym pieczęć konstruktora (zakres `#[cfg(test)]`). Dzięki temu ręcznie spreparowany
/// JSON nie może sfałszować upoważnienia w kodzie produkcyjnym.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HilIndependentEvidence {
    /// Wersja schematu dokumentu dowodowego.
    pub schema_version: u32,
    /// Unikalny identyfikator dowodu.
    pub evidence_id: String,
    /// Podmiot wydający dowód (jednoznaczny lokalny identyfikator authority).
    pub issuer: String,
    /// Czas wydania dowodu (RFC 3339).
    pub issued_at: String,
    /// Opcjonalny czas wygaśnięcia dowodu (RFC 3339).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Zakres, dla którego dowód został wydany.
    pub scope: HilEvidenceScope,
    /// SHA-256 kanonicznej postaci planu testu.
    pub plan_sha256: String,
    /// SHA-256 kanonicznej postaci oświadczenia operatora.
    pub attestation_sha256: String,
    /// Identyfikator przypadku testowego.
    pub test_case_id: String,
    /// Identyfikator radia.
    pub rig_id: String,
    /// Identyfikator sesji laboratoryjnej.
    pub session_id: String,
    /// Identyfikator próby (ochrona przed powtórzeniem).
    pub attempt_id: String,
    /// Zmierzone fakty sprzętowe (nazwa faktu → wartość zmierzona).
    pub measured_facts: BTreeMap<String, bool>,
    /// Status integralności dokumentu w momencie walidacji.
    #[serde(default)]
    pub integrity_ok: bool,
    /// Pieczęć konstruktora — nigdy nie jest serializowana.
    #[serde(skip)]
    seal: EvidenceSeal,
}

impl HilIndependentEvidence {
    /// Zwraca identyfikator dowodu w bezpiecznej, audytowalnej postaci.
    pub fn audit_id(&self) -> &str {
        &self.evidence_id
    }

    /// Zwraca skrócony (12 znaków) skrót planu, bezpieczny do logowania.
    pub fn plan_digest_prefix(&self) -> String {
        digest_prefix(&self.plan_sha256)
    }

    /// Czy dokument jest kompletny strukturalnie (bez oceny merytorycznej).
    pub fn is_well_formed(&self) -> bool {
        self.schema_version == HIL_AUTHORIZATION_SCHEMA_VERSION
            && !self.evidence_id.trim().is_empty()
            && !self.issuer.trim().is_empty()
            && !self.test_case_id.trim().is_empty()
            && !self.rig_id.trim().is_empty()
            && !self.session_id.trim().is_empty()
            && !self.attempt_id.trim().is_empty()
            && is_sha256_hex(&self.plan_sha256)
            && is_sha256_hex(&self.attestation_sha256)
            && chrono::DateTime::parse_from_rfc3339(&self.issued_at).is_ok()
            && !self.measured_facts.is_empty()
            && self.integrity_ok
    }
}

/// Dokument oświadczenia operatora. Osobny dokument, nie dowód.
///
/// Oświadczenie opisuje stan zgłoszony przez operatora i wiąże się z planem testu skrótem.
/// Nie może samo odblokować wykonania sprzętowego: wymagany jest niezależny
/// [`HilIndependentEvidence`], którego `attestation_sha256` wskazuje na ten dokument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilAttestationDocument {
    /// Wersja schematu dokumentu oświadczenia.
    #[serde(default)]
    pub schema_version: u32,
    /// Identyfikator stanowiska (musi odpowiadać planowi).
    pub bench_id: String,
    /// Identyfikator sesji (musi odpowiadać planowi).
    pub session_id: String,
    /// Identyfikator testu (musi odpowiadać planowi).
    pub test_case_id: String,
    /// Identyfikator radia (musi odpowiadać planowi).
    pub rig_id: String,
    /// Deklarowany skrót SHA-256 planu (wiąże oświadczenie z planem).
    #[serde(default)]
    pub plan_sha256: String,
    /// Oświadczenia operatora.
    pub statements: HilOperatorAttestationStatements,
}

impl HilAttestationDocument {
    /// Oblicza SHA-256 kanonicznej postaci oświadczenia.
    ///
    /// Skrót integralności, **nie** podpis kryptograficzny.
    pub fn attestation_sha256(&self) -> String {
        let canonical = serde_json::to_string(self).unwrap_or_else(|_| String::from("{}"));
        let mut input =
            Vec::with_capacity(HIL_ATTESTATION_DIGEST_DOMAIN.len() + 1 + canonical.len());
        input.extend_from_slice(HIL_ATTESTATION_DIGEST_DOMAIN.as_bytes());
        input.push(b'\n');
        input.extend_from_slice(canonical.as_bytes());
        sha256_hex(&input)
    }
}

/// Oświadczenia operatora (dane deklaratywne, nie dowód).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilOperatorAttestationStatements {
    /// Potwierdzenie fizycznego podłączenia sztucznego obciążenia 50 Ω.
    pub dummy_load_physically_connected: bool,
    /// Potwierdzenie zabezpieczenia strefy RF.
    pub rf_perimeter_secured: bool,
    /// Świadoma, jawna zgoda operatora na próby PTT.
    pub ptt_consent_granted: bool,
    /// Sygnatura czasowa oświadczenia (RFC 3339).
    pub attestation_timestamp: String,
    /// Opis stanowiska i procedury weryfikacji.
    pub description: String,
    /// Potwierdzenie fizycznego odłączenia anteny nadawczej.
    pub antenna_disconnected: bool,
    /// Potwierdzenie odłączenia albo zablokowania wzmacniacza mocy.
    pub pa_disconnected_or_locked: bool,
    /// Potwierdzenie wyłączenia lub zbypassowania tunera.
    pub tuner_bypassed: bool,
    /// Dostępność fizycznego wyłącznika awaryjnego STOP.
    pub physical_stop_available: bool,
    /// Udokumentowana procedura awaryjnego przerwania testu.
    pub abort_procedure_documented: bool,
    /// Formalne potwierdzenie zapoznania się operatora ze stanowiskiem.
    pub operator_acknowledged: bool,
    /// Jawne potwierdzenie gotowości stanowiska przez operatora.
    pub bench_readiness_confirmed: bool,
    /// Potwierdzenie fizycznej obecności operatora przy stanowisku.
    pub operator_present: bool,
    /// Potwierdzenie prowadzenia próby pod bezpośrednim nadzorem operatora.
    pub supervised_trial_confirmed: bool,
    /// Potwierdzenie zgodności próby z regulaminem laboratorium.
    pub rules_compliance_attested: bool,
    /// Potwierdzenie posiadania wymaganych uprawnień operatorskich.
    pub operator_privileges_attested: bool,
    /// Potwierdzenie dopuszczalności pasma częstotliwości.
    pub allowed_band_attested: bool,
    /// Potwierdzenie dopuszczalności rodzaju emisji / modulacji.
    pub allowed_mode_attested: bool,
    /// Potwierdzenie braku nieautoryzowanej emisji radiowej.
    pub no_unauthorized_emission_attested: bool,
    /// Potwierdzenie znajomości procedury STOP przez osoby obecne przy stanowisku.
    pub personnel_know_stop_procedure: bool,
}

impl HilOperatorAttestationStatements {
    /// Czy wszystkie wymagane oświadczenia zostały złożone.
    pub fn all_statements_affirmed(&self) -> bool {
        self.dummy_load_physically_connected
            && self.rf_perimeter_secured
            && self.ptt_consent_granted
            && self.antenna_disconnected
            && self.pa_disconnected_or_locked
            && self.tuner_bypassed
            && self.physical_stop_available
            && self.abort_procedure_documented
            && self.operator_acknowledged
            && self.bench_readiness_confirmed
            && self.operator_present
            && self.supervised_trial_confirmed
            && self.rules_compliance_attested
            && self.operator_privileges_attested
            && self.allowed_band_attested
            && self.allowed_mode_attested
            && self.no_unauthorized_emission_attested
            && self.personnel_know_stop_procedure
    }

    /// Czy oświadczenie ma sygnaturę czasową i opis.
    pub fn has_timestamp_and_description(&self) -> bool {
        !self.attestation_timestamp.trim().is_empty() && !self.description.trim().is_empty()
    }
}

/// Wynik autoryzacji wykonania sprzętowego HIL.
///
/// Wszystkie warianty poza [`Self::HardwareExecutionAuthorized`] są odmową.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
pub enum HilAuthorizationOutcome {
    /// Walidacja nie została wykonana.
    #[default]
    NotEvaluated,
    /// Konfiguracja albo plan testu są niepoprawne.
    ConfigurationInvalid,
    /// Brak oświadczenia operatora.
    OperatorAttestationMissing,
    /// Oświadczenie operatora jest niekompletne albo niezgodne z planem.
    OperatorAttestationInvalid,
    /// Brak niezależnego dowodu sprzętowego.
    EvidenceMissing,
    /// Dowód jest niepoprawny albo niekompletny.
    EvidenceInvalid,
    /// Dowód wygasł.
    EvidenceExpired,
    /// Dowód nie jest jeszcze ważny.
    EvidenceNotYetValid,
    /// Dowód dotyczy innego planu, testu, radia, sesji albo zakresu.
    EvidenceScopeMismatch,
    /// Ten sam dowód został już użyty.
    EvidenceReplayed,
    /// Brak niezależnego zatwierdzenia (dowód nie pochodzi od dopuszczonego podmiotu).
    IndependentApprovalMissing,
    /// Wyłącznie suchy przebieg: brak uprawnienia do PTT.
    DryRunOnly,
    /// Wykonanie sprzętowe zabronione.
    HardwareExecutionDenied,
    /// Wykonanie sprzętowe autoryzowane.
    HardwareExecutionAuthorized,
}

impl HilAuthorizationOutcome {
    /// Czy wynik upoważnia fizyczne wykonanie PTT.
    pub fn authorizes_hardware_ptt(self) -> bool {
        matches!(self, Self::HardwareExecutionAuthorized)
    }

    /// Czy wynik jest kontrolowaną odmową.
    pub fn is_denial(self) -> bool {
        !self.authorizes_hardware_ptt()
    }

    /// Stabilna nazwa wyniku (audyt, logi, testy).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotEvaluated => "NotEvaluated",
            Self::ConfigurationInvalid => "ConfigurationInvalid",
            Self::OperatorAttestationMissing => "OperatorAttestationMissing",
            Self::OperatorAttestationInvalid => "OperatorAttestationInvalid",
            Self::EvidenceMissing => "EvidenceMissing",
            Self::EvidenceInvalid => "EvidenceInvalid",
            Self::EvidenceExpired => "EvidenceExpired",
            Self::EvidenceNotYetValid => "EvidenceNotYetValid",
            Self::EvidenceScopeMismatch => "EvidenceScopeMismatch",
            Self::EvidenceReplayed => "EvidenceReplayed",
            Self::IndependentApprovalMissing => "IndependentApprovalMissing",
            Self::DryRunOnly => "DryRunOnly",
            Self::HardwareExecutionDenied => "HardwareExecutionDenied",
            Self::HardwareExecutionAuthorized => "HardwareExecutionAuthorized",
        }
    }
}

impl fmt::Display for HilAuthorizationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rekord audytowy decyzji autoryzacyjnej — bez sekretów i bez danych uwierzytelniających.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilAuthorizationAudit {
    /// Wersja schematu decyzji.
    pub schema_version: u32,
    /// Czas ewaluacji (RFC 3339) użyty przez walidator.
    pub evaluated_at: String,
    /// Wynik autoryzacji.
    pub outcome: HilAuthorizationOutcome,
    /// Czytelny powód odmowy (bez sekretów).
    pub reason: String,
    /// Brakujący element wskazany przez walidator.
    #[serde(default)]
    pub missing: Option<String>,
    /// Identyfikator dowodu, jeśli bezpieczny do ujawnienia.
    #[serde(default)]
    pub evidence_id: Option<String>,
    /// Zakres dowodu w bezpiecznej postaci.
    #[serde(default)]
    pub scope: Option<String>,
    /// Skrót planu w bezpiecznej, skróconej postaci (12 znaków).
    #[serde(default)]
    pub plan_digest_prefix: Option<String>,
    /// Identyfikator próby.
    #[serde(default)]
    pub attempt_id: Option<String>,
}

impl HilAuthorizationAudit {
    fn new(outcome: HilAuthorizationOutcome, evaluated_at: DateTime<Utc>) -> Self {
        Self {
            schema_version: HIL_AUTHORIZATION_SCHEMA_VERSION,
            evaluated_at: evaluated_at.to_rfc3339(),
            outcome,
            reason: String::new(),
            missing: None,
            evidence_id: None,
            scope: None,
            plan_digest_prefix: None,
            attempt_id: None,
        }
    }

    fn denied(
        outcome: HilAuthorizationOutcome,
        reason: impl Into<String>,
        missing: Option<&str>,
        evaluated_at: DateTime<Utc>,
    ) -> Self {
        let mut audit = Self::new(outcome, evaluated_at);
        audit.reason = reason.into();
        audit.missing = missing.map(str::to_string);
        audit
    }
}

/// Domknięta decyzja autoryzacyjna.
///
/// Struktura nie ma publicznych pól i nie da się jej zbudować poza tym modułem. Jedynym
/// produkcyjnym producentem decyzji jest [`HilHardwareExecutionAuthorizer`]; wariant
/// [`HilAuthorizationOutcome::HardwareExecutionAuthorized`] może powstać wyłącznie po
/// przejściu pełnej walidacji niezależnego dowodu.
///
/// Typ **nie** implementuje `Deserialize`: decyzji nie można wczytać ani sfałszować z JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HilAuthorizationDecision {
    outcome: HilAuthorizationOutcome,
    audit: HilAuthorizationAudit,
}

impl HilAuthorizationDecision {
    /// Decyzja domyślna: brak ewaluacji, brak upoważnienia.
    pub fn not_evaluated() -> Self {
        Self {
            outcome: HilAuthorizationOutcome::NotEvaluated,
            audit: HilAuthorizationAudit::new(HilAuthorizationOutcome::NotEvaluated, Utc::now()),
        }
    }

    fn from_audit(outcome: HilAuthorizationOutcome, audit: HilAuthorizationAudit) -> Self {
        Self { outcome, audit }
    }

    /// Wynik autoryzacji.
    pub fn outcome(&self) -> HilAuthorizationOutcome {
        self.outcome
    }

    /// Czy decyzja upoważnia fizyczne wykonanie PTT.
    pub fn authorizes_hardware_ptt(&self) -> bool {
        self.outcome.authorizes_hardware_ptt()
    }

    /// Rekord audytowy decyzji.
    pub fn audit(&self) -> &HilAuthorizationAudit {
        &self.audit
    }

    /// Czytelny powód odmowy.
    pub fn reason(&self) -> &str {
        &self.audit.reason
    }
}

impl Default for HilAuthorizationDecision {
    fn default() -> Self {
        Self::not_evaluated()
    }
}

impl fmt::Display for HilAuthorizationDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.outcome)
    }
}

/// Wejście walidatora autoryzacyjnego.
#[derive(Debug, Clone, Copy)]
pub struct HilAuthorizationInputs<'a> {
    /// Plan testu (konfiguracja).
    pub plan: &'a HilTestBenchManifest,
    /// Oświadczenie operatora (osobny dokument).
    pub attestation: Option<&'a HilAttestationDocument>,
    /// Niezależny dowód (osobny dokument, z zewnętrznego kanału).
    pub evidence: Option<&'a HilIndependentEvidence>,
}

/// Bramka autoryzacyjna HIL. Manifest testu nie jest authority dla samego siebie.
///
/// Walidator przyjmuje trzy rozłączne wejścia i zwraca domkniętą decyzję. Nie tworzy dowodu,
/// nie zapisuje go i nie „uzupełnia” brakujących pól. Każdy błąd jest kontrolowaną odmową.
pub struct HilHardwareExecutionAuthorizer;

impl HilHardwareExecutionAuthorizer {
    /// Wykonuje pełną walidację i zwraca decyzję autoryzacyjną.
    ///
    /// Brak dowodu ⇒ [`HilAuthorizationOutcome::EvidenceMissing`].
    /// Brak oświadczenia ⇒ [`HilAuthorizationOutcome::OperatorAttestationMissing`].
    pub fn authorize_at(
        inputs: HilAuthorizationInputs<'_>,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        // 1. Konfiguracja / struktura planu.
        if let Err(err) = inputs.plan.validate_plan() {
            let decision = HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::ConfigurationInvalid,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::ConfigurationInvalid,
                    err.to_string(),
                    Some("test_plan"),
                    evaluated_at,
                ),
            );
            log_denial(&decision);
            return decision;
        }

        // 2. Oświadczenie operatora jest wymagane — dowód wiąże się z jego skrótem.
        let Some(attestation) = inputs.attestation else {
            let decision = HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::OperatorAttestationMissing,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::OperatorAttestationMissing,
                    "operator attestation document is required and was not supplied".to_string(),
                    Some("operator_attestation"),
                    evaluated_at,
                ),
            );
            log_denial(&decision);
            return decision;
        };

        let plan_digest = inputs.plan.plan_sha256();
        let attestation_digest = attestation.attestation_sha256();

        // 3. Spójność oświadczenia z planem.
        if let Err(err) = validate_attestation_binding(inputs.plan, attestation, &plan_digest) {
            let decision = HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::OperatorAttestationInvalid,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::OperatorAttestationInvalid,
                    err.to_string(),
                    Some("operator_attestation"),
                    evaluated_at,
                ),
            );
            log_denial(&decision);
            return decision;
        }

        // 4. Dowód niezależny jest wymagany.
        let Some(evidence) = inputs.evidence else {
            let decision = HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::EvidenceMissing,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::EvidenceMissing,
                    format!(
                        "no independent evidence for plan {}; a test plan and an operator \
                         attestation cannot authorize hardware execution",
                        digest_prefix(&plan_digest)
                    ),
                    Some("independent_evidence"),
                    evaluated_at,
                ),
            );
            log_denial(&decision);
            return decision;
        };

        // 5. Walidacja dowodu — każdy błąd jest kontrolowaną odmową.
        match validate_evidence(
            attestation,
            evidence,
            &plan_digest,
            &attestation_digest,
            evaluated_at,
        ) {
            Ok(()) => {
                let mut audit = HilAuthorizationAudit::new(
                    HilAuthorizationOutcome::HardwareExecutionAuthorized,
                    evaluated_at,
                );
                audit.reason = "independent evidence covers every required measured fact for this \
                                plan, attestation, test, rig, session, scope and attempt"
                    .to_string();
                audit.evidence_id = Some(evidence.evidence_id.clone());
                audit.scope = Some(evidence.scope.as_str().to_string());
                audit.plan_digest_prefix = Some(digest_prefix(&plan_digest));
                audit.attempt_id = Some(evidence.attempt_id.clone());
                let decision = HilAuthorizationDecision::from_audit(
                    HilAuthorizationOutcome::HardwareExecutionAuthorized,
                    audit,
                );
                log::info!(
                    "HIL authorization: authorized (evidence_id={}, scope={}, plan={}, attempt={})",
                    evidence.evidence_id,
                    evidence.scope,
                    digest_prefix(&plan_digest),
                    evidence.attempt_id
                );
                decision
            }
            Err(err) => {
                let (outcome, missing) = outcome_for_evidence_error(&err);
                let mut audit =
                    HilAuthorizationAudit::denied(outcome, err.to_string(), missing, evaluated_at);
                audit.evidence_id = Some(evidence.evidence_id.clone());
                audit.scope = Some(evidence.scope.as_str().to_string());
                audit.plan_digest_prefix = Some(digest_prefix(&plan_digest));
                audit.attempt_id = Some(evidence.attempt_id.clone());
                let decision = HilAuthorizationDecision::from_audit(outcome, audit);
                log_denial(&decision);
                decision
            }
        }
    }

    /// Suchy przebieg: waliduje konfigurację i oświadczenie, nigdy nie upoważnia PTT.
    ///
    /// Wynik `DryRunOnly` jest jawnym, nie-wykonawczym sukcesem konfiguracji. Nie może zostać
    /// użyty jako upoważnienie PTT.
    pub fn dry_run_at(
        plan: &HilTestBenchManifest,
        attestation: Option<&HilAttestationDocument>,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        if let Err(err) = plan.validate_plan() {
            return HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::ConfigurationInvalid,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::ConfigurationInvalid,
                    err.to_string(),
                    Some("test_plan"),
                    evaluated_at,
                ),
            );
        }
        let plan_digest = plan.plan_sha256();
        if let Some(attestation) = attestation
            && let Err(err) = validate_attestation_binding(plan, attestation, &plan_digest)
        {
            return HilAuthorizationDecision::from_audit(
                HilAuthorizationOutcome::OperatorAttestationInvalid,
                HilAuthorizationAudit::denied(
                    HilAuthorizationOutcome::OperatorAttestationInvalid,
                    err.to_string(),
                    Some("operator_attestation"),
                    evaluated_at,
                ),
            );
        }
        let mut audit =
            HilAuthorizationAudit::new(HilAuthorizationOutcome::DryRunOnly, evaluated_at);
        audit.reason = "configuration and attestation are valid for a dry run; hardware PTT \
                        remains unauthorized without independent evidence"
            .to_string();
        audit.plan_digest_prefix = Some(digest_prefix(&plan_digest));
        HilAuthorizationDecision::from_audit(HilAuthorizationOutcome::DryRunOnly, audit)
    }

    /// Weryfikuje, czy podany dowód dotyczy dokładnie tego planu.
    ///
    /// Funkcja diagnostyczna (audyt, testy); nie upoważnia niczego.
    pub fn evidence_matches_plan(
        plan: &HilTestBenchManifest,
        attestation: &HilAttestationDocument,
        evidence: &HilIndependentEvidence,
    ) -> bool {
        let plan_digest = plan.plan_sha256();
        validate_attestation_binding(plan, attestation, &plan_digest).is_ok()
            && evidence.plan_sha256 == plan_digest
            && evidence.attestation_sha256 == attestation.attestation_sha256()
            && evidence.test_case_id == plan.test_case_id
            && evidence.rig_id == plan.matrix.rig_id.as_str()
            && evidence.session_id == plan.session_id
    }
}

fn log_denial(decision: &HilAuthorizationDecision) {
    let audit = decision.audit();
    log::warn!(
        "HIL authorization denied: outcome={}, reason={}, missing={}, plan={}, attempt={}",
        audit.outcome,
        audit.reason,
        audit.missing.as_deref().unwrap_or("none"),
        audit.plan_digest_prefix.as_deref().unwrap_or("none"),
        audit.attempt_id.as_deref().unwrap_or("none")
    );
}

fn outcome_for_evidence_error(
    err: &HilEvidenceError,
) -> (HilAuthorizationOutcome, Option<&'static str>) {
    match err {
        HilEvidenceError::Missing => (HilAuthorizationOutcome::EvidenceMissing, Some("evidence")),
        HilEvidenceError::Malformed(_) => (
            HilAuthorizationOutcome::EvidenceInvalid,
            Some("evidence_fields"),
        ),
        HilEvidenceError::UnsupportedSchema(_) => (
            HilAuthorizationOutcome::EvidenceInvalid,
            Some("schema_version"),
        ),
        HilEvidenceError::UnsupportedIssuer(_) => (
            HilAuthorizationOutcome::IndependentApprovalMissing,
            Some("issuer"),
        ),
        HilEvidenceError::NotYetValid(_) => (
            HilAuthorizationOutcome::EvidenceNotYetValid,
            Some("issued_at"),
        ),
        HilEvidenceError::Expired(_) => {
            (HilAuthorizationOutcome::EvidenceExpired, Some("expires_at"))
        }
        HilEvidenceError::PlanMismatch => (
            HilAuthorizationOutcome::EvidenceScopeMismatch,
            Some("plan_sha256"),
        ),
        HilEvidenceError::AttestationMismatch => (
            HilAuthorizationOutcome::EvidenceScopeMismatch,
            Some("attestation_sha256"),
        ),
        HilEvidenceError::ScopeMismatch { .. } => (
            HilAuthorizationOutcome::EvidenceScopeMismatch,
            Some("scope"),
        ),
        HilEvidenceError::BindingMismatch { field, .. } => {
            (HilAuthorizationOutcome::EvidenceScopeMismatch, Some(field))
        }
        HilEvidenceError::MissingMeasuredFact(fact) | HilEvidenceError::MeasuredFactFalse(fact) => {
            (HilAuthorizationOutcome::EvidenceInvalid, Some(fact))
        }
        HilEvidenceError::Replayed { .. } => (
            HilAuthorizationOutcome::EvidenceReplayed,
            Some("attempt_id"),
        ),
        HilEvidenceError::ProviderError(_)
        | HilEvidenceError::ProviderTimeout
        | HilEvidenceError::Cancelled => (
            HilAuthorizationOutcome::EvidenceInvalid,
            Some("evidence_provider"),
        ),
        HilEvidenceError::IssuanceUnavailableInProduction => (
            HilAuthorizationOutcome::EvidenceMissing,
            Some("evidence_provider"),
        ),
    }
}

fn validate_attestation_binding(
    plan: &HilTestBenchManifest,
    attestation: &HilAttestationDocument,
    plan_digest: &str,
) -> Result<(), HilEvidenceError> {
    if attestation.schema_version != HIL_ATTESTATION_SCHEMA_VERSION {
        return Err(HilEvidenceError::UnsupportedSchema(
            attestation.schema_version,
        ));
    }
    if attestation.bench_id != plan.bench_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "bench_id",
            expected: plan.bench_id.clone(),
            declared: attestation.bench_id.clone(),
        });
    }
    if attestation.session_id != plan.session_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "session_id",
            expected: plan.session_id.clone(),
            declared: attestation.session_id.clone(),
        });
    }
    if attestation.test_case_id != plan.test_case_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "test_case_id",
            expected: plan.test_case_id.clone(),
            declared: attestation.test_case_id.clone(),
        });
    }
    if attestation.rig_id != plan.matrix.rig_id.as_str() {
        return Err(HilEvidenceError::BindingMismatch {
            field: "rig_id",
            expected: plan.matrix.rig_id.as_str().to_string(),
            declared: attestation.rig_id.clone(),
        });
    }
    if attestation.plan_sha256 != plan_digest {
        return Err(HilEvidenceError::PlanMismatch);
    }
    if !attestation.statements.has_timestamp_and_description() {
        return Err(HilEvidenceError::Malformed(
            "operator attestation requires a timestamp and a description".to_string(),
        ));
    }
    if !attestation.statements.all_statements_affirmed() {
        return Err(HilEvidenceError::Malformed(
            "operator attestation is incomplete; every required statement must be explicitly affirmed"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_evidence(
    attestation: &HilAttestationDocument,
    evidence: &HilIndependentEvidence,
    plan_digest: &str,
    attestation_digest: &str,
    evaluated_at: DateTime<Utc>,
) -> Result<(), HilEvidenceError> {
    if evidence.schema_version != HIL_AUTHORIZATION_SCHEMA_VERSION {
        return Err(HilEvidenceError::UnsupportedSchema(evidence.schema_version));
    }
    if !evidence.is_well_formed() {
        return Err(HilEvidenceError::Malformed(
            "evidence document is incomplete, has an invalid digest or failed its integrity check"
                .to_string(),
        ));
    }
    if !is_supported_issuer(&evidence.issuer) {
        return Err(HilEvidenceError::UnsupportedIssuer(evidence.issuer.clone()));
    }

    let issued_at = chrono::DateTime::parse_from_rfc3339(&evidence.issued_at)
        .map_err(|e| HilEvidenceError::Malformed(format!("issued_at is not RFC 3339: {e}")))?
        .with_timezone(&Utc);
    if issued_at > evaluated_at {
        return Err(HilEvidenceError::NotYetValid(evidence.issued_at.clone()));
    }
    if let Some(expires_at) = &evidence.expires_at {
        let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at)
            .map_err(|e| HilEvidenceError::Malformed(format!("expires_at is not RFC 3339: {e}")))?
            .with_timezone(&Utc);
        if expires_at <= issued_at {
            return Err(HilEvidenceError::Malformed(
                "expires_at must be later than issued_at".to_string(),
            ));
        }
        if evaluated_at >= expires_at {
            return Err(HilEvidenceError::Expired(expires_at.to_rfc3339()));
        }
    }

    if !evidence.scope.covers_hardware_ptt() {
        return Err(HilEvidenceError::ScopeMismatch {
            required: HilEvidenceScope::HardwarePtt,
            declared: evidence.scope,
        });
    }
    if evidence.plan_sha256 != plan_digest {
        return Err(HilEvidenceError::PlanMismatch);
    }
    if evidence.attestation_sha256 != attestation_digest {
        return Err(HilEvidenceError::AttestationMismatch);
    }
    if evidence.test_case_id != attestation.test_case_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "test_case_id",
            expected: attestation.test_case_id.clone(),
            declared: evidence.test_case_id.clone(),
        });
    }
    if evidence.rig_id != attestation.rig_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "rig_id",
            expected: attestation.rig_id.clone(),
            declared: evidence.rig_id.clone(),
        });
    }
    if evidence.session_id != attestation.session_id {
        return Err(HilEvidenceError::BindingMismatch {
            field: "session_id",
            expected: attestation.session_id.clone(),
            declared: evidence.session_id.clone(),
        });
    }

    for fact in HIL_REQUIRED_MEASURED_FACTS {
        match evidence.measured_facts.get(fact) {
            Some(true) => {}
            Some(false) => return Err(HilEvidenceError::MeasuredFactFalse(fact)),
            None => return Err(HilEvidenceError::MissingMeasuredFact(fact)),
        }
    }
    Ok(())
}

/// Podmioty wydające dowód jawnie dopuszczone przez bieżącą politykę.
///
/// Lista jest zamknięta i zawiera wyłącznie lokalne identyfikatory authority. Żaden z nich nie
/// jest implementowany w kodzie produkcyjnym tej bramki — dopuszczenie identyfikatora nie tworzy
/// dostawcy dowodu. Wpisy dotyczą wyłącznie testów jednostkowych (`cfg(test)`), które budują
/// dokumenty w pamięci, aby zweryfikować logikę walidatora.
pub const HIL_SUPPORTED_EVIDENCE_ISSUERS: [&str; 2] = [
    "splogbook-test-authority/offline",
    "splogbook-lab-authority/reserved-not-implemented",
];

/// Czy podmiot wydający jest jawnie dopuszczony.
pub fn is_supported_issuer(issuer: &str) -> bool {
    HIL_SUPPORTED_EVIDENCE_ISSUERS.contains(&issuer)
}

/// Czy tekst jest poprawnym, małymi literami zapisanym skrótem SHA-256 (64 znaki hex).
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Pierwsze 12 znaków skrótu — bezpieczna postać do logów i audytu.
pub fn digest_prefix(value: &str) -> String {
    value.chars().take(12).collect()
}

/// Oblicza SHA-256 dla ciągu bajtów w postaci małymi literami zapisanego hex.
///
/// Jest to skrót integralności, **nie** podpis kryptograficzny.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Rejestr zużytych prób (ochrona przed powtórzeniem dowodu w obrębie sesji procesu).
///
/// Rejestr jest lokalny i deterministyczny; nie korzysta z zegara ściennego ani z sieci.
#[derive(Debug, Default)]
pub struct HilAttemptRegistry {
    consumed: std::sync::Mutex<std::collections::BTreeSet<String>>,
}

impl HilAttemptRegistry {
    /// Tworzy pusty rejestr.
    pub fn new() -> Self {
        Self::default()
    }

    /// Klucz próby: sesja + identyfikator próby.
    fn key(session_id: &str, attempt_id: &str) -> String {
        format!("{session_id}\u{1f}{attempt_id}")
    }

    /// Sprawdza, czy próba została już zużyta.
    pub fn is_consumed(&self, session_id: &str, attempt_id: &str) -> bool {
        match self.consumed.lock() {
            Ok(guard) => guard.contains(&Self::key(session_id, attempt_id)),
            Err(poisoned) => poisoned
                .into_inner()
                .contains(&Self::key(session_id, attempt_id)),
        }
    }

    /// Zużywa próbę. Zwraca `Err` przy powtórzeniu; nigdy nie panikuje na zatrutym mutexie.
    pub fn consume(&self, session_id: &str, attempt_id: &str) -> Result<(), HilEvidenceError> {
        let key = Self::key(session_id, attempt_id);
        let inserted = match self.consumed.lock() {
            Ok(mut guard) => guard.insert(key),
            Err(poisoned) => poisoned.into_inner().insert(key),
        };
        if inserted {
            Ok(())
        } else {
            Err(HilEvidenceError::Replayed {
                attempt_id: attempt_id.to_string(),
            })
        }
    }
}

/// Wynik zapytania do dostawcy dowodu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HilEvidenceProviderOutcome {
    /// Dostawca zwrócił dokument dowodowy.
    Evidence(Box<HilIndependentEvidence>),
    /// Dostawca nie ma dowodu (brak niezależnego dowodu).
    Missing,
    /// Dostawca zwrócił błąd.
    Error(String),
    /// Dostawca przekroczył limit czasu.
    Timeout,
    /// Operacja została anulowana.
    Cancelled,
    /// Odmowa z powodu niezgodności zakresu/wiązania.
    Rejected(HilEvidenceError),
}

/// Kontrakt dostawcy niezależnego dowodu.
///
/// Trait jest zdefiniowany w kodzie produkcyjnym, ale **żadna produkcyjna implementacja nie
/// istnieje** w tej bramce. Testowe implementacje żyją wyłącznie w zakresie testowym
/// (`#[cfg(test)]` w bibliotece oraz `tests/`/`tests/support/`).
pub trait HilEvidenceProvider {
    /// Zwraca dowód dla zadanego planu i oświadczenia.
    fn request_evidence(
        &self,
        plan: &HilTestBenchManifest,
        attestation: &HilAttestationDocument,
    ) -> HilEvidenceProviderOutcome;
}

/// Bezpieczny, fail-closed adapter dostawcy: brak dowodu, błąd, timeout i anulowanie
/// mapowane są na kontrolowaną odmowę.
pub fn request_evidence_or_deny<P: HilEvidenceProvider>(
    provider: Option<&P>,
    plan: &HilTestBenchManifest,
    attestation: &HilAttestationDocument,
) -> Result<HilIndependentEvidence, HilEvidenceError> {
    let Some(provider) = provider else {
        return Err(HilEvidenceError::IssuanceUnavailableInProduction);
    };
    match provider.request_evidence(plan, attestation) {
        HilEvidenceProviderOutcome::Evidence(evidence) => Ok(*evidence),
        HilEvidenceProviderOutcome::Missing => Err(HilEvidenceError::Missing),
        HilEvidenceProviderOutcome::Error(reason) => Err(HilEvidenceError::ProviderError(reason)),
        HilEvidenceProviderOutcome::Timeout => Err(HilEvidenceError::ProviderTimeout),
        HilEvidenceProviderOutcome::Cancelled => Err(HilEvidenceError::Cancelled),
        HilEvidenceProviderOutcome::Rejected(err) => Err(err),
    }
}

#[cfg(test)]
pub(crate) mod test_evidence_support {
    //! TEST-ONLY: konstruktory dowodu i helpery dla testów jednostkowych.
    //!
    //! Ten moduł jest kompilowany wyłącznie pod `#[cfg(test)]`. Nie istnieje w kodzie
    //! produkcyjnym ani w artefaktach wydania.

    use super::*;
    use crate::cat::hil::manifest::{
        HIL_PLAN_SCHEMA_VERSION, HilDummyLoadSpec, HilMatrixEntry, HilRfSafetyConfig,
        HilRfTermination, HilTransportConfig,
    };

    /// Identyfikator testowego podmiotu wydającego.
    pub const TEST_ISSUER: &str = HIL_SUPPORTED_EVIDENCE_ISSUERS[0];

    /// Buduje kompletny, poprawny dowód testowy dla planu i oświadczenia.
    pub fn issue_test_evidence(
        plan: &HilTestBenchManifest,
        attestation: &HilAttestationDocument,
    ) -> HilIndependentEvidence {
        let mut measured_facts = BTreeMap::new();
        for fact in HIL_REQUIRED_MEASURED_FACTS {
            measured_facts.insert(fact.to_string(), true);
        }
        HilIndependentEvidence {
            schema_version: HIL_AUTHORIZATION_SCHEMA_VERSION,
            evidence_id: format!("TEST-EVIDENCE-{}", plan.test_case_id),
            issuer: TEST_ISSUER.to_string(),
            issued_at: "2026-10-18T09:00:00Z".to_string(),
            expires_at: Some("2026-10-18T23:59:59Z".to_string()),
            scope: HilEvidenceScope::HardwarePtt,
            plan_sha256: plan.plan_sha256(),
            attestation_sha256: attestation.attestation_sha256(),
            test_case_id: plan.test_case_id.clone(),
            rig_id: plan.matrix.rig_id.as_str().to_string(),
            session_id: plan.session_id.clone(),
            attempt_id: plan.attempt_id.clone(),
            measured_facts,
            integrity_ok: true,
            seal: EvidenceSeal(()),
        }
    }

    /// Buduje poprawne oświadczenie testowe dla planu.
    pub fn issue_test_attestation(plan: &HilTestBenchManifest) -> HilAttestationDocument {
        HilAttestationDocument {
            schema_version: HIL_ATTESTATION_SCHEMA_VERSION,
            bench_id: plan.bench_id.clone(),
            session_id: plan.session_id.clone(),
            test_case_id: plan.test_case_id.clone(),
            rig_id: plan.matrix.rig_id.as_str().to_string(),
            plan_sha256: plan.plan_sha256(),
            statements: HilOperatorAttestationStatements {
                dummy_load_physically_connected: true,
                rf_perimeter_secured: true,
                ptt_consent_granted: true,
                attestation_timestamp: "2026-10-18T09:30:00Z".to_string(),
                description: "Software-only F-15 verification bench".to_string(),
                antenna_disconnected: true,
                pa_disconnected_or_locked: true,
                tuner_bypassed: true,
                physical_stop_available: true,
                abort_procedure_documented: true,
                operator_acknowledged: true,
                bench_readiness_confirmed: true,
                operator_present: true,
                supervised_trial_confirmed: true,
                rules_compliance_attested: true,
                operator_privileges_attested: true,
                allowed_band_attested: true,
                allowed_mode_attested: true,
                no_unauthorized_emission_attested: true,
                personnel_know_stop_procedure: true,
            },
        }
    }

    /// Buduje minimalny poprawny plan testowy (bez żadnych oświadczeń o stanie sprzętu).
    pub fn test_plan() -> HilTestBenchManifest {
        HilTestBenchManifest {
            schema_version: HIL_PLAN_SCHEMA_VERSION,
            bench_id: "LAB-BENCH-SP6INA-01".to_string(),
            session_id: "SESSION-F15-01".to_string(),
            test_case_id: "hil_hardware_ptt_01".to_string(),
            attempt_id: "ATTEMPT-F15-0001".to_string(),
            created_at: "2026-10-18T09:30:00Z".to_string(),
            expires_at: Some("2026-10-18T23:59:59Z".to_string()),
            tested_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
            expected_commit: None,
            working_tree_clean: None,
            software_tests_passed: None,
            operator_id: "SP6INA".to_string(),
            operator_callsign: "SP6INA".to_string(),
            operator_name: "Mariusz Woźniak".to_string(),
            hil_enabled: true,
            allow_tx: true,
            matrix: HilMatrixEntry {
                backend_kind: crate::cat::backend::CatBackendKind::IcomCiV,
                rig_id: crate::cat::routing::RigId::new("RIG-F15-01").unwrap(),
                model_name: "IC-7300".to_string(),
                transport: HilTransportConfig::Serial {
                    port_name: "COM3".to_string(),
                    baud_rate: 115_200,
                    data_bits: 8,
                    parity: "none".to_string(),
                    stop_bits: 1,
                    flow_control: "none".to_string(),
                },
                firmware_version: "v1.42".to_string(),
            },
            rf_safety: HilRfSafetyConfig {
                termination: HilRfTermination::DummyLoad(HilDummyLoadSpec {
                    power_rating_watts: 100.0,
                    max_swr: 1.15,
                    serial_number: "DL-50W-001".to_string(),
                    impedance_ohms: 50.0,
                }),
                max_test_power_watts: 10.0,
                frequency_range_hz: (14_000_000, 14_350_000),
                interlock_engaged: true,
                max_test_duration_ms: 500,
                smartlink_disabled: true,
                unattended_remote_tx_disabled: true,
                remote_endpoint_is_local_or_controlled: true,
                operator_has_immediate_stop_control: true,
                out_of_band_data_plane_disabled: true,
            },
            operator_attestation: None,
            instrument_verification: None,
        }
    }

    /// Buduje poprawną decyzję autoryzacyjną dla testów ścieżki PTT.
    pub fn authorized_decision(
        plan: &HilTestBenchManifest,
        attestation: &HilAttestationDocument,
        evidence: &HilIndependentEvidence,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan,
                attestation: Some(attestation),
                evidence: Some(evidence),
            },
            evaluated_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_evidence_support::*;
    use super::*;

    fn reference_time() -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-10-18T12:00:00Z")
            .expect("reference time")
            .with_timezone(&Utc)
    }

    #[test]
    fn f15_plan_alone_is_denied() {
        let plan = test_plan();
        let decision = HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan: &plan,
                attestation: None,
                evidence: None,
            },
            reference_time(),
        );
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::OperatorAttestationMissing
        );
        assert!(!decision.authorizes_hardware_ptt());
    }

    #[test]
    fn f15_attestation_alone_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let decision = HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan: &plan,
                attestation: Some(&attestation),
                evidence: None,
            },
            reference_time(),
        );
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceMissing);
        assert!(!decision.authorizes_hardware_ptt());
        assert_eq!(
            decision.audit().missing.as_deref(),
            Some("independent_evidence")
        );
    }

    #[test]
    fn f15_plan_digest_and_attestation_digest_are_bound() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        assert_eq!(evidence.plan_sha256, plan.plan_sha256());
        assert_eq!(
            evidence.attestation_sha256,
            attestation.attestation_sha256()
        );
    }

    #[test]
    fn f15_expired_evidence_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.expires_at = Some("2026-10-18T10:00:00Z".to_string());
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceExpired);
        assert!(!decision.authorizes_hardware_ptt());
    }

    #[test]
    fn f15_not_yet_valid_evidence_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.issued_at = "2026-10-19T00:00:00Z".to_string();
        evidence.expires_at = None;
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::EvidenceNotYetValid
        );
    }

    #[test]
    fn f15_wrong_scope_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.scope = HilEvidenceScope::DryRunNoPtt;
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::EvidenceScopeMismatch
        );
    }

    #[test]
    fn f15_missing_measured_fact_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.measured_facts.remove("antenna.disconnected");
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceInvalid);
        assert_eq!(
            decision.audit().missing.as_deref(),
            Some("antenna.disconnected")
        );
    }

    #[test]
    fn f15_false_measured_fact_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence
            .measured_facts
            .insert("amplifier.disabled_or_locked".to_string(), false);
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceInvalid);
    }

    #[test]
    fn f15_wrong_plan_digest_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.plan_sha256 = "0".repeat(64);
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::EvidenceScopeMismatch
        );
    }

    #[test]
    fn f15_wrong_attestation_digest_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.attestation_sha256 = "0".repeat(64);
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::EvidenceScopeMismatch
        );
    }

    #[test]
    fn f15_unsupported_issuer_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.issuer = "manifest-self".to_string();
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(
            decision.outcome(),
            HilAuthorizationOutcome::IndependentApprovalMissing
        );
    }

    #[test]
    fn f15_evidence_for_another_plan_is_denied() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);

        let mut other_plan = test_plan();
        other_plan.test_case_id = "hil_hardware_read_01".to_string();
        let other_attestation = issue_test_attestation(&other_plan);
        let decision = HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan: &other_plan,
                attestation: Some(&other_attestation),
                evidence: Some(&evidence),
            },
            reference_time(),
        );
        assert!(decision.outcome().is_denial());
    }

    #[test]
    fn f15_dry_run_never_authorizes_ptt() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let decision =
            HilHardwareExecutionAuthorizer::dry_run_at(&plan, Some(&attestation), reference_time());
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::DryRunOnly);
        assert!(!decision.authorizes_hardware_ptt());
    }

    #[test]
    fn f15_provider_absent_is_denied() {
        struct NoEvidenceProvider;
        impl HilEvidenceProvider for NoEvidenceProvider {
            fn request_evidence(
                &self,
                _plan: &HilTestBenchManifest,
                _attestation: &HilAttestationDocument,
            ) -> HilEvidenceProviderOutcome {
                HilEvidenceProviderOutcome::Missing
            }
        }
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let denied = request_evidence_or_deny::<NoEvidenceProvider>(None, &plan, &attestation);
        assert_eq!(
            denied.unwrap_err(),
            HilEvidenceError::IssuanceUnavailableInProduction
        );
        let missing = request_evidence_or_deny(Some(&NoEvidenceProvider), &plan, &attestation);
        assert_eq!(missing.unwrap_err(), HilEvidenceError::Missing);
    }

    #[test]
    fn f15_replay_is_denied() {
        let registry = HilAttemptRegistry::new();
        assert!(
            registry
                .consume("SESSION-F15-01", "ATTEMPT-F15-0001")
                .is_ok()
        );
        assert!(matches!(
            registry.consume("SESSION-F15-01", "ATTEMPT-F15-0001"),
            Err(HilEvidenceError::Replayed { .. })
        ));
        assert!(registry.is_consumed("SESSION-F15-01", "ATTEMPT-F15-0001"));
        assert!(!registry.is_consumed("SESSION-F15-02", "ATTEMPT-F15-0001"));
    }

    #[test]
    fn f15_default_decision_is_not_evaluated_and_denied() {
        let decision = HilAuthorizationDecision::default();
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::NotEvaluated);
        assert!(!decision.authorizes_hardware_ptt());
        assert!(decision.outcome().is_denial());
    }

    #[test]
    fn f15_sha256_hex_is_lowercase_64_chars() {
        let digest = sha256_hex(b"SPLogbook");
        assert_eq!(digest.len(), 64);
        assert!(is_sha256_hex(&digest));
        assert_eq!(digest_prefix(&digest).len(), 12);
    }

    #[test]
    fn f15_evidence_serialisation_never_persists_the_seal() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        let json = serde_json::to_value(&evidence).expect("serialise evidence");
        assert!(
            json.get("seal").is_none(),
            "the evidence constructor seal must never be serialised"
        );
    }

    // =====================================================================
    // DETERMINISTIC STRESS MATRIX (no sleep, no wall clock, no I/O)
    // =====================================================================

    fn denied(plan: &HilTestBenchManifest, evidence: Option<&HilIndependentEvidence>) -> bool {
        let attestation = issue_test_attestation(plan);
        let decision = HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan,
                attestation: Some(&attestation),
                evidence,
            },
            reference_time(),
        );
        !decision.authorizes_hardware_ptt()
    }

    #[test]
    fn f15_stress_missing_fields_2000() {
        for i in 0..2000usize {
            let mut plan = test_plan();
            match i % 5 {
                0 => plan.test_case_id.clear(),
                1 => plan.attempt_id.clear(),
                2 => plan.session_id.clear(),
                3 => plan.schema_version = 0,
                _ => plan.bench_id.clear(),
            }
            assert!(denied(&plan, None), "iteration {i}");
        }
    }

    #[test]
    fn f15_stress_self_certification_2000() {
        for i in 0..2000usize {
            let mut plan = test_plan();
            plan.working_tree_clean = Some(true);
            plan.software_tests_passed = Some(true);
            plan.operator_attestation = Some(crate::cat::hil::manifest::HilOperatorAttestation {
                dummy_load_physically_connected: true,
                rf_perimeter_secured: true,
                ptt_consent_granted: true,
                attestation_timestamp: format!("2026-10-18T09:30:{:02}Z", i % 60),
                description: "self-certified bench".to_string(),
                antenna_disconnected: true,
                pa_disconnected_or_locked: true,
                tuner_bypassed: true,
                physical_stop_available: true,
                abort_procedure_documented: true,
                operator_acknowledged: true,
                bench_readiness_confirmed: true,
                operator_present: true,
                supervised_trial_confirmed: true,
                rules_compliance_attested: true,
                operator_privileges_attested: true,
                allowed_band_attested: true,
                allowed_mode_attested: true,
                no_unauthorized_emission_attested: true,
                personnel_know_stop_procedure: true,
            });
            assert!(denied(&plan, None), "iteration {i}");
        }
    }

    #[test]
    fn f15_stress_all_booleans_true_2000() {
        for _ in 0..2000usize {
            let plan = test_plan();
            assert!(plan.hil_enabled && plan.allow_tx);
            assert!(plan.rf_safety.interlock_engaged);
            assert!(plan.rf_safety.smartlink_disabled);
            assert!(plan.rf_safety.unattended_remote_tx_disabled);
            assert!(plan.rf_safety.remote_endpoint_is_local_or_controlled);
            assert!(plan.rf_safety.operator_has_immediate_stop_control);
            assert!(plan.rf_safety.out_of_band_data_plane_disabled);
            assert!(denied(&plan, None));
        }
    }

    #[test]
    fn f15_stress_missing_evidence_2000() {
        for _ in 0..2000usize {
            let plan = test_plan();
            assert!(denied(&plan, None));
        }
    }

    #[test]
    fn f15_stress_malformed_evidence_1000() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        for i in 0..1000usize {
            let mut evidence = issue_test_evidence(&plan, &attestation);
            match i % 4 {
                0 => evidence.evidence_id.clear(),
                1 => evidence.integrity_ok = false,
                2 => evidence.plan_sha256 = "XYZ".to_string(),
                _ => evidence.schema_version = 99,
            }
            assert!(denied(&plan, Some(&evidence)), "iteration {i}");
        }
    }

    #[test]
    fn f15_stress_expired_evidence_1000() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        for i in 0..1000usize {
            let mut evidence = issue_test_evidence(&plan, &attestation);
            evidence.expires_at = Some(format!("2026-10-18T10:{:02}:00Z", i % 60));
            assert!(denied(&plan, Some(&evidence)), "iteration {i}");
        }
    }

    #[test]
    fn f15_stress_wrong_scope_1000() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        for _ in 0..1000usize {
            let mut evidence = issue_test_evidence(&plan, &attestation);
            evidence.scope = HilEvidenceScope::DryRunNoPtt;
            assert!(denied(&plan, Some(&evidence)));
        }
    }

    #[test]
    fn f15_stress_wrong_digest_1000() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        for i in 0..1000usize {
            let mut evidence = issue_test_evidence(&plan, &attestation);
            evidence.plan_sha256 = format!("{i:064x}");
            assert!(denied(&plan, Some(&evidence)), "iteration {i}");
        }
    }

    #[test]
    fn f15_stress_replay_1000() {
        let plan = test_plan();
        for i in 0..1000usize {
            let registry = HilAttemptRegistry::new();
            assert!(registry.consume(&plan.session_id, &plan.attempt_id).is_ok());
            assert!(matches!(
                registry.consume(&plan.session_id, &plan.attempt_id),
                Err(HilEvidenceError::Replayed { .. })
            ));
            assert!(registry.is_consumed(&plan.session_id, &plan.attempt_id));
            let _ = i;
        }
    }

    #[test]
    fn f15_stress_provider_failures_1000_each() {
        struct Failing(HilEvidenceProviderOutcome);
        impl HilEvidenceProvider for Failing {
            fn request_evidence(
                &self,
                _plan: &HilTestBenchManifest,
                _attestation: &HilAttestationDocument,
            ) -> HilEvidenceProviderOutcome {
                self.0.clone()
            }
        }
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        for i in 0..1000usize {
            // Provider timeout
            let timeout = Failing(HilEvidenceProviderOutcome::Timeout);
            assert_eq!(
                request_evidence_or_deny(Some(&timeout), &plan, &attestation).unwrap_err(),
                HilEvidenceError::ProviderTimeout
            );
            // Provider error
            let error = Failing(HilEvidenceProviderOutcome::Error(format!("failure {i}")));
            assert!(matches!(
                request_evidence_or_deny(Some(&error), &plan, &attestation),
                Err(HilEvidenceError::ProviderError(_))
            ));
            // Cancellation
            let cancelled = Failing(HilEvidenceProviderOutcome::Cancelled);
            assert_eq!(
                request_evidence_or_deny(Some(&cancelled), &plan, &attestation).unwrap_err(),
                HilEvidenceError::Cancelled
            );
        }
    }

    #[test]
    fn f15_stress_migration_1000() {
        for _ in 0..1000usize {
            let mut plan = test_plan();
            plan.schema_version = 0;
            assert!(plan.requires_migration());
            assert!(plan.validate_plan().is_err());
            assert!(denied(&plan, None));
        }
    }

    #[test]
    fn f15_stress_test_only_provider_500() {
        for _ in 0..500usize {
            let plan = test_plan();
            let attestation = issue_test_attestation(&plan);
            let evidence = issue_test_evidence(&plan, &attestation);
            assert!(HilHardwareExecutionAuthorizer::evidence_matches_plan(
                &plan,
                &attestation,
                &evidence
            ));
            let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
            assert!(decision.authorizes_hardware_ptt());
        }
    }
}
