// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! CAT-018 — SOFTWARE-ONLY Kenwood PTT protocol conformance.
//!
//! Every assertion is grounded in an official JVC Kenwood document that was opened and read in
//! this gate (see `.local-work/reports/CAT_018_019_PRE_IMPLEMENTATION.md`):
//!
//! * **TS-590S / TS-590SG** — PC CONTROL COMMAND Reference **B5A-0316-20/01** (2019-01-30),
//!   `TX` p. 30, `RX` p. 25, `IF` p. 15, `ID` p. 14, `SM` p. 27, error messages p. 3.
//! * **TS-890S** — PC CONTROL COMMAND Reference **B5A-2944-00/01** (2019-01-30),
//!   `TX` p. 67, `RX` p. 59, `ID` p. 36, `SM` p. 62, error messages p. 3
//!   (**no `IF` command**).
//! * **TS-990S** — PC CONTROL COMMAND Reference **B5A-3170-00** (2019-01-30),
//!   `TX` p. 61, `RX` p. 51, `ID` p. 32, `SM` p. 57, error messages p. 3
//!   (**no `IF` command**).
//!
//! The tests drive the **real** production `KenwoodBackend` / `KenwoodCodec` through a test-only
//! in-memory wire double (`tests/support/kenwood_wire.rs`). No serial port, no USB device, no COM
//! port, no network, no transceiver, no PTT and no RF is involved.

use std::time::Duration;

use splogbook::cat::backend::RigBackend;
use splogbook::cat::native::kenwood::KenwoodBackend;
use splogbook::cat::native::kenwood::codec::KenwoodCat;
use splogbook::cat::native::kenwood::models::{
    KENWOOD_DOC_VERIFIED, KENWOOD_PTT_CONTRACTS, KenwoodPttContract,
};
use splogbook::cat::native::profile::RigModelProfile;
use splogbook::cat::native::registry::{
    KENWOOD_TS590S, KENWOOD_TS590SG, KENWOOD_TS890S, KENWOOD_TS990S,
};
use splogbook::cat::rig::{RigCommand, RigError};

#[path = "support/kenwood_wire.rs"]
mod kenwood_wire;

use kenwood_wire::{KenwoodWire, KenwoodWireConfig, PttAnswer, WireErrorFrame};

// ============================================================================
// Shared helpers
// ============================================================================

/// One supported Kenwood profile together with its documented wire behaviour.
struct Model {
    label: &'static str,
    profile: RigModelProfile,
    config: KenwoodWireConfig,
    /// `true` when the official reference defines the `IF` command.
    documents_if: bool,
    /// Documented S-meter read form.
    smeter_read: &'static str,
}

fn shipped_models() -> Vec<Model> {
    vec![
        Model {
            label: "TS-590S",
            profile: KENWOOD_TS590S.clone(),
            config: KenwoodWireConfig::ts590s(),
            documents_if: true,
            smeter_read: "SM0;",
        },
        Model {
            label: "TS-590SG",
            profile: KENWOOD_TS590SG.clone(),
            config: KenwoodWireConfig::ts590sg(),
            documents_if: true,
            smeter_read: "SM0;",
        },
        Model {
            label: "TS-890S",
            profile: KENWOOD_TS890S.clone(),
            config: KenwoodWireConfig::ts890s(),
            documents_if: false,
            smeter_read: "SM;",
        },
        Model {
            label: "TS-990S",
            profile: KENWOOD_TS990S.clone(),
            config: KenwoodWireConfig::ts990s(),
            documents_if: false,
            smeter_read: "SM0;",
        },
    ]
}

async fn connect(model: &Model, config: KenwoodWireConfig) -> (KenwoodBackend, KenwoodWire) {
    kenwood_wire::connect_backend(model.profile.clone(), config).await
}

/// The PTT-related frames the production client wrote, in order.
fn ptt_frames(wire: &KenwoodWire) -> Vec<String> {
    wire.frames()
        .into_iter()
        .filter(|frame| frame == "TX;" || frame == "RX;" || frame.starts_with("TX"))
        .collect()
}

// ============================================================================
// PHASE 8 — PROGRAMMATIC REPRODUCTION OF CAT-018
// ============================================================================

/// The documented Answer of the `TX` command is `TX P1;` and every documented `P1`
/// (`0` = SEND, `1` = DATA SEND, `2` = TX Tune) means the transceiver is transmitting.
/// `TX0;` therefore reports **transmit**, never receive (`TX` p. 30 / p. 67 / p. 61).
#[test]
fn cat018_tx0_documented_answer_is_transmit() {
    assert_eq!(
        KenwoodCat::parse_ptt_resp("TX0;"),
        Some(true),
        "documented TX P1; with P1=0 (SEND) is a transmit answer"
    );
    assert_eq!(KenwoodCat::parse_ptt_resp("TX1;"), Some(true));
    assert_eq!(KenwoodCat::parse_ptt_resp("TX2;"), Some(true));
    assert_eq!(KenwoodCat::parse_ptt_resp("RX;"), Some(false));
}

/// Consequence of the same mismatch through the real backend: the documented Auto-Information
/// answer `TX0;` arrives while the transceiver transmits. The backend must keep the TX state and
/// the disconnect fail-safe must send the documented `RX;`.
#[tokio::test(start_paused = true)]
async fn cat018_tx0_auto_information_keeps_disconnect_failsafe() {
    let mut cfg = KenwoodWireConfig::ts590s();
    cfg.push_unsolicited_tx0 = true;
    let (mut backend, wire) = kenwood_wire::connect_backend(KENWOOD_TS590S.clone(), cfg).await;

    let _ = backend.poll().await;

    assert!(
        backend.current_state().ptt,
        "the documented TX0; answer means the transceiver is transmitting"
    );

    backend.disconnect().await;
    assert!(
        wire.wait_for("RX;", Duration::from_secs(2)).await,
        "a transceiver left in the documented TX state must receive RX; on disconnect"
    );
    wire.shutdown().await;
}

/// `IF` exists only in B5A-0316-20/01 (TS-590S/SG p. 15). Sending `IF;` to a TS-890S or TS-990S
/// is an undocumented command and must never happen; without `IF` the documented confirmation
/// channel is the Auto-Information Answer, so a silent peer leaves the state unconfirmed instead
/// of failing after the transmitter was keyed.
#[tokio::test(start_paused = true)]
async fn cat018_if_is_never_sent_to_models_without_the_documented_command() {
    let mut cfg = KenwoodWireConfig::ts890s();
    cfg.tx_answer = PttAnswer::Silent;
    let (mut backend, wire) = kenwood_wire::connect_backend(KENWOOD_TS890S.clone(), cfg).await;

    let outcome = backend.execute(&RigCommand::SetPtt(true)).await;

    assert!(
        !wire.sent("IF;"),
        "TS-890S (B5A-2944-00/01) documents no IF command"
    );
    assert!(
        outcome.is_ok(),
        "the documented TX; write succeeded; an unconfirmed state is not a write failure"
    );
    assert!(
        !backend.current_state().ptt,
        "write success must never be reported as a confirmed physical TX"
    );
    backend.disconnect().await;
    wire.shutdown().await;
}

/// The documented error messages (`?;`, `E;`, `O;`, p. 3) are explicit rejections: they must fail
/// closed immediately and must never be retried.
#[tokio::test(start_paused = true)]
async fn cat018_documented_error_frame_fails_closed() {
    let mut cfg = KenwoodWireConfig::ts590s();
    cfg.tx_answer = PttAnswer::Error(WireErrorFrame::Syntax);
    let (mut backend, wire) = kenwood_wire::connect_backend(KENWOOD_TS590S.clone(), cfg).await;

    let outcome = backend.execute(&RigCommand::SetPtt(true)).await;

    assert!(outcome.is_err(), "a ?; rejection is an error");
    assert!(!backend.current_state().ptt);
    assert_eq!(
        wire.count("TX;"),
        1,
        "PTT ON must never be retried after a documented rejection"
    );
    backend.disconnect().await;
    wire.shutdown().await;
}

// ============================================================================
// PHASE 13 — PER-MODEL CONTRACT
// ============================================================================

/// Every supported Kenwood profile carries its own official document, and no brand-wide
/// assumption replaces a model-specific one.
#[test]
fn kenwood_supported_profiles_have_model_specific_documented_contracts() {
    let models = shipped_models();
    assert_eq!(models.len(), 4, "four shipped Kenwood profiles");

    let mut ids = Vec::new();
    for model in &models {
        let contract = KenwoodPttContract::for_model(model.profile.model_id)
            .unwrap_or_else(|| panic!("{} has no documented PTT contract", model.label));
        assert_eq!(contract.model_id, model.profile.model_id);
        assert_eq!(contract.documentation_status, KENWOOD_DOC_VERIFIED);
        assert!(
            contract.document.contains("PC CONTROL COMMAND"),
            "{} must cite the official PC CONTROL COMMAND reference",
            model.label
        );
        assert!(
            contract.document.contains("2019-01-30"),
            "{} must cite the document revision date",
            model.label
        );
        assert!(!contract.ptt_on.is_empty() && !contract.ptt_off.is_empty());
        assert_ne!(contract.ptt_on, contract.ptt_off);
        assert_eq!(
            contract.documents_status_read(),
            model.documents_if,
            "{}: IF availability must match the official reference",
            model.label
        );
        assert_eq!(contract.smeter_read, Some(model.smeter_read));
        ids.push(contract.model_id);
    }
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 4, "each model has exactly one contract");

    // A model without an official document has no contract at all (fail-closed).
    assert!(KenwoodPttContract::for_model("TS-2000").is_none());
    assert!(KenwoodPttContract::for_model("").is_none());
    // The user-declared compatible profile is explicit and carries no model document.
    let custom = KenwoodPttContract::for_model("CUSTOM-KENWOOD").expect("custom contract");
    assert!(custom.status_read.is_none());
    assert!(custom.smeter_read.is_none());
    assert_ne!(custom.documentation_status, KENWOOD_DOC_VERIFIED);
    assert_eq!(KENWOOD_PTT_CONTRACTS.len(), 5);
}

/// The documented ON/OFF commands are identical to what the production codec produces, per model.
#[test]
fn kenwood_documented_ptt_commands_match_the_production_codec_per_model() {
    for model in shipped_models() {
        let contract = KenwoodPttContract::for_model(model.profile.model_id).expect("contract");
        assert_eq!(
            contract.ptt_on,
            KenwoodCat::set_ptt_cmd(true),
            "{}",
            model.label
        );
        assert_eq!(
            contract.ptt_off,
            KenwoodCat::set_ptt_cmd(false),
            "{}",
            model.label
        );
        assert_eq!(contract.ptt_on, "TX;", "{}: P1 omitted = SEND", model.label);
        assert_eq!(
            contract.ptt_off, "RX;",
            "{}: documented receive",
            model.label
        );
    }
}

/// PTT ON writes exactly the documented `TX;` bytes — terminator included, nothing else, once.
#[tokio::test(start_paused = true)]
async fn kenwood_ptt_on_writes_exactly_the_documented_bytes_per_model() {
    for model in shipped_models() {
        let (mut backend, wire) = connect(&model, model.config.clone()).await;
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .unwrap_or_else(|e| panic!("{}: documented TX; must be accepted: {e}", model.label));

        assert_eq!(
            ptt_frames(&wire),
            vec!["TX;".to_string()],
            "{}: exactly one documented TX; frame",
            model.label
        );
        assert!(
            backend.current_state().ptt,
            "{}: documented answer confirms TX",
            model.label
        );
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// PTT OFF writes exactly the documented `RX;` bytes and clears the confirmed TX state.
#[tokio::test(start_paused = true)]
async fn kenwood_ptt_off_writes_exactly_the_documented_bytes_per_model() {
    for model in shipped_models() {
        let (mut backend, wire) = connect(&model, model.config.clone()).await;
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("TX");
        backend
            .execute(&RigCommand::SetPtt(false))
            .await
            .expect("RX");

        assert_eq!(
            ptt_frames(&wire),
            vec!["TX;".to_string(), "RX;".to_string()],
            "{}: documented ON then OFF frames",
            model.label
        );
        assert!(
            !backend.current_state().ptt,
            "{}: RX confirmed",
            model.label
        );
        backend.disconnect().await;
        assert_eq!(
            wire.count("RX;"),
            1,
            "{}: disconnect must not add a second RX; after a confirmed release",
            model.label
        );
        wire.shutdown().await;
    }
}

/// Only the models whose reference defines `IF` receive a status query; the others rely on the
/// documented Auto-Information Answer and never see an undocumented command.
#[tokio::test(start_paused = true)]
async fn kenwood_status_read_is_sent_only_to_documenting_models() {
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        let (mut backend, wire) = connect(&model, cfg).await;

        let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(
            outcome.is_ok(),
            "{}: the documented TX; write succeeded",
            model.label
        );

        if model.documents_if {
            assert!(
                wire.sent("IF;"),
                "{}: IF; is documented for this model and must confirm the state",
                model.label
            );
            // The simulated IF answer reports TX, so the state is confirmed.
            assert!(
                backend.current_state().ptt,
                "{}: IF P8=1 confirms TX",
                model.label
            );
        } else {
            assert!(
                !wire.sent("IF;"),
                "{}: no official reference defines IF; — it must never be sent",
                model.label
            );
            assert!(
                !backend.current_state().ptt,
                "{}: unconfirmed state must not claim TX",
                model.label
            );
        }
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// An inconsistent documented `IF` status read (P8 = 0 while TX was requested) is never a
/// confirmation: the operation ends fail-closed after the documented timeout.
#[tokio::test(start_paused = true)]
async fn kenwood_inconsistent_status_read_is_not_confirmation() {
    // The simulated transceiver stays silent after TX; and reports RX (P8=0) when queried.
    let cfg = KenwoodWireConfig {
        tx_answer: PttAnswer::Silent,
        if_status_override: Some(false),
        ..KenwoodWireConfig::ts590s()
    };
    let (client_io, radio_io) = tokio::io::duplex(2048);
    let wire = kenwood_wire::spawn_kenwood_wire(radio_io, cfg);
    let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());
    backend.connect().await.expect("connect");

    let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
    assert!(
        outcome.is_err(),
        "an inconsistent documented status read must fail closed: {outcome:?}"
    );
    assert!(!backend.current_state().ptt);
    assert_eq!(wire.count("TX;"), 1, "no retry of PTT ON");

    backend.disconnect().await;
    assert!(
        wire.wait_for("RX;", Duration::from_secs(2)).await,
        "the release obligation survives an inconsistent status read"
    );
    wire.shutdown().await;
}

/// Documented answers are accepted, everything undocumented is rejected (no state from guesses).
#[test]
fn kenwood_only_documented_answers_are_accepted() {
    for answer in ["TX0;", "TX1;", "TX2;"] {
        assert_eq!(KenwoodCat::parse_ptt_resp(answer), Some(true), "{answer}");
    }
    assert_eq!(KenwoodCat::parse_ptt_resp("RX;"), Some(false));

    for undocumented in [
        "TX;", "TX3;", "TX9;", "TX00;", "TX01;", "?", "?;", "E;", "O;", "", "IF;", "IF0000;",
        "MD2;",
    ] {
        assert_eq!(
            KenwoodCat::parse_ptt_resp(undocumented),
            None,
            "undocumented answer '{undocumented}' must not produce a PTT state"
        );
    }
}

/// Documented error frames are rejections for every model and every direction, with no retry.
#[tokio::test(start_paused = true)]
async fn kenwood_documented_error_frames_fail_closed_per_model() {
    for model in shipped_models() {
        for error in [
            WireErrorFrame::Syntax,
            WireErrorFrame::Communication,
            WireErrorFrame::Overrun,
        ] {
            let mut cfg = model.config.clone();
            cfg.tx_answer = PttAnswer::Error(error);
            let (mut backend, wire) = connect(&model, cfg).await;
            let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
            assert!(
                outcome.is_err(),
                "{}: {} must be an error",
                model.label,
                error.wire()
            );
            assert!(!backend.current_state().ptt);
            assert_eq!(
                wire.count("TX;"),
                1,
                "{}: no retry after {}",
                model.label,
                error.wire()
            );
            backend.disconnect().await;
            wire.shutdown().await;

            let mut cfg = model.config.clone();
            cfg.rx_answer = PttAnswer::Error(error);
            let (mut backend, wire) = connect(&model, cfg).await;
            backend
                .execute(&RigCommand::SetPtt(true))
                .await
                .expect("TX");
            let outcome = backend.execute(&RigCommand::SetPtt(false)).await;
            assert!(
                outcome.is_err(),
                "{}: a rejected RX; is never a confirmed release",
                model.label
            );
            assert!(!backend.current_state().ptt);
            assert_eq!(
                wire.count("RX;"),
                1,
                "{}: no retry after {}",
                model.label,
                error.wire()
            );
            backend.disconnect().await;
            wire.shutdown().await;
        }
    }
}

/// A silent transceiver never produces a successful, confirmed TX; on the models without `IF`
/// the documented write is accepted but the state stays unconfirmed.
#[tokio::test(start_paused = true)]
async fn kenwood_silent_transceiver_never_yields_confirmed_tx() {
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        cfg.silent_status_read = true;
        let (mut backend, wire) = connect(&model, cfg).await;
        let outcome = backend.execute(&RigCommand::SetPtt(true)).await;

        if model.documents_if {
            assert!(
                matches!(outcome, Err(RigError::Timeout { .. })),
                "{}: a missing documented IF answer is a timeout: {outcome:?}",
                model.label
            );
        } else {
            assert!(
                outcome.is_ok(),
                "{}: the documented write succeeded; the state is merely unconfirmed",
                model.label
            );
        }
        assert!(
            !backend.current_state().ptt,
            "{}: no confirmed TX",
            model.label
        );
        assert_eq!(wire.count("TX;"), 1, "{}: exactly one PTT ON", model.label);
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// The documented release obligation survives an unconfirmed ON: disconnect writes `RX;` once.
#[tokio::test(start_paused = true)]
async fn kenwood_disconnect_releases_unconfirmed_tx_once_per_model() {
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        let (mut backend, wire) = connect(&model, cfg).await;
        let _ = backend.execute(&RigCommand::SetPtt(true)).await;
        backend.disconnect().await;

        assert!(
            wire.wait_for("RX;", Duration::from_secs(2)).await,
            "{}: unconfirmed TX must be released on disconnect",
            model.label
        );
        assert_eq!(
            wire.count("RX;"),
            1,
            "{}: exactly one release attempt",
            model.label
        );
        wire.shutdown().await;
    }
}

/// A model without an official document is rejected before any byte is written.
#[tokio::test(start_paused = true)]
async fn kenwood_unknown_model_fails_closed_without_writing() {
    for unknown in ["TS-2000", "TS-480", "TS-570", "K2"] {
        let mut profile = KENWOOD_TS590S.clone();
        profile.model_id = Box::leak(unknown.to_string().into_boxed_str());
        let (client_io, radio_io) = tokio::io::duplex(2048);
        let wire = kenwood_wire::spawn_kenwood_wire(radio_io, KenwoodWireConfig::ts590s());
        let mut backend = KenwoodBackend::with_stream(client_io, profile);
        backend.connect().await.expect("documented probe");

        let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(
            matches!(
                outcome,
                Err(RigError::Unsupported {
                    command: "SetPtt",
                    ..
                })
            ),
            "{unknown}: undocumented model must be refused: {outcome:?}"
        );
        assert!(
            ptt_frames(&wire).is_empty(),
            "{unknown}: no PTT bytes may be written"
        );
        assert!(!backend.current_state().ptt);
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// PTT requires a live transport and never invents one.
#[tokio::test(start_paused = true)]
async fn kenwood_ptt_requires_a_connected_transport() {
    let mut backend = KenwoodBackend::new_serial(
        splogbook::cat::native::transport::SerialConfig::new("COM1", 115_200),
        KENWOOD_TS590S.clone(),
    );
    let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
    assert_eq!(outcome, Err(RigError::NotConnected));
}

/// The S-meter read form is model-specific: `SM0;` for TS-590S/SG/990S, `SM;` for TS-890S.
#[tokio::test(start_paused = true)]
async fn kenwood_smeter_read_form_is_model_specific() {
    for model in shipped_models() {
        let (mut backend, wire) = connect(&model, model.config.clone()).await;
        let _ = backend.poll().await;
        assert!(
            wire.sent(model.smeter_read),
            "{}: documented S-meter read '{}'",
            model.label,
            model.smeter_read
        );
        if model.smeter_read == "SM;" {
            assert!(
                !wire.sent("SM0;"),
                "{}: TS-890S documents SM; without a parameter",
                model.label
            );
        }
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// `probe()` never sends an undocumented command: the fallback is the documented `ID;`.
#[tokio::test(start_paused = true)]
async fn kenwood_probe_uses_documented_id_fallback() {
    for model in shipped_models() {
        let (client_io, radio_io) = tokio::io::duplex(2048);
        let wire = kenwood_wire::spawn_kenwood_wire(radio_io, model.config.clone());
        let mut backend = KenwoodBackend::with_stream(client_io, model.profile.clone());
        backend.probe().await.expect("probe");
        assert!(
            !wire.sent("IF;"),
            "{}: probe must not send an undocumented IF;",
            model.label
        );
        wire.shutdown().await;
    }
}

// ============================================================================
// PHASE 15 — FAULT INJECTION
// ============================================================================

/// Deterministic fault injection over the real backend: transport loss, silent peers, documented
/// rejections, duplicate and stale answers, partial frames and cancellation. The invariants are:
/// no panic, no retry, no false confirmation, and the release obligation is preserved.
#[tokio::test(start_paused = true)]
async fn kenwood_fault_injection_matrix_keeps_safety_invariants() {
    // 1. Transport disappears immediately after `TX;` was written.
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        cfg.close_after_tx = true;
        let (client_io, radio_io) = tokio::io::duplex(2048);
        let wire = kenwood_wire::spawn_kenwood_wire(radio_io, cfg);
        let mut backend = KenwoodBackend::with_stream(client_io, model.profile.clone());
        backend.connect().await.expect("connect");
        let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(
            outcome.is_err(),
            "{}: transport loss is an error",
            model.label
        );
        assert!(!backend.current_state().ptt);
        assert_eq!(wire.count("TX;"), 1);
        backend.disconnect().await;
        wire.shutdown().await;
    }

    // 2. Duplicate documented answer + a stale frame for another command in one buffer.
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.push_raw_after_tx = Some("TX0;SM00015;");
        let (client_io, radio_io) = tokio::io::duplex(4096);
        let wire = kenwood_wire::spawn_kenwood_wire(radio_io, cfg);
        let mut backend = KenwoodBackend::with_stream(client_io, model.profile.clone());
        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("TX");
        // The duplicate TX0; and the stale SM; frame are drained by the next poll.
        let state = backend.poll().await.expect("poll");
        assert!(
            state.ptt,
            "{}: a duplicate documented TX0; keeps TX",
            model.label
        );
        assert_eq!(wire.count("TX;"), 1);
        backend
            .execute(&RigCommand::SetPtt(false))
            .await
            .expect("RX");
        assert!(!backend.current_state().ptt);
        backend.disconnect().await;
        wire.shutdown().await;
    }

    // 3. Cancellation after the documented TX; write keeps the release obligation.
    for model in shipped_models() {
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        let (mut backend, wire) = connect(&model, cfg).await;
        {
            let waker = std::task::Waker::noop();
            let mut context = std::task::Context::from_waker(waker);
            let mut future = Box::pin(backend.execute(&RigCommand::SetPtt(true)));
            let _ = std::future::Future::poll(future.as_mut(), &mut context);
        }
        assert!(
            wire.wait_for("TX;", Duration::from_millis(100)).await,
            "{}: the command was written",
            model.label
        );
        assert!(
            !backend.current_state().ptt,
            "{}: cancelled = unconfirmed",
            model.label
        );
        backend.disconnect().await;
        assert!(
            wire.wait_for("RX;", Duration::from_secs(2)).await,
            "{}: cancelled TX must still be released",
            model.label
        );
        wire.shutdown().await;
    }

    // 4. Truncated frame (no terminator) is never a state change.
    for model in shipped_models() {
        let (client_io, radio_io) = tokio::io::duplex(2048);
        let wire = kenwood_wire::spawn_raw_kenwood_wire(radio_io, "TX0");
        let mut backend = KenwoodBackend::with_stream(client_io, model.profile.clone());
        backend.connect().await.expect("connect");
        let state = backend.poll().await.expect("poll");
        assert!(
            !state.ptt,
            "{}: a frame without ';' is not an answer",
            model.label
        );
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

// ============================================================================
// PHASE 16 — COMBINATORIAL MATRIX
// ============================================================================

/// model × command × documented answer × transport outcome.
#[tokio::test(start_paused = true)]
async fn kenwood_combinatorial_matrix() {
    let answers = [
        PttAnswer::DocumentedAnswer,
        PttAnswer::Silent,
        PttAnswer::Error(WireErrorFrame::Syntax),
        PttAnswer::Error(WireErrorFrame::Overrun),
    ];

    for model in shipped_models() {
        for answer in answers {
            for ptt in [true, false] {
                let mut cfg = model.config.clone();
                if ptt {
                    cfg.tx_answer = answer;
                } else {
                    cfg.rx_answer = answer;
                }
                if answer == PttAnswer::Silent {
                    cfg.silent_status_read = true;
                }
                let (mut backend, wire) = connect(&model, cfg).await;
                if !ptt {
                    let _ = backend.execute(&RigCommand::SetPtt(true)).await;
                }
                let outcome = backend.execute(&RigCommand::SetPtt(ptt)).await;
                let state_ptt = backend.current_state().ptt;

                match answer {
                    PttAnswer::DocumentedAnswer => {
                        assert!(outcome.is_ok(), "{}: {ptt}: {outcome:?}", model.label);
                        assert_eq!(
                            state_ptt, ptt,
                            "{}: documented answer sets state",
                            model.label
                        );
                    }
                    PttAnswer::Silent => {
                        if model.documents_if && ptt {
                            assert!(outcome.is_err(), "{}: IF timeout", model.label);
                        }
                        assert!(!state_ptt, "{}: silence never confirms TX", model.label);
                    }
                    PttAnswer::Error(_) => {
                        assert!(outcome.is_err(), "{}: rejection is an error", model.label);
                        assert!(!state_ptt, "{}: rejection never confirms TX", model.label);
                    }
                }

                assert!(!wire.sent("TX0;"), "TX0; is never sent by the client");
                if !model.documents_if {
                    assert!(!wire.sent("IF;"), "{}: no undocumented IF;", model.label);
                }
                assert!(
                    wire.count("TX;") <= 1,
                    "{}: PTT ON is never retried",
                    model.label
                );
                backend.disconnect().await;
                wire.shutdown().await;
            }
        }
    }
}

// ============================================================================
// PHASE 17 — DETERMINISTIC STRESS
// ============================================================================

/// 1000 documented ON + 1000 documented OFF commands per supported profile.
#[tokio::test(start_paused = true)]
async fn kenwood_stress_documented_on_off_1000_per_profile() {
    for model in shipped_models() {
        let (mut backend, wire) = connect(&model, model.config.clone()).await;
        for iteration in 0..1000 {
            backend
                .execute(&RigCommand::SetPtt(true))
                .await
                .unwrap_or_else(|e| panic!("{}[{iteration}]: TX failed: {e}", model.label));
            assert!(backend.current_state().ptt);
            backend
                .execute(&RigCommand::SetPtt(false))
                .await
                .unwrap_or_else(|e| panic!("{}[{iteration}]: RX failed: {e}", model.label));
            assert!(!backend.current_state().ptt);
        }
        assert_eq!(wire.count("TX;"), 1000);
        assert_eq!(wire.count("RX;"), 1000);
        backend.disconnect().await;
        wire.shutdown().await;
    }
}

/// 500 documented rejections, 500 timeouts and 500 transport losses per supported profile.
#[tokio::test(start_paused = true)]
async fn kenwood_stress_error_timeout_disconnect_500_per_profile() {
    for model in shipped_models() {
        // 500 documented rejections (`?;`) on PTT ON — one `TX;` each, never retried.
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Error(WireErrorFrame::Syntax);
        let (mut backend, wire) = connect(&model, cfg).await;
        for iteration in 0..500 {
            let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
            assert!(outcome.is_err(), "{}[{iteration}]: rejection", model.label);
            assert!(!backend.current_state().ptt);
        }
        assert_eq!(wire.count("TX;"), 500);
        backend.disconnect().await;
        wire.shutdown().await;

        // 500 timeouts (silent transceiver, no status answer) — never a confirmed TX, no retry.
        let mut cfg = model.config.clone();
        cfg.tx_answer = PttAnswer::Silent;
        cfg.silent_status_read = true;
        let (mut backend, wire) = connect(&model, cfg).await;
        for iteration in 0..500 {
            let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
            if model.documents_if {
                assert!(
                    matches!(outcome, Err(RigError::Timeout { .. })),
                    "{}[{iteration}]: timeout expected: {outcome:?}",
                    model.label
                );
            } else {
                assert!(
                    outcome.is_ok(),
                    "{}[{iteration}]: unconfirmed write",
                    model.label
                );
            }
            assert!(!backend.current_state().ptt);
        }
        assert_eq!(wire.count("TX;"), 500);
        backend.disconnect().await;
        wire.shutdown().await;

        // 500 transport losses immediately after `TX;`.
        for iteration in 0..500 {
            let mut cfg = model.config.clone();
            cfg.tx_answer = PttAnswer::Silent;
            cfg.close_after_tx = true;
            let (client_io, radio_io) = tokio::io::duplex(2048);
            let wire = kenwood_wire::spawn_kenwood_wire(radio_io, cfg);
            let mut backend = KenwoodBackend::with_stream(client_io, model.profile.clone());
            backend.connect().await.expect("connect");
            let outcome = backend.execute(&RigCommand::SetPtt(true)).await;
            assert!(
                outcome.is_err(),
                "{}[{iteration}]: transport loss is an error",
                model.label
            );
            assert!(!backend.current_state().ptt);
            backend.disconnect().await;
            wire.shutdown().await;
        }
    }
}
