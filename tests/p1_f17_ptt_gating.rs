// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! F-17 — kontrakty centralnego dispatchera PTT.
//!
//! Testy integracyjne weryfikują **produkcyjny** moduł
//! [`splogbook::gui::ptt_input`]:
//!
//! * brak pojedynczego klawisza i rozłączność intencji ON/OFF,
//! * bramki fokusu tekstowego, IME, fokusu okna i modala,
//! * odrzucanie `release` i `repeat` przed jakimkolwiek wywołaniem kanału nadawczego,
//! * wymóg autoryzacji F-15 dla PTT ON (i brak takiego wymogu dla PTT OFF),
//! * macierz kombinatoryczną, wstrzykiwanie błędów i serie stresowe,
//! * brak paniki i brak emisji RF (kanał nadawczy jest wyłącznie rejestrującym doublem
//!   w zakresie testowym).
//!
//! Ten plik jest jedynym miejscem, w którym występuje
//! [`HilAuthorizationOutcome::HardwareExecutionAuthorized`] poza modułem F-15: pozytywna
//! kontrola dowodzi, że ścieżka ON jest osiągalna i poprawnie uporządkowana. Warstwa GUI
//! (`src/gui/**`) nie może nazwać tego wariantu — pilnuje tego `security_guard`.

use eframe::egui;
use splogbook::cat::hil::authorization::HilAuthorizationOutcome;
use splogbook::gui::ptt_input::{
    PttActionSource, PttDispatchOutcome, PttFocusReleaseGuard, PttImeTracker, PttInputSnapshot,
    PttIntent, PttPortError, PttTransmitPort, PttTransmitState, dispatch, project_tx_state,
};
use splogbook::gui::shortcuts::ShortcutGates;

/// Rejestrujący kanał nadawczy — **wyłącznie zakres testowy**. Nie jest kompilowany do
/// programu produkcyjnego i nie może wybrać żadnej ścieżki wykonawczej.
#[derive(Debug, Default, Clone)]
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

const AUTHORIZED: HilAuthorizationOutcome = HilAuthorizationOutcome::HardwareExecutionAuthorized;

const ALL_AUTHORIZATION_OUTCOMES: [HilAuthorizationOutcome; 14] = [
    HilAuthorizationOutcome::NotEvaluated,
    HilAuthorizationOutcome::ConfigurationInvalid,
    HilAuthorizationOutcome::OperatorAttestationMissing,
    HilAuthorizationOutcome::OperatorAttestationInvalid,
    HilAuthorizationOutcome::EvidenceMissing,
    HilAuthorizationOutcome::EvidenceInvalid,
    HilAuthorizationOutcome::EvidenceExpired,
    HilAuthorizationOutcome::EvidenceNotYetValid,
    HilAuthorizationOutcome::EvidenceScopeMismatch,
    HilAuthorizationOutcome::EvidenceReplayed,
    HilAuthorizationOutcome::IndependentApprovalMissing,
    HilAuthorizationOutcome::DryRunOnly,
    HilAuthorizationOutcome::HardwareExecutionDenied,
    HilAuthorizationOutcome::HardwareExecutionAuthorized,
];

const ALL_TX_STATES: [PttTransmitState; 5] = [
    PttTransmitState::Idle,
    PttTransmitState::Requesting,
    PttTransmitState::Owned,
    PttTransmitState::Releasing,
    PttTransmitState::Unknown,
];

fn open_gates() -> ShortcutGates {
    ShortcutGates {
        text_editing: false,
        ime_composing: false,
        window_focused: true,
        modal_open: false,
    }
}

/// Profile bramek użyte w macierzy kombinatorycznej.
fn gate_profiles() -> [(&'static str, ShortcutGates); 6] {
    [
        ("open", open_gates()),
        (
            "text",
            ShortcutGates {
                text_editing: true,
                ..open_gates()
            },
        ),
        (
            "ime",
            ShortcutGates {
                ime_composing: true,
                ..open_gates()
            },
        ),
        (
            "unfocused",
            ShortcutGates {
                window_focused: false,
                ..open_gates()
            },
        ),
        (
            "modal",
            ShortcutGates {
                modal_open: true,
                ..open_gates()
            },
        ),
        ("fail_closed", ShortcutGates::fail_closed()),
    ]
}

fn snapshot(
    source: PttActionSource,
    intent: PttIntent,
    pressed: bool,
    repeat: bool,
    modifiers_ok: bool,
    gates: ShortcutGates,
) -> PttInputSnapshot {
    PttInputSnapshot {
        source,
        intent,
        pressed,
        repeat,
        modifiers_ok,
        gates,
    }
}

fn shortcut_snapshot(intent: PttIntent) -> PttInputSnapshot {
    snapshot(
        PttActionSource::KeyboardShortcut,
        intent,
        true,
        false,
        true,
        open_gates(),
    )
}

// ==========================================================================================
// 1. Podstawowy kontrakt: rozłączność intencji i brak pojedynczego klawisza
// ==========================================================================================

#[test]
fn f17_ptt_intents_are_disjoint_and_named_explicitly() {
    assert_ne!(PttIntent::RequestOn, PttIntent::RequestOff);
    assert_eq!(PttIntent::RequestOn.as_str(), "request_on");
    assert_eq!(PttIntent::RequestOff.as_str(), "request_off");
    // Brak wariantu „toggle” w modelu intencji.
    assert!(!PttIntent::RequestOn.as_str().contains("toggle"));
    assert!(!PttIntent::RequestOff.as_str().contains("toggle"));
}

#[test]
fn f17_all_sources_are_production_sources() {
    for source in [
        PttActionSource::KeyboardShortcut,
        PttActionSource::ScreenButton,
        PttActionSource::CommandPalette,
        PttActionSource::FocusLossRelease,
    ] {
        assert!(!source.as_str().is_empty());
        // Żadne źródło nie jest testowym providerem wejścia.
        for forbidden in ["mock", "fake", "test", "double", "simulated"] {
            assert!(
                !source.as_str().contains(forbidden),
                "{source:?} must not expose a test-only input provider"
            );
        }
    }
}

#[test]
fn f17_authorized_on_request_is_reachable_and_calls_the_port_once() {
    let mut port = RecordingPort::default();
    let outcome = dispatch(
        &shortcut_snapshot(PttIntent::RequestOn),
        PttTransmitState::Idle,
        AUTHORIZED,
        &mut port,
    );
    assert_eq!(outcome, PttDispatchOutcome::RequestedOn);
    assert!(outcome.is_request());
    assert_eq!(port.on_calls, 1);
    assert_eq!(port.off_calls, 0);
}

#[test]
fn f17_off_never_requires_authorization() {
    for authorization in ALL_AUTHORIZATION_OUTCOMES {
        let mut port = RecordingPort::default();
        let outcome = dispatch(
            &shortcut_snapshot(PttIntent::RequestOff),
            PttTransmitState::Owned,
            authorization,
            &mut port,
        );
        assert_eq!(outcome, PttDispatchOutcome::RequestedOff, "{authorization}");
        assert_eq!(port.off_calls, 1);
        assert_eq!(port.on_calls, 0);
    }
}

// ==========================================================================================
// 2. Bramki wejściowe: fokus tekstowy, IME, fokus okna, modal
// ==========================================================================================

#[test]
fn f17_release_and_repeat_are_rejected_before_any_gate() {
    let mut port = RecordingPort::default();
    for (pressed, repeat, expected) in [
        (false, false, PttDispatchOutcome::IgnoredRelease),
        (true, true, PttDispatchOutcome::IgnoredRepeat),
        (false, true, PttDispatchOutcome::IgnoredRelease),
    ] {
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            pressed,
            repeat,
            true,
            open_gates(),
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, expected);
    }
    assert_eq!(port.on_calls, 0);
    assert_eq!(port.off_calls, 0);
}

#[test]
fn f17_modifier_mismatch_is_rejected() {
    let mut port = RecordingPort::default();
    let input = snapshot(
        PttActionSource::KeyboardShortcut,
        PttIntent::RequestOn,
        true,
        false,
        false,
        open_gates(),
    );
    assert_eq!(
        dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port),
        PttDispatchOutcome::ShortcutMismatch
    );
    assert_eq!(port.on_calls, 0);
}

#[test]
fn f17_each_gate_blocks_both_intents() {
    for (name, gates) in gate_profiles() {
        if name == "open" {
            continue;
        }
        let mut port = RecordingPort::default();
        for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
            for tx_state in ALL_TX_STATES {
                let input = snapshot(
                    PttActionSource::KeyboardShortcut,
                    intent,
                    true,
                    false,
                    true,
                    gates,
                );
                let outcome = dispatch(&input, tx_state, AUTHORIZED, &mut port);
                assert!(
                    !outcome.is_request(),
                    "{name} must block {intent:?} in {tx_state:?}"
                );
            }
        }
        assert_eq!(port.on_calls, 0, "{name}");
        assert_eq!(port.off_calls, 0, "{name}");
    }
}

#[test]
fn f17_focus_loss_release_bypasses_input_gates_but_only_for_off() {
    let mut port = RecordingPort::default();
    let off = snapshot(
        PttActionSource::FocusLossRelease,
        PttIntent::RequestOff,
        true,
        false,
        true,
        ShortcutGates::fail_closed(),
    );
    assert_eq!(
        dispatch(&off, PttTransmitState::Owned, AUTHORIZED, &mut port),
        PttDispatchOutcome::RequestedOff
    );
    assert_eq!(port.off_calls, 1);

    for authorization in ALL_AUTHORIZATION_OUTCOMES {
        let on = snapshot(
            PttActionSource::FocusLossRelease,
            PttIntent::RequestOn,
            true,
            false,
            true,
            open_gates(),
        );
        let outcome = dispatch(&on, PttTransmitState::Idle, authorization, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::ShortcutMismatch);
    }
    assert_eq!(port.on_calls, 0);
}

// ==========================================================================================
// 3. Wstrzykiwanie błędów
// ==========================================================================================

#[test]
fn f17_port_errors_are_failures_never_successes() {
    let mut on_port = RecordingPort {
        fail_on: Some(PttPortError::NoActiveRig),
        ..RecordingPort::default()
    };
    let outcome = dispatch(
        &shortcut_snapshot(PttIntent::RequestOn),
        PttTransmitState::Idle,
        AUTHORIZED,
        &mut on_port,
    );
    assert_eq!(
        outcome,
        PttDispatchOutcome::Failed(PttPortError::NoActiveRig)
    );
    assert!(outcome.is_denial());
    assert_eq!(outcome.code(), "failed");

    let mut off_port = RecordingPort {
        fail_off: Some(PttPortError::RequestChannelClosed),
        ..RecordingPort::default()
    };
    let outcome = dispatch(
        &shortcut_snapshot(PttIntent::RequestOff),
        PttTransmitState::Owned,
        AUTHORIZED,
        &mut off_port,
    );
    assert_eq!(
        outcome,
        PttDispatchOutcome::Failed(PttPortError::RequestChannelClosed)
    );
    assert!(
        outcome.is_denial(),
        "an OFF error must never be reported as success"
    );
}

#[test]
fn f17_arbiter_error_states_never_call_the_port() {
    for tx_state in [
        PttTransmitState::Requesting,
        PttTransmitState::Releasing,
        PttTransmitState::Unknown,
    ] {
        let mut port = RecordingPort::default();
        for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
            let outcome = dispatch(&shortcut_snapshot(intent), tx_state, AUTHORIZED, &mut port);
            assert!(!outcome.is_request(), "{tx_state:?} / {intent:?}");
        }
        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }
}

#[test]
fn f17_unknown_tx_state_requires_resynchronisation_and_is_never_optimistic() {
    for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
        let mut port = RecordingPort::default();
        assert_eq!(
            dispatch(
                &shortcut_snapshot(intent),
                PttTransmitState::Unknown,
                AUTHORIZED,
                &mut port
            ),
            PttDispatchOutcome::RefusedUnknownTxState
        );
        assert_eq!(port.on_calls, 0);
        assert_eq!(port.off_calls, 0);
    }
}

// ==========================================================================================
// 4. Macierz kombinatoryczna (deterministyczna, bez czasu ściennego)
// ==========================================================================================

/// Czy wszystkie bramki wejściowe są otwarte.
fn gates_open(gates: ShortcutGates) -> bool {
    !gates.text_editing && !gates.ime_composing && gates.window_focused && !gates.modal_open
}

#[test]
fn f17_combinatorial_matrix_holds_every_safety_invariant() {
    let sources = [
        PttActionSource::KeyboardShortcut,
        PttActionSource::ScreenButton,
        PttActionSource::CommandPalette,
        PttActionSource::FocusLossRelease,
    ];
    let mut cases = 0usize;
    let mut requests = 0usize;
    let mut on_requests = 0usize;

    for source in sources {
        for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
            for pressed in [true, false] {
                for repeat in [true, false] {
                    for modifiers_ok in [true, false] {
                        for (_, gates) in gate_profiles() {
                            for tx_state in ALL_TX_STATES {
                                for authorization in ALL_AUTHORIZATION_OUTCOMES {
                                    cases += 1;
                                    let input = snapshot(
                                        source,
                                        intent,
                                        pressed,
                                        repeat,
                                        modifiers_ok,
                                        gates,
                                    );
                                    let mut port = RecordingPort::default();
                                    let outcome =
                                        dispatch(&input, tx_state, authorization, &mut port);

                                    // P1/P2: dokładnie jedno wywołanie kanału dla żądania, zero dla odmowy.
                                    if outcome.is_request() {
                                        requests += 1;
                                        assert_eq!(
                                            port.on_calls + port.off_calls,
                                            1,
                                            "a request must call the port exactly once: {input:?} {tx_state:?} {authorization}"
                                        );
                                    } else {
                                        assert_eq!(port.on_calls, 0, "{input:?} {tx_state:?}");
                                        assert_eq!(port.off_calls, 0, "{input:?} {tx_state:?}");
                                    }

                                    // P3: PTT ON wyłącznie z autoryzacją, w stanie Idle, po pełnej bramce.
                                    if outcome == PttDispatchOutcome::RequestedOn {
                                        on_requests += 1;
                                        assert_eq!(intent, PttIntent::RequestOn);
                                        assert_eq!(
                                            authorization,
                                            HilAuthorizationOutcome::HardwareExecutionAuthorized
                                        );
                                        assert_eq!(tx_state, PttTransmitState::Idle);
                                        assert_ne!(source, PttActionSource::FocusLossRelease);
                                        assert!(pressed && !repeat && modifiers_ok);
                                        assert!(gates_open(gates));
                                    }

                                    // P4: PTT OFF wyłącznie z potwierdzonego stanu Owned.
                                    if outcome == PttDispatchOutcome::RequestedOff {
                                        assert_eq!(intent, PttIntent::RequestOff);
                                        assert_eq!(tx_state, PttTransmitState::Owned);
                                        if source != PttActionSource::FocusLossRelease {
                                            assert!(pressed && !repeat && modifiers_ok);
                                            assert!(gates_open(gates));
                                        }
                                    }

                                    // P5: żadne wywołanie ON bez autoryzacji.
                                    if port.on_calls > 0 {
                                        assert_eq!(
                                            authorization,
                                            HilAuthorizationOutcome::HardwareExecutionAuthorized,
                                            "no PTT ON without F-15 authorization"
                                        );
                                        assert_ne!(source, PttActionSource::FocusLossRelease);
                                    }

                                    // P6: determinizm — ta sama kombinacja daje ten sam wynik.
                                    let mut second_port = RecordingPort::default();
                                    let second =
                                        dispatch(&input, tx_state, authorization, &mut second_port);
                                    assert_eq!(outcome, second);
                                    assert_eq!(port.on_calls, second_port.on_calls);
                                    assert_eq!(port.off_calls, second_port.off_calls);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(
        cases >= 6_000,
        "the combinatorial matrix must be substantial, got {cases}"
    );
    // Pozytywna kontrola musi być osiągalna — inaczej macierz nie dowodzi niczego.
    assert!(on_requests > 0, "the authorized ON path must be reachable");
    assert!(requests > on_requests);
}

// ==========================================================================================
// 5. Serie stresowe (deterministyczne, bez sleep)
// ==========================================================================================

#[test]
fn f17_stress_bare_f7_equivalent_is_always_denied() {
    let mut denials = 0usize;
    for _ in 0..5_000 {
        let mut port = RecordingPort::default();
        // Odpowiednik „samego F7”: modyfikatory nie spełniają kontraktu.
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            true,
            false,
            false,
            open_gates(),
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::ShortcutMismatch);
        assert_eq!(port.on_calls, 0);
        denials += 1;
    }
    assert_eq!(denials, 5_000);
}

#[test]
fn f17_stress_text_focus_is_always_denied() {
    let mut denials = 0usize;
    for _ in 0..5_000 {
        let mut port = RecordingPort::default();
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            true,
            false,
            true,
            ShortcutGates {
                text_editing: true,
                ..open_gates()
            },
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::IgnoredTextEditing);
        assert_eq!(port.on_calls, 0);
        denials += 1;
    }
    assert_eq!(denials, 5_000);
}

#[test]
fn f17_stress_repeat_is_always_denied() {
    let mut denials = 0usize;
    for _ in 0..5_000 {
        let mut port = RecordingPort::default();
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            true,
            true,
            true,
            open_gates(),
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::IgnoredRepeat);
        assert_eq!(port.on_calls, 0);
        denials += 1;
    }
    assert_eq!(denials, 5_000);
}

#[test]
fn f17_stress_missing_authorization_is_always_denied() {
    let mut denials = 0usize;
    for authorization in ALL_AUTHORIZATION_OUTCOMES {
        if authorization == HilAuthorizationOutcome::HardwareExecutionAuthorized {
            continue;
        }
        for _ in 0..385 {
            let mut port = RecordingPort::default();
            let outcome = dispatch(
                &shortcut_snapshot(PttIntent::RequestOn),
                PttTransmitState::Idle,
                authorization,
                &mut port,
            );
            assert!(matches!(
                outcome,
                PttDispatchOutcome::AuthorizationDenied(_)
            ));
            assert_eq!(port.on_calls, 0);
            denials += 1;
        }
    }
    assert_eq!(denials, 13 * 385);
}

#[test]
fn f17_stress_focus_loss_is_always_bounded_to_one_attempt() {
    for _ in 0..2_000 {
        let mut guard = PttFocusReleaseGuard::new();
        assert_eq!(guard.observe(true, PttTransmitState::Owned), None);
        assert_eq!(
            guard.observe(false, PttTransmitState::Owned),
            Some(PttIntent::RequestOff)
        );
        for _ in 0..8 {
            assert_eq!(guard.observe(false, PttTransmitState::Owned), None);
        }
    }
}

#[test]
fn f17_stress_ime_is_always_denied() {
    for _ in 0..2_000 {
        let mut tracker = PttImeTracker::new();
        let preedit = egui_event_preedit("に");
        assert!(tracker.observe_frame(std::slice::from_ref(&preedit)));
        let mut port = RecordingPort::default();
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            true,
            false,
            true,
            ShortcutGates {
                ime_composing: true,
                ..open_gates()
            },
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::IgnoredImeActive);
        assert_eq!(port.on_calls, 0);
    }
}

#[test]
fn f17_stress_modal_is_always_denied() {
    for _ in 0..2_000 {
        let mut port = RecordingPort::default();
        let input = snapshot(
            PttActionSource::KeyboardShortcut,
            PttIntent::RequestOn,
            true,
            false,
            true,
            ShortcutGates {
                modal_open: true,
                ..open_gates()
            },
        );
        let outcome = dispatch(&input, PttTransmitState::Idle, AUTHORIZED, &mut port);
        assert_eq!(outcome, PttDispatchOutcome::IgnoredModalOpen);
        assert_eq!(port.on_calls, 0);
    }
}

#[test]
fn f17_stress_multi_event_frames_never_duplicate_intents() {
    for _ in 0..2_000 {
        let mut port = RecordingPort::default();
        // Pięć zdarzeń w jednej klatce: po pierwszym żądaniu arbiter przechodzi w `Requesting`,
        // więc kolejne odczyty tej samej klatki nie mogą powtórzyć ON.
        let mut tx_state = PttTransmitState::Idle;
        let mut requests = 0usize;
        for _ in 0..5 {
            let outcome = dispatch(
                &shortcut_snapshot(PttIntent::RequestOn),
                tx_state,
                AUTHORIZED,
                &mut port,
            );
            if outcome.is_request() {
                requests += 1;
                tx_state = PttTransmitState::Requesting;
            }
        }
        assert_eq!(
            requests, 1,
            "a frame with several events may produce at most one ON"
        );
        assert_eq!(port.on_calls, 1);
    }
}

#[test]
fn f17_stress_cancellation_and_arbiter_errors_are_denied() {
    for _ in 0..1_000 {
        let mut port = RecordingPort::default();
        // Anulowanie: stan Requesting/Releasing, czyli żądanie już nadane i niepotwierdzone.
        for tx_state in [PttTransmitState::Requesting, PttTransmitState::Releasing] {
            let outcome = dispatch(
                &shortcut_snapshot(PttIntent::RequestOff),
                tx_state,
                AUTHORIZED,
                &mut port,
            );
            assert!(!outcome.is_request());
        }
        assert_eq!(port.off_calls, 0);
    }
}

#[test]
fn f17_stress_off_error_is_never_a_success() {
    for _ in 0..1_000 {
        let mut port = RecordingPort {
            fail_off: Some(PttPortError::NoActiveRig),
            ..RecordingPort::default()
        };
        let outcome = dispatch(
            &shortcut_snapshot(PttIntent::RequestOff),
            PttTransmitState::Owned,
            AUTHORIZED,
            &mut port,
        );
        assert!(matches!(outcome, PttDispatchOutcome::Failed(_)));
        assert!(outcome.is_denial());
    }
}

#[test]
fn f17_stress_test_only_port_double_is_lawful() {
    // 500 iteracji rejestrującego double: dowód, że double nie omija dispatchera
    // i że każde żądanie jest dokładnie jednym wywołaniem kanału.
    for _ in 0..500 {
        let mut port = RecordingPort::default();
        assert_eq!(
            dispatch(
                &shortcut_snapshot(PttIntent::RequestOn),
                PttTransmitState::Idle,
                AUTHORIZED,
                &mut port
            ),
            PttDispatchOutcome::RequestedOn
        );
        assert_eq!(port.on_calls, 1);
    }
}

// ==========================================================================================
// 6. Kontrakty pomocnicze: IME, strażnik fokusu, projekcja stanu
// ==========================================================================================

fn egui_event_preedit(text: &str) -> egui::Event {
    egui::Event::Ime(egui::ImeEvent::Preedit {
        text: text.to_string(),
        active_range_chars: None,
    })
}

#[test]
fn f17_ime_tracker_is_sticky_and_fail_closed() {
    let mut tracker = PttImeTracker::new();
    assert!(!tracker.is_composing());
    assert!(!tracker.observe_frame(&[]));

    let preedit = egui_event_preedit("あ");
    assert!(tracker.observe_frame(std::slice::from_ref(&preedit)));
    assert!(tracker.is_composing());
    // Kolejne klatki bez zdarzeń IME pozostają zamknięte — kompozycja trwa.
    for _ in 0..50 {
        assert!(tracker.observe_frame(&[]));
    }

    let commit = egui::Event::Ime(egui::ImeEvent::Commit("あ".to_string()));
    assert!(tracker.observe_frame(std::slice::from_ref(&commit)));
    assert!(!tracker.is_composing());
    assert!(!tracker.observe_frame(&[]));

    // Reset (utrata fokusu okna) nie może pozostawić zamkniętej bramki na zawsze.
    tracker.reset();
    assert!(!tracker.is_composing());
}

#[test]
fn f17_ime_tracker_treats_every_ime_frame_as_closed() {
    let mut tracker = PttImeTracker::new();
    let empty_preedit = egui::Event::Ime(egui::ImeEvent::Preedit {
        text: String::new(),
        active_range_chars: None,
    });
    assert!(tracker.observe_frame(std::slice::from_ref(&empty_preedit)));
    assert!(!tracker.is_composing());
    assert!(!tracker.observe_frame(&[]));

    let delete = egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
        before_chars: 2,
        after_chars: 0,
    });
    assert!(tracker.observe_frame(std::slice::from_ref(&delete)));
    assert!(tracker.is_composing());
}

#[test]
fn f17_focus_release_guard_requires_confirmed_transmit() {
    for tx_state in [
        PttTransmitState::Idle,
        PttTransmitState::Requesting,
        PttTransmitState::Releasing,
        PttTransmitState::Unknown,
    ] {
        let mut guard = PttFocusReleaseGuard::new();
        assert_eq!(guard.observe(true, tx_state), None);
        assert_eq!(guard.observe(false, tx_state), None, "{tx_state:?}");
    }
}

#[test]
fn f17_tx_state_projection_is_total() {
    use splogbook::cat::routing::{RigCommandId, RigId};
    use splogbook::cat::tx_arbiter::{TxArbiterState, TxUnknownReason};
    let rig = RigId::hub();
    let command = RigCommandId::new(7);
    assert_eq!(
        project_tx_state(&TxArbiterState::Idle),
        PttTransmitState::Idle
    );
    assert_eq!(
        project_tx_state(&TxArbiterState::Requesting {
            rig_id: rig.clone(),
            command_id: command
        }),
        PttTransmitState::Requesting
    );
    assert_eq!(
        project_tx_state(&TxArbiterState::Owned {
            rig_id: rig.clone(),
            command_id: command
        }),
        PttTransmitState::Owned
    );
    assert_eq!(
        project_tx_state(&TxArbiterState::Releasing {
            rig_id: rig,
            command_id: command
        }),
        PttTransmitState::Releasing
    );
    assert_eq!(
        project_tx_state(&TxArbiterState::Unknown {
            rig_id: None,
            reason: TxUnknownReason::Timeout
        }),
        PttTransmitState::Unknown
    );
}

#[test]
fn f17_outcome_codes_are_stable() {
    let expected = [
        (PttDispatchOutcome::IgnoredRelease, "ignored_release"),
        (PttDispatchOutcome::IgnoredRepeat, "ignored_repeat"),
        (
            PttDispatchOutcome::IgnoredTextEditing,
            "ignored_text_editing",
        ),
        (PttDispatchOutcome::IgnoredImeActive, "ignored_ime_active"),
        (
            PttDispatchOutcome::IgnoredWindowUnfocused,
            "ignored_window_unfocused",
        ),
        (PttDispatchOutcome::IgnoredModalOpen, "ignored_modal_open"),
        (PttDispatchOutcome::ShortcutMismatch, "shortcut_mismatch"),
        (
            PttDispatchOutcome::AlreadyTransmitting,
            "already_transmitting",
        ),
        (PttDispatchOutcome::NotTransmitting, "not_transmitting"),
        (PttDispatchOutcome::RefusedBusy, "refused_busy"),
        (
            PttDispatchOutcome::RefusedUnknownTxState,
            "refused_unknown_tx_state",
        ),
        (
            PttDispatchOutcome::AuthorizationDenied(HilAuthorizationOutcome::NotEvaluated),
            "authorization_denied",
        ),
        (PttDispatchOutcome::RequestedOn, "requested_on"),
        (PttDispatchOutcome::RequestedOff, "requested_off"),
        (
            PttDispatchOutcome::Failed(PttPortError::NoActiveRig),
            "failed",
        ),
    ];
    for (outcome, code) in expected {
        assert_eq!(outcome.code(), code);
    }
}

// ==========================================================================================
// 7. Brak paniki w całej przestrzeni wejść (panic-free sweep)
// ==========================================================================================

#[test]
fn f17_every_input_never_panics_and_never_emits_rf() {
    let sources = [
        PttActionSource::KeyboardShortcut,
        PttActionSource::ScreenButton,
        PttActionSource::CommandPalette,
        PttActionSource::FocusLossRelease,
    ];
    let mut executed = 0usize;
    for source in sources {
        for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
            for pressed in [true, false] {
                for repeat in [true, false] {
                    for modifiers_ok in [true, false] {
                        for (_, gates) in gate_profiles() {
                            for tx_state in ALL_TX_STATES {
                                let input =
                                    snapshot(source, intent, pressed, repeat, modifiers_ok, gates);
                                let mut port = RecordingPort {
                                    fail_on: Some(PttPortError::NoActiveRig),
                                    fail_off: Some(PttPortError::RequestChannelClosed),
                                    ..RecordingPort::default()
                                };
                                let outcome = dispatch(
                                    &input,
                                    tx_state,
                                    HilAuthorizationOutcome::NotEvaluated,
                                    &mut port,
                                );
                                // Brak autoryzacji ⇒ zero PTT ON, niezależnie od stanu.
                                assert_eq!(port.on_calls, 0);
                                assert!(!outcome.code().is_empty());
                                executed += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(executed, 4 * 2 * 2 * 2 * 2 * 6 * 5);
}

#[test]
fn f17_zero_ptt_on_without_authorization_across_the_whole_matrix() {
    let mut on_count = 0usize;
    let mut off_count = 0usize;
    for source in [
        PttActionSource::KeyboardShortcut,
        PttActionSource::ScreenButton,
        PttActionSource::CommandPalette,
    ] {
        for intent in [PttIntent::RequestOn, PttIntent::RequestOff] {
            for (_, gates) in gate_profiles() {
                for tx_state in ALL_TX_STATES {
                    for authorization in ALL_AUTHORIZATION_OUTCOMES {
                        if authorization == HilAuthorizationOutcome::HardwareExecutionAuthorized {
                            continue;
                        }
                        let input = snapshot(source, intent, true, false, true, gates);
                        let mut port = RecordingPort::default();
                        let outcome = dispatch(&input, tx_state, authorization, &mut port);
                        if outcome == PttDispatchOutcome::RequestedOn {
                            on_count += 1;
                        }
                        if outcome == PttDispatchOutcome::RequestedOff {
                            off_count += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(on_count, 0, "PTT_ON_COUNT_WITHOUT_AUTHORIZATION must be 0");
    // PTT OFF pozostaje dostępne, gdy arbiter potwierdza nadawanie — to wymóg bezpieczeństwa.
    assert!(
        off_count > 0,
        "PTT OFF must stay reachable without authorization"
    );
}
