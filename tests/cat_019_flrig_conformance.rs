// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! CAT-019 — SOFTWARE-ONLY FLRig XML-RPC PTT protocol conformance.
//!
//! Every assertion is grounded in the official FLRig **2.0.00** author source
//! (`https://github.com/w1hkj/flrig`, tag `v2.0.00`):
//!
//! * `src/server/xml_server.cxx` `mlist[]`:
//!   `{ "rig.set_ptt", "n:i", "set PTT 1/0 (on/off)" }`,
//!   `{ "rig.get_ptt", "i:n", "return PTT state" }`,
//!   `{ "rig.set_verify_ptt", "n:i", "set & verify PTT 1/0 (on/off)" }`,
//!   `{ "rig.set_ptt_fast", "n:i", "deprecated; use set_ptt" }`;
//! * `rig_set_ptt::execute` → `PTT = int(params[0]);` and **no** result assignment;
//! * `rig_get_ptt::execute` → `result = int(PTT);`;
//! * both return `result = 0` (an `<int>`) when the transceiver is offline or XML-RPC is
//!   disabled — the documented signature is `n:i`, so a successful set answers *nil*;
//! * `src/xmlrpcpp/XmlRpcValue.cpp` `assertType(TypeInt)` throws `XmlRpcException("type error")`
//!   for a non-integer argument; the default fault code is −1 (`XmlRpcException.h`).
//!
//! The tests drive the **real** production `FlrigClient` against a test-only loopback XML-RPC
//! server (`tests/support/flrig_wire.rs`), on a dynamic `127.0.0.1` port. No public network, no
//! real FLRig daemon, no transceiver, no PTT and no RF is involved.

use std::time::Duration;

use splogbook::cat::flrig::FlrigClient;

#[path = "support/flrig_wire.rs"]
mod flrig_wire;

use flrig_wire::{FlrigReply, FlrigWire, refused_endpoint, spawn_flrig_wire};

/// The exact documented XML-RPC body for `rig.set_ptt` with the documented integer parameter.
fn documented_set_ptt_body(value: i64) -> String {
    format!(
        "<?xml version=\"1.0\"?>\n<methodCall><methodName>rig.set_ptt</methodName><params><param><value><int>{value}</int></value></param></params></methodCall>"
    )
}

async fn client_for(reply: FlrigReply) -> (FlrigClient, FlrigWire) {
    let wire = spawn_flrig_wire(reply).await;
    let client = FlrigClient::new("127.0.0.1", wire.port());
    (client, wire)
}

// ============================================================================
// PHASE 9 — PROGRAMMATIC REPRODUCTION OF CAT-019
// ============================================================================

/// `rig.set_ptt` is documented as `n:i` and its implementation consumes the parameter with
/// `int(params[0])`: the wire must carry exactly one `<int>` parameter with the value `1`/`0`.
#[tokio::test]
async fn cat019_set_ptt_request_uses_the_documented_integer_type() {
    for (ptt, value) in [(true, 1_i64), (false, 0_i64)] {
        let (client, wire) = client_for(FlrigReply::Nil).await;
        let _ = client.set_ptt(ptt).await;

        let body = wire.last_body();
        assert_eq!(
            body,
            documented_set_ptt_body(value),
            "rig.set_ptt must carry the documented <int> parameter"
        );
        assert!(!body.contains("<boolean>"), "no boolean parameter");
        wire.shutdown().await;
    }
}

/// The documented signature `n:i` returns nothing: the daemon answers with a `<param>` that has no
/// `<value>`. That is a **successful** PTT command, not a parse failure.
#[tokio::test]
async fn cat019_documented_nil_response_is_success() {
    let (client, wire) = client_for(FlrigReply::Nil).await;
    let outcome = client.set_ptt(true).await;
    assert!(
        outcome.is_ok(),
        "the documented nil response is a successful rig.set_ptt: {outcome:?}"
    );
    assert_eq!(wire.request_count(), 1);
    wire.shutdown().await;
}

/// When the transceiver is offline or XML-RPC is disabled the daemon answers `<int>0</int>`
/// without touching PTT. That answer must never be reported as a successful PTT command.
#[tokio::test]
async fn cat019_offline_integer_answer_is_not_success() {
    let (client, wire) = client_for(FlrigReply::Int(0)).await;
    let outcome = client.set_ptt(true).await;
    assert!(
        outcome.is_err(),
        "rig.set_ptt answered <int>0</int>: the daemon did not execute the command"
    );
    wire.shutdown().await;
}

/// A documented argument-type rejection is a fault and must fail closed (no second PTT ON, no
/// boolean/int fallback).
#[tokio::test]
async fn cat019_type_error_fault_fails_closed() {
    let (client, wire) = client_for(FlrigReply::Fault(-1, "type error")).await;
    let outcome = client.set_ptt(true).await;
    assert!(outcome.is_err(), "a type error fault is an error");
    assert_eq!(
        wire.request_count(),
        1,
        "a type error must never be retried with another XML-RPC type"
    );
    wire.shutdown().await;
}

// ============================================================================
// PHASE 14 — FULL SET/GET CONFORMANCE
// ============================================================================

/// One method name, exactly one parameter, the documented HTTP endpoint.
#[tokio::test]
async fn cat019_request_has_one_method_one_parameter_and_the_documented_endpoint() {
    let (client, wire) = client_for(FlrigReply::Nil).await;
    client.set_ptt(true).await.expect("documented set");
    let request = wire.requests().into_iter().next().expect("one request");

    assert!(
        request.head.starts_with("POST /RPC2 HTTP/1.1"),
        "documented XML-RPC endpoint path: {}",
        request.head
    );
    assert!(
        request
            .head
            .to_ascii_lowercase()
            .contains(&format!("host: 127.0.0.1:{}", wire.port())),
        "Host header must carry the daemon endpoint"
    );
    assert!(request.head.contains("Content-Type: text/xml"));
    assert_eq!(request.body.matches("<methodName>").count(), 1);
    assert_eq!(request.body.matches("<param>").count(), 1);
    assert_eq!(request.body.matches("<value>").count(), 1);
    assert!(request.body.ends_with("</methodCall>"));
    wire.shutdown().await;
}

/// `rig.get_ptt` is documented as `i:n`: an `<int>` 0/1 is the only documented answer.
#[tokio::test]
async fn cat019_get_ptt_accepts_the_documented_integer() {
    for (value, expected) in [(1_i64, true), (0_i64, false)] {
        let (client, wire) = client_for(FlrigReply::Int(value)).await;
        let ptt = client.get_ptt().await.expect("documented integer answer");
        assert_eq!(ptt, expected);
        assert!(
            wire.last_body()
                .contains("<methodName>rig.get_ptt</methodName>")
        );
        wire.shutdown().await;
    }
}

/// Response types other than the documented `<int>` are protocol errors (no implicit coercion).
#[tokio::test]
async fn cat019_get_ptt_rejects_undocumented_response_types() {
    for inner in [
        "<boolean>1</boolean>",
        "<double>1.0</double>",
        "<string>1</string>",
        "<dateTime.iso8601>20260101T00:00:00</dateTime.iso8601>",
    ] {
        let (client, wire) = client_for(FlrigReply::RawValue(inner)).await;
        let outcome = client.get_ptt().await;
        assert!(
            outcome.is_err(),
            "rig.get_ptt is documented as i:n; {inner} is an undocumented response type"
        );
        wire.shutdown().await;
    }

    // An integer outside the documented 0/1 range is not a PTT state either.
    let (client, wire) = client_for(FlrigReply::Int(2)).await;
    assert!(client.get_ptt().await.is_err(), "PTT is documented as 1/0");
    wire.shutdown().await;
}

/// HTTP errors are never a successful PTT command.
#[tokio::test]
async fn cat019_http_errors_are_rejected() {
    for (status, reason) in [
        (400_u16, "Bad Request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not Found"),
        (500, "Internal Server Error"),
    ] {
        let (client, wire) = client_for(FlrigReply::Http(
            status,
            reason,
            "<?xml version=\"1.0\"?><methodResponse><params><param></param></params></methodResponse>",
        ))
        .await;
        let outcome = client.set_ptt(true).await;
        assert!(outcome.is_err(), "HTTP {status} must fail closed");
        wire.shutdown().await;
    }
}

/// Malformed XML, an empty body, a truncated body and an oversized body all fail closed.
#[tokio::test]
async fn cat019_malformed_and_truncated_responses_fail_closed() {
    for reply in [
        FlrigReply::Malformed,
        FlrigReply::Empty,
        FlrigReply::Truncated,
        FlrigReply::Oversized,
    ] {
        let (client, wire) = client_for(reply.clone()).await;
        let outcome = client.set_ptt(true).await;
        assert!(outcome.is_err(), "{reply:?} must fail closed: {outcome:?}");
        wire.shutdown().await;
    }
}

/// A silent daemon is a client-side timeout, never a confirmed PTT.
#[tokio::test]
async fn cat019_silent_daemon_times_out_without_confirming_ptt() {
    let (client, wire) = client_for(FlrigReply::Silent).await;
    let outcome = tokio::time::timeout(Duration::from_secs(10), client.set_ptt(true)).await;
    let outcome = outcome.expect("client must respect its own timeout");
    assert!(outcome.is_err(), "no answer is a timeout, not a success");
    wire.shutdown().await;
}

/// A refused connection is an error, never a success.
#[tokio::test]
async fn cat019_connection_refused_is_an_error() {
    let port = refused_endpoint().await;
    let client = FlrigClient::new("127.0.0.1", port);
    let outcome = tokio::time::timeout(Duration::from_secs(10), client.set_ptt(true)).await;
    assert!(outcome.expect("bounded").is_err());
}

/// A SET reply is not a physical PTT confirmation: GET must be the documented source of state,
/// and a SET/GET mismatch must never be reported as a confirmed TX.
#[tokio::test]
async fn cat019_set_response_is_not_a_physical_ptt_confirmation() {
    let wire = spawn_flrig_wire(FlrigReply::Nil).await;
    let client = FlrigClient::new("127.0.0.1", wire.port());
    client.set_ptt(true).await.expect("documented set accepted");
    wire.shutdown().await;

    // A daemon that accepted SET but reports RX on GET must be believed on GET.
    let (client, wire) = client_for(FlrigReply::Int(0)).await;
    let state = client.get_ptt().await.expect("documented integer state");
    assert!(!state, "SET must never be treated as a physical TX");
    wire.shutdown().await;
}

/// A rejected OFF command is never a successful release.
#[tokio::test]
async fn cat019_off_failure_is_not_success() {
    let (client, wire) = client_for(FlrigReply::Fault(-1, "type error")).await;
    let outcome = client.set_ptt(false).await;
    assert!(outcome.is_err(), "a rejected RX command is an error");
    assert_eq!(wire.request_count(), 1, "no retry with another type");
    wire.shutdown().await;
}

/// After a documented PTT ON there is no second PTT ON with a different type (no fallback).
#[tokio::test]
async fn cat019_no_boolean_int_fallback_after_type_error() {
    let (client, wire) = client_for(FlrigReply::Fault(-1, "type error")).await;
    let _ = client.set_ptt(true).await;
    let bodies: Vec<String> = wire.requests().into_iter().map(|r| r.body).collect();
    assert_eq!(bodies.len(), 1, "exactly one PTT ON attempt");
    assert!(bodies[0].contains("<int>1</int>"));
    assert!(!bodies.iter().any(|body| body.contains("<boolean>")));
    wire.shutdown().await;
}

// ============================================================================
// PHASE 15/16 — FAULT INJECTION AND COMBINATORIAL MATRIX
// ============================================================================

/// value × documented/undocumented response × HTTP status: only the documented `nil` answer of a
/// successful `rig.set_ptt` is a success.
#[tokio::test]
async fn cat019_combinatorial_matrix() {
    let replies = [
        FlrigReply::Nil,
        FlrigReply::Int(0),
        FlrigReply::Int(1),
        FlrigReply::RawValue("<boolean>1</boolean>"),
        FlrigReply::Fault(-1, "type error"),
        FlrigReply::Malformed,
        FlrigReply::Empty,
        FlrigReply::Http(500, "Internal Server Error", "ignored"),
    ];

    for ptt in [true, false] {
        for reply in &replies {
            let (client, wire) = client_for(reply.clone()).await;
            let outcome = client.set_ptt(ptt).await;
            let expect_ok = *reply == FlrigReply::Nil;
            assert_eq!(
                outcome.is_ok(),
                expect_ok,
                "ptt={ptt} reply={reply:?} outcome={outcome:?}"
            );
            let body = wire.last_body();
            assert_eq!(body, documented_set_ptt_body(i64::from(ptt)));
            assert!(!body.contains("<boolean>"));
            wire.shutdown().await;
        }
    }
}

/// Fault injection: cancellation, reconnect after a dropped connection and cleanup.
#[tokio::test]
async fn cat019_fault_injection_and_reconnect() {
    // 1. Cancellation after the request was written: no panic, the client stays usable.
    let (client, wire) = client_for(FlrigReply::Silent).await;
    {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        let mut future = Box::pin(client.set_ptt(true));
        let _ = std::future::Future::poll(future.as_mut(), &mut context);
    }
    wire.shutdown().await;

    // 2. A dropped connection is handled by the bounded reconnect retry (same documented payload).
    let (client, wire) = client_for(FlrigReply::Truncated).await;
    assert!(client.set_ptt(true).await.is_err());
    assert!(client.set_ptt(false).await.is_err());
    let bodies: Vec<String> = wire.requests().into_iter().map(|r| r.body).collect();
    assert_eq!(bodies.len(), 2, "one request per call, no duplicate PTT ON");
    assert!(bodies[0].contains("<int>1</int>"));
    assert!(bodies[1].contains("<int>0</int>"));
    client.disconnect().await;
    wire.shutdown().await;
}

// ============================================================================
// PHASE 17 — DETERMINISTIC STRESS (documented answers)
// ============================================================================
// The timeout/disconnect stress requires a short client timeout and therefore lives in the
// `#[cfg(test)]` module of `src/cat/flrig.rs` (`FlrigClient::with_timeout`).

/// 2000 documented `rig.set_ptt` int commands (1000 ON + 1000 OFF) and 2000 documented
/// `rig.get_ptt` reads over the keep-alive connection.
#[tokio::test]
async fn cat019_stress_documented_int_on_off_2000() {
    let wire = spawn_flrig_wire(FlrigReply::Nil).await;
    let client = FlrigClient::new("127.0.0.1", wire.port());

    for iteration in 0..1000 {
        client
            .set_ptt(true)
            .await
            .unwrap_or_else(|e| panic!("[{iteration}] ON: {e}"));
        client
            .set_ptt(false)
            .await
            .unwrap_or_else(|e| panic!("[{iteration}] OFF: {e}"));
    }
    assert_eq!(wire.request_count(), 2000);
    for body in wire.requests() {
        assert!(body.body.contains("<int>0</int>") || body.body.contains("<int>1</int>"));
        assert!(!body.body.contains("<boolean>"));
    }
    wire.shutdown().await;
}

/// 1000 malformed-argument faults, 1000 XML-RPC faults and 1000 HTTP errors — all fail closed,
/// all carry the documented integer payload, none is retried with another type.
#[tokio::test]
async fn cat019_stress_faults_and_http_errors_1000_each() {
    let cases = [FlrigReply::Fault(-1, "type error"), FlrigReply::Malformed];
    for reply in cases {
        let wire = spawn_flrig_wire(reply.clone()).await;
        let client = FlrigClient::new("127.0.0.1", wire.port());
        for iteration in 0..1000 {
            let outcome = client.set_ptt(true).await;
            assert!(outcome.is_err(), "[{iteration}] {reply:?}");
        }
        let bodies: Vec<String> = wire.requests().into_iter().map(|r| r.body).collect();
        assert_eq!(bodies.len(), 1000);
        for body in bodies {
            assert!(body.contains("<int>1</int>"));
            assert!(!body.contains("<boolean>"));
        }
        wire.shutdown().await;
    }

    // HTTP errors: the client drops the connection on a protocol error and reconnects.
    let wire = spawn_flrig_wire(FlrigReply::Http(
        500,
        "Internal Server Error",
        "<?xml version=\"1.0\"?><methodResponse><params><param></param></params></methodResponse>",
    ))
    .await;
    let client = FlrigClient::new("127.0.0.1", wire.port());
    for iteration in 0..1000 {
        assert!(
            client.set_ptt(true).await.is_err(),
            "[{iteration}] HTTP 500"
        );
    }
    wire.shutdown().await;
}

/// A 1000-iteration matrix over every HTTP status class keeps failing closed.
#[tokio::test]
async fn cat019_stress_http_status_matrix() {
    for (status, reason) in [
        (400_u16, "Bad Request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not Found"),
        (500, "Internal Server Error"),
    ] {
        let wire = spawn_flrig_wire(FlrigReply::Http(status, reason, "ignored")).await;
        let client = FlrigClient::new("127.0.0.1", wire.port());
        for iteration in 0..200 {
            assert!(
                client.set_ptt(true).await.is_err(),
                "HTTP {status} iteration {iteration}"
            );
        }
        wire.shutdown().await;
    }
}
