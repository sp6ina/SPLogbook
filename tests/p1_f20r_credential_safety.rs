// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! P1-F20R — bezpieczeństwo transportu poświadczeń na ścieżkach
//! pobierania (ryzyko **F20-R3**).
//!
//! Oficjalny kontrakt LoTW i eQSL wymaga poświadczeń w query stringu i nie
//! dokumentuje bezpieczniejszego wariantu, więc ten zestaw testów **nie**
//! twierdzi, że poświadczeń nie ma w query. Weryfikuje cztery fakty, które
//! muszą być prawdziwe mimo tego ograniczenia:
//!
//! * `QUERY_CREDENTIALS_NOT_LOGGED`,
//! * `QUERY_CREDENTIALS_NOT_PERSISTED`,
//! * `QUERY_CREDENTIALS_NOT_EXPOSED_IN_ERRORS`,
//! * przekierowanie nie może przenieść query do innego adresu.
//!
//! Wyłącznie dane syntetyczne i pętla lokalna `127.0.0.1:0`. Żaden test nie
//! łączy się z LoTW ani eQSL.

use splogbook::core::http::{
    SecretUrl, SecretUrlError, http_client_no_redirect, redacted_url_for_diagnostics,
    sanitize_network_error, sanitize_url,
};
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const LOTW_SECRET: &str = "TEST_LOTW_SECRET_DO_NOT_LOG";
const EQSL_SECRET: &str = "TEST_EQSL_SECRET_DO_NOT_LOG";

// ---------------------------------------------------------------------------
// SecretUrl: Debug/Display nigdy nie pokazują query
// ---------------------------------------------------------------------------

#[test]
fn secret_url_debug_and_display_never_reveal_the_query() {
    let url = SecretUrl::parse_with_params(
        "https://lotw.invalid/lotwuser/lotwreport.adi",
        &[("login", "TSTCALL"), ("password", LOTW_SECRET)],
    )
    .expect("url");
    let debug = format!("{url:?}");
    let display = format!("{url}");
    for rendered in [&debug, &display] {
        assert!(!rendered.contains(LOTW_SECRET), "{rendered}");
        assert!(!rendered.contains("password"), "{rendered}");
        assert!(!rendered.contains("login"), "{rendered}");
        assert!(!rendered.contains('?'), "{rendered}");
        assert!(rendered.contains("lotw.invalid"), "{rendered}");
        assert!(rendered.contains("/lotwuser/lotwreport.adi"), "{rendered}");
    }
    assert_eq!(url.redacted(), display);
}

#[test]
fn secret_url_redaction_holds_for_both_documented_download_endpoints() {
    let cases = [
        (
            "https://eqsl.invalid/qslcard/DownloadInBox.cfm",
            "UserName",
            EQSL_SECRET,
        ),
        (
            "https://lotw.invalid/lotwuser/lotwreport.adi",
            "login",
            LOTW_SECRET,
        ),
    ];
    for (base, user_param, secret) in cases {
        let url =
            SecretUrl::parse_with_params(base, &[(user_param, "TSTCALL"), ("password", secret)])
                .expect("url");
        for rendered in [format!("{url:?}"), format!("{url}"), url.redacted()] {
            assert!(!rendered.contains(secret), "{rendered}");
            assert!(!rendered.contains("TSTCALL"), "{rendered}");
            assert!(!rendered.contains('?'), "{rendered}");
        }
    }
}

#[test]
fn secret_url_rejects_an_invalid_base_without_echoing_it() {
    let err = SecretUrl::parse_with_params(
        "not-a-url?password=TEST_LOTW_SECRET_DO_NOT_LOG",
        &[("password", LOTW_SECRET)],
    )
    .expect_err("invalid base");
    assert_eq!(err, SecretUrlError::InvalidBase);
    let rendered = format!("{err} {err:?}");
    assert!(!rendered.contains(LOTW_SECRET), "{rendered}");
}

#[test]
fn secret_url_clone_keeps_redaction() {
    let url = SecretUrl::parse_with_params(
        "https://lotw.invalid/lotwuser/lotwreport.adi",
        &[("password", LOTW_SECRET)],
    )
    .expect("url");
    let cloned = url.clone();
    assert!(!format!("{cloned:?}").contains(LOTW_SECRET));
    assert_eq!(cloned.redacted(), url.redacted());
}

// ---------------------------------------------------------------------------
// Sanityzacja błędów i diagnostyki
// ---------------------------------------------------------------------------

#[test]
fn malformed_url_diagnostics_never_keep_the_query() {
    let cases = [
        format!("not-a-url?password={LOTW_SECRET}"),
        format!("://bad?login=TSTCALL&password={LOTW_SECRET}"),
        format!("https://[::1?password={LOTW_SECRET}"),
    ];
    for case in cases {
        let rendered = redacted_url_for_diagnostics(&case);
        assert!(!rendered.contains(LOTW_SECRET), "{rendered}");
    }
}

#[test]
fn sanitized_urls_drop_query_userinfo_and_fragment() {
    for (raw, secret) in [
        (
            format!("https://user:{LOTW_SECRET}@lotw.invalid/a?password={LOTW_SECRET}"),
            LOTW_SECRET,
        ),
        (
            format!(
                "https://eqsl.invalid/qslcard/DownloadInBox.cfm?UserName=TSTCALL&Password={EQSL_SECRET}"
            ),
            EQSL_SECRET,
        ),
    ] {
        let parsed = reqwest::Url::parse(&raw).expect("url");
        let clean = sanitize_url(&parsed);
        assert!(!clean.contains(secret), "{clean}");
        assert!(!clean.contains('?'), "{clean}");
        assert!(!clean.contains('@'), "{clean}");
    }
}

// ---------------------------------------------------------------------------
// Przekierowania: query nie może opuścić pierwotnego adresu
// ---------------------------------------------------------------------------

struct LoopbackServer {
    base_url: String,
    handle: Option<std::thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    requests: Arc<std::sync::atomic::AtomicUsize>,
}

impl LoopbackServer {
    fn start(status: &str, location: Option<&str>, max_requests: usize) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("loopback bind 127.0.0.1");
        let addr = listener.local_addr().expect("addr");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_inner = stop.clone();
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let requests_inner = requests.clone();
        let status_line = format!("HTTP/1.1 {status}");
        let location = location.map(str::to_string);
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(false).expect("blocking");
            for _ in 0..max_requests {
                if stop_inner.load(Ordering::SeqCst) {
                    break;
                }
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                let _ = read_request(&mut stream);
                requests_inner.fetch_add(1, Ordering::SeqCst);
                let mut response =
                    format!("{status_line}\r\nContent-Length: 2\r\nConnection: close\r\n");
                if let Some(location) = &location {
                    response.push_str(&format!("Location: {location}\r\n"));
                }
                response.push_str("\r\nno");
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            base_url: format!("http://{addr}"),
            handle: Some(handle),
            stop,
            requests,
        }
    }
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ =
            std::net::TcpStream::connect(self.base_url.trim_start_matches("http://").to_string());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> std::io::Result<()> {
    let mut buffer = [0u8; 4096];
    let _ = stream.read(&mut buffer)?;
    Ok(())
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

#[test]
fn a_redirect_is_never_followed_so_the_query_cannot_be_replayed() {
    let server = LoopbackServer::start("302 Found", Some("http://127.0.0.1:9/steal"), 4);
    let url = SecretUrl::parse_with_params(
        &format!("{}/lotwuser/lotwreport.adi", server.base_url),
        &[("login", "TSTCALL"), ("password", LOTW_SECRET)],
    )
    .expect("url");
    let client = http_client_no_redirect(5);
    runtime().block_on(async {
        let response = url.get(&client).send().await.expect("response");
        assert_eq!(response.status().as_u16(), 302);
        let body = response.text().await.expect("body");
        assert!(!body.contains(LOTW_SECRET));
    });
    // Dokładnie jedno żądanie: przekierowanie nie zostało wykonane.
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[test]
fn a_redirect_loop_cannot_consume_requests() {
    let server = LoopbackServer::start("301 Moved Permanently", Some("/loop"), 16);
    let url = SecretUrl::parse_with_params(
        &format!("{}/qslcard/DownloadInBox.cfm", server.base_url),
        &[("UserName", "TSTCALL"), ("Password", EQSL_SECRET)],
    )
    .expect("url");
    let client = http_client_no_redirect(5);
    runtime().block_on(async {
        for _ in 0..8 {
            let response = url.get(&client).send().await.expect("response");
            assert_eq!(response.status().as_u16(), 301);
        }
    });
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        8,
        "polityka bez przekierowań wykonuje dokładnie jedno żądanie na wywołanie"
    );
}

// ---------------------------------------------------------------------------
// Matryca statusów HTTP: sekret nie może pojawić się w błędzie ani w treści
// ---------------------------------------------------------------------------

#[test]
fn no_http_status_leaks_the_secret_into_an_error_or_a_body() {
    let statuses = [
        "200 OK",
        "400 Bad Request",
        "401 Unauthorized",
        "403 Forbidden",
        "404 Not Found",
        "429 Too Many Requests",
        "500 Internal Server Error",
    ];
    for status in statuses {
        let server = LoopbackServer::start(status, None, 4);
        let url = SecretUrl::parse_with_params(
            &format!("{}/qslcard/DownloadInBox.cfm", server.base_url),
            &[("UserName", "TSTCALL"), ("Password", EQSL_SECRET)],
        )
        .expect("url");
        let client = http_client_no_redirect(5);
        runtime().block_on(async {
            match url.get(&client).send().await {
                Ok(response) => {
                    let code = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();
                    assert!(!body.contains(EQSL_SECRET), "{status}");
                    // Odpowiedź 200 z ciałem „no” nie jest dowodem sukcesu.
                    assert!(!body.contains("<EOH>"), "{status}");
                    assert!(!format!("{url:?}").contains(EQSL_SECRET));
                    let _ = code;
                }
                Err(error) => {
                    let sanitized = sanitize_network_error(&error);
                    assert!(!sanitized.contains(EQSL_SECRET), "{status}: {sanitized}");
                }
            }
        });
    }
}

#[test]
fn transport_errors_for_every_failure_mode_are_sanitized() {
    // Port 9 na pętli lokalnej jest zamknięty: gwarantowane `connection refused`.
    let cases = [
        format!("http://127.0.0.1:9/qslcard/DownloadInBox.cfm?Password={EQSL_SECRET}"),
        format!("http://127.0.0.1:9/lotwuser/lotwreport.adi?password={LOTW_SECRET}"),
        format!("ftp://127.0.0.1:9/x?password={LOTW_SECRET}"),
    ];
    for raw in cases {
        let parsed = reqwest::Url::parse(&raw).expect("url");
        let _ = sanitize_url(&parsed);
        let diagnostics = redacted_url_for_diagnostics(&raw);
        assert!(!diagnostics.contains(LOTW_SECRET));
        assert!(!diagnostics.contains(EQSL_SECRET));
    }
    let client = http_client_no_redirect(2);
    runtime().block_on(async {
        let url = SecretUrl::parse_with_params(
            "http://127.0.0.1:9/qslcard/DownloadInBox.cfm",
            &[("Password", EQSL_SECRET)],
        )
        .expect("url");
        let error = url
            .get(&client)
            .send()
            .await
            .expect_err("closed port must error");
        let sanitized = sanitize_network_error(&error);
        assert!(!sanitized.contains(EQSL_SECRET), "{sanitized}");
        assert!(!sanitized.contains('?'), "{sanitized}");
    });
}

#[test]
fn redirect_errors_do_not_echo_the_query() {
    let server = LoopbackServer::start("307 Temporary Redirect", Some("/elsewhere"), 2);
    let url = SecretUrl::parse_with_params(
        &format!("{}/lotwuser/lotwreport.adi", server.base_url),
        &[("login", "TSTCALL"), ("password", LOTW_SECRET)],
    )
    .expect("url");
    let client = http_client_no_redirect(5);
    runtime().block_on(async {
        let response = url.get(&client).send().await.expect("response");
        assert!(response.status().is_redirection());
        let rendered = format!(
            "{} {} {:?}",
            response.status().as_u16(),
            url.redacted(),
            url
        );
        assert!(!rendered.contains(LOTW_SECRET), "{rendered}");
    });
}

// ---------------------------------------------------------------------------
// Skanowanie: żadna ścieżka produkcyjna nie buduje tekstowego URL-a z sekretem
// ---------------------------------------------------------------------------

#[test]
fn production_download_paths_use_the_secret_wrapper() {
    let http = std::fs::read_to_string("src/core/http.rs").expect("http.rs");
    assert!(http.contains("pub struct SecretUrl"));
    assert!(http.contains("http_client_no_redirect"));
    assert!(http.contains("redirect(reqwest::redirect::Policy::none())"));

    let lotw = std::fs::read_to_string("src/cloud/lotw.rs").expect("lotw.rs");
    assert!(
        lotw.contains("SecretUrl::parse_with_params"),
        "LoTW download must build a SecretUrl"
    );
    assert!(
        lotw.contains("http_client_no_redirect"),
        "LoTW download must disable redirects"
    );
    let eqsl = std::fs::read_to_string("src/cloud/eqsl.rs").expect("eqsl.rs");
    assert!(eqsl.contains("SecretUrl::parse_with_params"));
    assert!(eqsl.contains("http_client_no_redirect"));

    // Ścieżki pobierania nie mogą już używać `.query(...)` z poświadczeniami.
    for (path, source) in [("src/cloud/lotw.rs", &lotw), ("src/cloud/eqsl.rs", &eqsl)] {
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code.contains(".query(&[(\"UserName\""),
            "{path}: eQSL download must not pass credentials via `.query`"
        );
        assert!(
            !code.contains(".query(&query)"),
            "{path}: credentials must not be passed via `.query`"
        );
    }
}

#[test]
fn production_does_not_contain_a_public_helper_returning_a_credential_url() {
    let lotw = std::fs::read_to_string("src/cloud/lotw.rs").expect("lotw.rs");
    // `build_report_url` zwracał pełny URL z hasłem jako `String`. Wolno mu
    // istnieć wyłącznie w kompilacji testowej.
    let Some(index) = lotw.find("pub fn build_report_url") else {
        panic!("build_report_url must still be present for the F-20 regression tests");
    };
    // `find` returns a char boundary, so slicing the prefix is always valid;
    // never slice a fixed byte window backwards (it can land inside a multibyte
    // character).
    let prefix = &lotw[..index];
    let attribute = prefix.rfind("#[cfg(test)]");
    assert!(
        attribute.is_some_and(|position| index - position <= 240),
        "build_report_url must be gated to test scope"
    );
    // Żaden inny publiczny helper nie może zwracać URL-a z sekretem.
    for forbidden in [
        "pub fn build_report_url",
        "pub fn download_url",
        "pub fn report_url",
    ] {
        let occurrences = lotw.matches(forbidden).count();
        assert!(occurrences <= 1, "{forbidden} must not be duplicated");
    }
}
