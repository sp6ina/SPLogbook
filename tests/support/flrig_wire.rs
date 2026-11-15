// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! TEST-ONLY FLRig 2.0.00 XML-RPC *wire* double (STAGE-CAT-018-019-SOFTWARE).
//!
//! This module lives **only** in `tests/support/` and is included explicitly with
//! `#[path = "support/flrig_wire.rs"]`. It is never part of the production library, it is not
//! reachable from the GUI, the CLI, the configuration or the environment, and it never appears
//! in a release artifact.
//!
//! The double is a loopback `127.0.0.1:0` HTTP/1.1 XML-RPC server that records the **exact bytes**
//! the real production [`splogbook::cat::flrig::FlrigClient`] writes and answers with the response
//! shapes produced by FLRig 2.0.00 `src/xmlrpcpp/XmlRpcServer.cpp` (`generateResponse`,
//! `generateFaultResponse`) for the documented method signatures in
//! `src/server/xml_server.cxx` (`rig.set_ptt` = `n:i`, `rig.get_ptt` = `i:n`):
//!
//! * `n:` methods that never assign a result answer with a `<param>` that carries **no `<value>`**
//!   (the documented *nil* response);
//! * `i:` methods answer with `<value><int>…</int></value>`;
//! * a rejected argument type produces the documented fault (`XmlRpcException("type error")`,
//!   default code −1).
//!
//! No public network, no real FLRig daemon, no transceiver, no PTT and no RF is involved.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One recorded HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedRequest {
    /// Request line plus headers, exactly as received (without the final blank line).
    pub head: String,
    /// XML-RPC request body, exactly as received.
    pub body: String,
}

/// How the simulated FLRig daemon answers one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlrigReply {
    /// Documented response of a method that returns nothing (`rig.set_ptt` = `n:i`).
    Nil,
    /// Documented `<value><int>V</int></value>` response (`rig.get_ptt` = `i:n`).
    Int(i64),
    /// A documented `<value>` element with arbitrary inner XML (used to test type rejection).
    RawValue(&'static str),
    /// Documented XML-RPC fault (`XmlRpcException`, default code −1).
    Fault(i64, &'static str),
    /// Explicit HTTP status with an XML body.
    Http(u16, &'static str, &'static str),
    /// Malformed XML body.
    Malformed,
    /// `Content-Length: 0`.
    Empty,
    /// Accept the request and never answer (client-side timeout).
    Silent,
    /// Advertise a longer body than is written, then close (truncated body).
    Truncated,
    /// Advertise an oversized body (`Content-Length` above the production limit).
    Oversized,
}

/// Handle to a running FLRig wire double.
pub struct FlrigWire {
    host: String,
    port: u16,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: tokio::task::JoinHandle<()>,
}

impl FlrigWire {
    /// Documented endpoint (`host:port`) of the wire double.
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Every recorded request, in order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Number of recorded requests.
    pub fn request_count(&self) -> usize {
        self.requests.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// The XML-RPC body of the last recorded request (empty when nothing was received).
    pub fn last_body(&self) -> String {
        self.requests()
            .last()
            .map(|r| r.body.clone())
            .unwrap_or_default()
    }

    /// The HTTP head of the last recorded request.
    pub fn last_head(&self) -> String {
        self.requests()
            .last()
            .map(|r| r.head.clone())
            .unwrap_or_default()
    }

    /// Stops the simulated daemon.
    pub async fn shutdown(self) {
        self.handle.abort();
    }
}

/// The documented *nil* body emitted by `XmlRpcServer::generateResponse("")` for `rig.set_ptt`.
pub const DOCUMENTED_NIL_BODY: &str = "<?xml version=\"1.0\"?>\r\n<methodResponse><params><param>\r\n\t\r\n</param></params></methodResponse>\r\n";

/// The documented successful `<int>` body (`rig.get_ptt` assigns `result = int(PTT)`).
pub fn documented_int_body(value: i64) -> String {
    format!(
        "<?xml version=\"1.0\"?>\r\n<methodResponse><params><param>\r\n\t<value><int>{value}</int></value>\r\n</param></params></methodResponse>\r\n"
    )
}

/// The documented fault body (`generateFaultResponse`: `faultCode` + `faultString`).
pub fn documented_fault_body(code: i64, message: &str) -> String {
    format!(
        "<?xml version=\"1.0\"?>\r\n<methodResponse><fault>\r\n\t<value><struct><member><name>faultCode</name><value><int>{code}</int></value></member><member><name>faultString</name><value><string>{message}</string></value></member></struct></value>\r\n</fault></methodResponse>\r\n"
    )
}

fn build_response(reply: &FlrigReply) -> Option<String> {
    let body = match reply {
        FlrigReply::Nil => DOCUMENTED_NIL_BODY.to_string(),
        FlrigReply::Int(v) => documented_int_body(*v),
        FlrigReply::RawValue(inner) => format!(
            "<?xml version=\"1.0\"?>\r\n<methodResponse><params><param>\r\n\t<value>{inner}</value>\r\n</param></params></methodResponse>\r\n"
        ),
        FlrigReply::Fault(code, message) => documented_fault_body(*code, message),
        FlrigReply::Http(_, _, body) => (*body).to_string(),
        FlrigReply::Malformed => "<methodResponse><params><param><value><int>1</int>".to_string(),
        FlrigReply::Empty => String::new(),
        FlrigReply::Silent => return None,
        FlrigReply::Truncated => {
            let partial = "<?xml version=\"1.0\"?><methodResponse><params>";
            let head = format!(
                "HTTP/1.1 200 OK\r\nServer: XMLRPC\r\nContent-Type: text/xml\r\nContent-length: {}\r\n\r\n",
                partial.len() + 64
            );
            return Some(format!("{head}{partial}"));
        }
        FlrigReply::Oversized => {
            let head = "HTTP/1.1 200 OK\r\nServer: XMLRPC\r\nContent-Type: text/xml\r\nContent-length: 70000\r\n\r\n"
                .to_string();
            return Some(format!("{head}{DOCUMENTED_NIL_BODY}"));
        }
    };
    Some(format!(
        "HTTP/1.1 200 OK\r\nServer: XMLRPC\r\nContent-Type: text/xml\r\nContent-length: {}\r\n\r\n{}",
        body.len(),
        body
    ))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn content_length(head: &str) -> Option<usize> {
    for line in head.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            return value.trim().parse::<usize>().ok();
        }
    }
    None
}

async fn serve_connection(
    mut socket: TcpStream,
    recorder: Arc<Mutex<Vec<RecordedRequest>>>,
    reply: FlrigReply,
) {
    let mut pending: Vec<u8> = Vec::new();

    loop {
        // Read the HTTP head.
        let header_end = loop {
            if let Some(position) = find(&pending, b"\r\n\r\n") {
                break position;
            }
            let mut tmp = [0u8; 2048];
            match socket.read(&mut tmp).await {
                Ok(0) | Err(_) => return,
                Ok(n) => pending.extend_from_slice(&tmp[..n]),
            }
            if pending.len() > 16_384 {
                return;
            }
        };

        let head = String::from_utf8_lossy(&pending[..header_end]).to_string();
        let body_start = header_end + 4;
        let length = content_length(&head).unwrap_or(0);

        while pending.len() < body_start + length {
            let mut tmp = [0u8; 2048];
            match socket.read(&mut tmp).await {
                Ok(0) | Err(_) => return,
                Ok(n) => pending.extend_from_slice(&tmp[..n]),
            }
        }

        let body = String::from_utf8_lossy(&pending[body_start..body_start + length]).to_string();
        pending.drain(..body_start + length);
        if let Ok(mut recorded) = recorder.lock() {
            recorded.push(RecordedRequest { head, body });
        }

        match &reply {
            FlrigReply::Silent => {
                // Keep the connection open without answering: the client must time out.
                continue;
            }
            FlrigReply::Truncated => {
                if let Some(response) = build_response(&reply) {
                    let _ = socket.write_all(response.as_bytes()).await;
                }
                let _ = socket.shutdown().await;
                return;
            }
            FlrigReply::Http(status, reason, body) => {
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nServer: XMLRPC\r\nContent-Type: text/xml\r\nContent-length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                if socket.write_all(response.as_bytes()).await.is_err() {
                    return;
                }
            }
            _ => {
                if let Some(response) = build_response(&reply)
                    && socket.write_all(response.as_bytes()).await.is_err()
                {
                    return;
                }
            }
        }
    }
}

/// Spawns the simulated FLRig daemon on an ephemeral loopback port.
pub async fn spawn_flrig_wire(reply: FlrigReply) -> FlrigWire {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test-owned loopback listener");
    let port = listener.local_addr().expect("local address").port();
    let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&requests);

    let handle = tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let recorder = Arc::clone(&recorder);
            let reply = reply.clone();
            serve_connection(socket, recorder, reply).await;
        }
    });

    FlrigWire {
        host: "127.0.0.1".to_string(),
        port,
        requests,
        handle,
    }
}

/// A loopback port with no listener, for "connection refused" tests.
pub async fn refused_endpoint() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test-owned loopback listener");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    port
}
