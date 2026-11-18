// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł Hardware-In-The-Loop (HIL) dla podsystemu Native CAT.
//!
//! Zapewnia:
//! - Struktury planu stanowiska laboratoryjnego ([`HilTestBenchManifest`]) — wyłącznie plan,
//! - Jawny model niezależnego dowodu ([`HilIndependentEvidence`]) i decyzji autoryzacyjnej
//!   ([`HilAuthorizationDecision`], F-15),
//! - Silnie typowane dowody gotowości ([`HilReadinessEvidenceKind`]),
//! - Deterministyczny walidator konfiguracji HIL-READY ([`HilReadinessValidator`]),
//! - Bezpieczne sesje pomiarowe ([`HilHardwareSession`]),
//! - Izolację faz NATIVE-7A (offline) oraz NATIVE-7B (stanowisko fizyczne).
//!
//! **Zasada F-15:** manifest testu nie jest dowodem gotowości i nie może sam siebie
//! autoryzować. Gotowość sprzętowa wymaga niezależnego dokumentu dowodowego wydanego przez
//! osobny podmiot; w kodzie produkcyjnym nie istnieje żaden dostawca dowodu, więc wynik jest
//! zawsze fail-closed (`EvidenceMissing` → brak PTT).

pub mod authorization;
pub mod error;
pub mod evidence;
pub mod manifest;
pub mod session;
pub mod validator;

pub use authorization::{
    HIL_ATTESTATION_SCHEMA_VERSION, HIL_AUTHORIZATION_SCHEMA_VERSION, HIL_PLAN_DIGEST_DOMAIN,
    HIL_REQUIRED_MEASURED_FACTS, HIL_SUPPORTED_EVIDENCE_ISSUERS, HilAttemptRegistry,
    HilAttestationDocument, HilAuthorizationAudit, HilAuthorizationDecision,
    HilAuthorizationInputs, HilAuthorizationOutcome, HilEvidenceError, HilEvidenceProvider,
    HilEvidenceProviderOutcome, HilEvidenceScope, HilHardwareExecutionAuthorizer,
    HilIndependentEvidence, HilOperatorAttestationStatements, digest_prefix, is_sha256_hex,
    is_supported_issuer, request_evidence_or_deny, sha256_hex,
};
pub use error::HilError;
pub use evidence::{
    CANONICAL_HIL_READY_DESCRIPTIONS, CANONICAL_HIL_READY_IDS, HilAbortProcedure, HilCommandResult,
    HilEvidenceOrigin, HilExecutionContext, HilExecutionStage, HilExtendedRequirement,
    HilImplementationCoverage, HilReadinessCheck, HilReadinessCondition, HilReadinessEvidenceKind,
    HilReadinessReport, HilReadinessStatus, HilRequirementAssessment, HilSoftwareBaselineEvidence,
};
pub use manifest::{
    HIL_MAX_TEST_DURATION_MS, HIL_MIN_TEST_DURATION_MS, HIL_PLAN_SCHEMA_VERSION, HilDummyLoadSpec,
    HilInstrumentMeasurement, HilMatrixEntry, HilOperatorAttestation, HilPlanValidationError,
    HilRfSafetyConfig, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
pub use session::{
    HilDispatcherCase, HilDispatcherReport, HilHardwareSession, HilProbeResult, HilPttConsentToken,
    HilPttExecutionResult, HilReleaseOutcome, HilSessionState, HilTxGuard,
    default_hil_evaluation_time, run_isolated_hardware_dispatcher,
    run_isolated_hardware_dispatcher_at, run_isolated_hardware_test,
};
pub use validator::{HilProfileValidationError, HilReadinessValidator};

/// Parses an RFC 3339 instant (`YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM)`) to a UTC
/// [`jiff::Timestamp`].
///
/// The RFC 3339 shape is checked before jiff parses the value: jiff alone also accepts
/// ISO 8601 / RFC 9557 forms that RFC 3339 does not allow (basic format, missing seconds,
/// bracketed annotations). HIL authorization must not widen what it accepts.
pub fn parse_rfc3339(value: &str) -> Result<jiff::Timestamp, String> {
    if !is_rfc3339_shape(value.as_bytes()) {
        return Err(format!("`{value}` is not an RFC 3339 date-time"));
    }
    value.parse::<jiff::Timestamp>().map_err(|e| e.to_string())
}

fn is_rfc3339_shape(b: &[u8]) -> bool {
    let digits = |from: usize, to: usize| {
        b.get(from..to)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    };
    let at = |i: usize, c: &[u8]| b.get(i).is_some_and(|x| c.contains(x));
    if !(digits(0, 4)
        && at(4, b"-")
        && digits(5, 7)
        && at(7, b"-")
        && digits(8, 10)
        && at(10, b"Tt ")
        && digits(11, 13)
        && at(13, b":")
        && digits(14, 16)
        && at(16, b":")
        && digits(17, 19))
    {
        return false;
    }
    let mut i = 19;
    if at(i, b".") {
        let start = i + 1;
        i = start;
        while at(i, b"0123456789") {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    match b.get(i..) {
        Some([b'Z' | b'z']) => true,
        Some([b'+' | b'-', h1, h2, b':', m1, m2]) => {
            [h1, h2, m1, m2].iter().all(|d| d.is_ascii_digit())
        }
        _ => false,
    }
}

#[cfg(test)]
mod rfc3339_tests {
    use super::parse_rfc3339;

    #[test]
    fn accepts_rfc3339_instants_in_any_offset() {
        let utc = parse_rfc3339("2026-10-18T12:00:00Z").unwrap();
        for same in [
            "2026-10-18T12:00:00z",
            "2026-10-18t12:00:00Z",
            "2026-10-18 12:00:00Z",
            "2026-10-18T12:00:00+00:00",
            "2026-10-18T14:00:00+02:00",
            "2026-10-18T07:00:00-05:00",
            "2026-10-18T12:00:00.000Z",
        ] {
            assert_eq!(parse_rfc3339(same), Ok(utc), "{same}");
        }
        assert_eq!(
            parse_rfc3339("2026-10-18T12:00:00.123456789Z")
                .unwrap()
                .subsec_nanosecond(),
            123_456_789
        );
    }

    #[test]
    fn rejects_forms_outside_rfc3339() {
        for bad in [
            "",
            "2026-10-18",
            "2026-10-18T12:00Z",
            "2026-10-18T12:00:00",
            "20261018T120000Z",
            "2026-10-18T12:00:00Z[UTC]",
            "2026-10-18T12:00:00+0000",
            "2026-10-18T12:00:00+00",
            "2026-10-18T12:00:00.Z",
            "2026-10-18T12:00:00Z ",
            " 2026-10-18T12:00:00Z",
            "2026-13-18T12:00:00Z",
            "2026-02-30T12:00:00Z",
            "2026-10-18T24:00:00Z",
        ] {
            assert!(parse_rfc3339(bad).is_err(), "{bad:?} must be rejected");
        }
    }
}
