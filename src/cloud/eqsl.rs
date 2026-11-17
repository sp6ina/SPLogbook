// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Klient eQSL.cc.
//!
//! Kontrakt uploadu oparty wyłącznie na oficjalnej dokumentacji eQSL
//! (`https://www.eqsl.cc/qslcard/ImportADIF.txt`, wersja z 01.03.2026):
//!
//!  * procesor to `https://www.eQSL.cc/qslcard/ImportADIF.cfm`, wywoływany
//!    metodą **FORM POST**,
//!  * ładunek ADIF jest przekazywany polem `ADIFData`,
//!  * poświadczenia są przekazywane polami formularza `EQSL_USER`/`EQSL_PSWD`
//!    (dokumentacja: „do NOT encapsulate them in ADIF style tags!”),
//!  * odpowiedź to strona HTML oznaczona komentarzem
//!    `<!-- Reply form eQSL.cc ADIF Real-time Interface -->`,
//!  * **sukces jest tekstem w treści**, nie kodem HTTP:
//!    `Result: x out of y records added`, gdzie `x == y` oznacza pełną akceptację,
//!  * `Warning:` oznacza rekord, który **nie został** zaimportowany,
//!  * `Error:` jest błędem krytycznym, a **każdy status HTTP inny niż 200** także,
//!  * od 02.03.2026 ładunek bez co najmniej jednego `<EOR>` kończy program
//!    **bez żadnego komunikatu** — brak linii `Result:` nie jest sukcesem.
//!
//! Dlatego ten moduł nigdy nie uznaje odpowiedzi za sukces bez jawnego dowodu
//! (`Result:`), a nieznaną odpowiedź klasyfikuje jako niejednoznaczną.

use reqwest::Client;

/// Oficjalny endpoint procesora `ImportADIF` (dokumentacja eQSL).
pub const EQSL_IMPORT_ENDPOINT: &str = "https://www.eqsl.cc/qslcard/ImportADIF.cfm";

/// Komentarz identyfikujący stronę odpowiedzi procesora.
pub const EQSL_REPLY_MARKER: &str = "Reply form eQSL.cc ADIF Real-time Interface";

/// Wynik pojedynczego wywołania `ImportADIF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqslSubmission {
    pub http_status: u16,
    pub body: String,
}

impl EqslSubmission {
    /// Krótki, zredagowany fragment treści do komunikatów i logów.
    pub fn body_excerpt(&self) -> String {
        let trimmed = self.body.trim();
        let mut excerpt: String = trimmed.chars().take(300).collect();
        if trimmed.chars().count() > 300 {
            excerpt.push('…');
        }
        excerpt
    }
}

/// Błąd transportu przy wysyłce do eQSL (bez treści żądania i bez sekretów).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EqslSubmitError {
    /// Przekroczono limit czasu — wynik jest niejednoznaczny.
    Timeout,
    /// Błąd połączenia/transportu.
    Transport(String),
}

impl std::fmt::Display for EqslSubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "eQSL: przekroczono limit czasu połączenia"),
            Self::Transport(detail) => write!(f, "eQSL: błąd połączenia: {detail}"),
        }
    }
}

impl std::error::Error for EqslSubmitError {}

/// Klasyfikacja odpowiedzi `ImportADIF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EqslClassification {
    /// Odpowiedź zawiera `Result: x out of y records added`.
    RecordsResult { added: usize, total: usize },
    /// Błąd krytyczny, po którym nic nie zostało zaimportowane.
    FatalNothingImported { detail: String },
    /// Błąd krytyczny, po którym stan importu jest niejednoznaczny
    /// (m.in. błędy zatrzymujące przetwarzanie na konkretnym rekordzie).
    FatalAmbiguous { detail: String },
    /// Brak `Result:`, ale obecne są ostrzeżenia o rekordach niezaimportowanych.
    WarningsOnly { rejected: usize, detail: String },
    /// Brak jakiegokolwiek dowodu (`Result:`), w tym ciche zakończenie przy
    /// braku `<EOR>`.
    NoResult { detail: String },
}

impl std::fmt::Display for EqslClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordsResult { added, total } => {
                write!(f, "eQSL: dodano {added} z {total} rekordów")
            }
            Self::FatalNothingImported { detail } => {
                write!(f, "eQSL: błąd krytyczny (nic nie zaimportowano): {detail}")
            }
            Self::FatalAmbiguous { detail } => {
                write!(
                    f,
                    "eQSL: błąd krytyczny o niejednoznacznym stanie importu: {detail}"
                )
            }
            Self::WarningsOnly { rejected, detail } => {
                write!(f, "eQSL: {rejected} rekordów niezaimportowanych: {detail}")
            }
            Self::NoResult { detail } => write!(f, "eQSL: brak wyniku importu: {detail}"),
        }
    }
}

/// Poświadczenia eQSL do wysyłki ADIF.
pub struct EqslClient {
    client: Client,
    username: String,
    password: String,
    /// Nadpisanie endpointu wyłącznie w zakresie testów (`#[cfg(test)]`).
    /// W kompilacji produkcyjnej pole nie istnieje.
    #[cfg(test)]
    endpoint: Option<String>,
}

impl EqslClient {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = crate::core::http::http_client_with_timeout(15);
        Self {
            client,
            username: username.into(),
            password: password.into(),
            #[cfg(test)]
            endpoint: None,
        }
    }

    /// Endpoint wysyłki. W produkcji zawsze oficjalny procesor eQSL.
    ///
    /// `self` jest używane wyłącznie w kompilacji testowej (nadpisanie
    /// endpointu jest dostępne tylko pod `#[cfg(test)]`).
    #[cfg_attr(not(test), allow(clippy::unused_self))]
    fn upload_endpoint(&self) -> &str {
        #[cfg(test)]
        {
            if let Some(endpoint) = self.endpoint.as_deref() {
                return endpoint;
            }
        }
        EQSL_IMPORT_ENDPOINT
    }

    /// Nadpisuje endpoint wyłącznie w testach (serwer loopback).
    #[cfg(test)]
    pub fn with_test_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Wysyła jeden pakiet ADIF do procesora eQSL.
    ///
    /// Świadomie **bez automatycznego ponawiania**: wynik niejednoznaczny
    /// (timeout, zerwane połączenie) nie może być zamieniony w sukces przez retry.
    /// Treść odpowiedzi jest redagowana z hasła, zanim opuści tę funkcję.
    pub async fn upload_adif(&self, adif_content: &str) -> Result<EqslSubmission, EqslSubmitError> {
        let response = self
            .client
            .post(self.upload_endpoint())
            .form(&[
                ("EQSL_USER", self.username.as_str()),
                ("EQSL_PSWD", self.password.as_str()),
                ("ADIFData", adif_content),
            ])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    EqslSubmitError::Timeout
                } else {
                    EqslSubmitError::Transport(crate::core::http::sanitize_network_error(&e))
                }
            })?;

        let http_status = response.status().as_u16();
        let body = response.text().await.map_err(|e| {
            if e.is_timeout() {
                EqslSubmitError::Timeout
            } else {
                EqslSubmitError::Transport(crate::core::http::sanitize_network_error(&e))
            }
        })?;

        Ok(EqslSubmission {
            http_status,
            body: redact_secret(&body, &self.password),
        })
    }

    /// Pobiera skrzynkę odbiorczą potwierdzonych łączności z serwisu eQSL.cc.
    ///
    /// Oficjalna dokumentacja (`https://www.eqsl.cc/qslcard/DownloadInBox.txt`,
    /// wersja z 12.10.2025) opisuje ten interfejs **wyłącznie** jako wywołanie
    /// z parametrami `UserName` i `Password` w URL. POST nie jest udokumentowany,
    /// a sesja/cookie są wzmiankowane bez żadnego mechanizmu (brak adresu
    /// logowania, nazwy cookie i czasu życia tokenu). Zmiana metody byłaby
    /// wymyśleniem protokołu, dlatego:
    ///
    ///  * adres powstaje jako [`crate::core::http::SecretUrl`] i nigdy nie jest zamieniany na tekst,
    ///  * klient ma **wyłączone przekierowania**, więc query nie może zostać
    ///    przeniesiony do innego adresu,
    ///  * błąd transportu jest sanowany, a treść odpowiedzi jest redagowana
    ///    z hasła, zanim opuści tę funkcję.
    pub async fn download_inbox_adif(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use crate::core::http::{SecretUrl, http_client_no_redirect};
        use reqwest::StatusCode;

        let client = http_client_no_redirect(30);
        let secret_url = SecretUrl::parse_with_params(
            "https://www.eqsl.cc/qslcard/DownloadInBox.cfm",
            &[
                ("UserName", self.username.as_str()),
                ("Password", self.password.as_str()),
            ],
        )?;

        let resp = secret_url.get(&client).send().await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                crate::core::http::sanitize_network_error(&e).into()
            },
        )?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            format!(
                "eQSL: błąd odczytu odpowiedzi ({}): {}",
                status.as_u16(),
                crate::core::http::sanitize_network_error(&e)
            )
        })?;
        let body = redact_secret(&body, &self.password);

        // Oficjalna dokumentacja: HTTP 200 jest zwracane także dla strony
        // „You are not yet logged in”, więc status HTTP nie jest dowodem.
        if status != StatusCode::OK {
            return Err(format!(
                "eQSL: pobieranie skrzynki odrzucone (HTTP {})",
                status.as_u16()
            )
            .into());
        }
        if body.contains("<EOH>")
            || body.contains("<eoh>")
            || body.contains("<CALL:")
            || body.contains("<call:")
        {
            Ok(body)
        } else {
            Err(body.into())
        }
    }
}

/// Usuwa wszystkie wystąpienia sekretu z tekstu (także pojedynczo znakowanych).
pub fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "<redacted>")
}

/// Klasyfikuje odpowiedź procesora `ImportADIF`.
///
/// Reguły pochodzą wyłącznie z oficjalnej dokumentacji eQSL:
/// 1. status HTTP inny niż 200 jest błędem krytycznym,
/// 2. `Result: x out of y records added` jest jedynym dowodem importu,
/// 3. `Error:` jest błędem krytycznym ( część z nich gwarantuje brak importu,
///    część zatrzymuje przetwarzanie na rekordzie — te są niejednoznaczne),
/// 4. `Warning:` oznacza rekord niezaimportowany,
/// 5. brak `Result:` (np. ciche zakończenie przy braku `<EOR>`) nie jest sukcesem.
pub fn classify_eqsl_response(http_status: u16, body: &str) -> EqslClassification {
    if http_status != 200 {
        return EqslClassification::FatalNothingImported {
            detail: format!("HTTP {http_status}"),
        };
    }

    if let Some((added, total)) = parse_records_result(body) {
        return EqslClassification::RecordsResult { added, total };
    }

    let lower = body.to_ascii_lowercase();
    if lower.contains("error:") {
        let detail = first_line_containing(body, "error:").unwrap_or_else(|| "Error".to_string());
        if is_error_that_imports_nothing(&lower) {
            return EqslClassification::FatalNothingImported { detail };
        }
        return EqslClassification::FatalAmbiguous { detail };
    }

    let warnings = count_occurrences(&lower, "warning:");
    if warnings > 0 {
        return EqslClassification::WarningsOnly {
            rejected: warnings,
            detail: first_line_containing(body, "warning:")
                .unwrap_or_else(|| "Warning".to_string()),
        };
    }

    let detail = if body.contains(EQSL_REPLY_MARKER) {
        "strona odpowiedzi eQSL bez linii `Result:`".to_string()
    } else {
        "nierozpoznana odpowiedź eQSL".to_string()
    };
    EqslClassification::NoResult { detail }
}

/// Wylicza wynik paczki na podstawie klasyfikacji odpowiedzi i liczby rekordów,
/// które faktycznie wysłano.
///
/// Kluczowy niezmiennik: `Accepted` wymaga, aby usługa zaraportowała
/// `Result: n out of n records added` **dla dokładnie tej liczby rekordów,
/// którą wysłano**. Rozbieżność liczby rekordów oznacza, że ładunek nie dotarł
/// w całości i nigdy nie jest traktowana jako sukces.
pub fn eqsl_batch_outcome(
    classification: &EqslClassification,
    expected_records: usize,
) -> crate::cloud::upload_plan::BatchOutcome {
    use crate::cloud::upload_plan::BatchOutcome;

    match classification {
        EqslClassification::RecordsResult { added, total } => {
            if *total != expected_records {
                return BatchOutcome::Failed {
                    detail: format!(
                        "eQSL zaraportował {total} rekordów, wysłano {expected_records}"
                    ),
                };
            }
            if *added == expected_records && expected_records > 0 {
                BatchOutcome::Accepted
            } else if *added == 0 {
                BatchOutcome::NoRecordsProcessed {
                    detail: "eQSL: 0 rekordów dodanych".to_string(),
                }
            } else {
                BatchOutcome::PartiallyAccepted {
                    rejected: expected_records - added,
                    detail: format!("eQSL: dodano {added} z {total}"),
                }
            }
        }
        EqslClassification::FatalNothingImported { detail } => BatchOutcome::Rejected {
            detail: detail.clone(),
        },
        EqslClassification::FatalAmbiguous { detail } | EqslClassification::NoResult { detail } => {
            BatchOutcome::Unknown {
                detail: detail.clone(),
            }
        }
        EqslClassification::WarningsOnly { rejected, detail } => BatchOutcome::PartiallyAccepted {
            rejected: (*rejected).min(expected_records),
            detail: detail.clone(),
        },
    }
}

/// Wyciąga `(added, total)` z linii `Result: x out of y records added`.
pub fn parse_records_result(body: &str) -> Option<(usize, usize)> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find("result:")? + "result:".len();
    let tail = &lower[start..];
    let out_pos = tail.find("out of")?;
    let added = digits_only(&tail[..out_pos]).parse::<usize>().ok()?;
    let after = &tail[out_pos + "out of".len()..];
    let record_pos = after.find("record")?;
    let total = digits_only(&after[..record_pos]).parse::<usize>().ok()?;
    Some((added, total))
}

fn digits_only(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn count_occurrences(haystack_lower: &str, needle_lower: &str) -> usize {
    haystack_lower.matches(needle_lower).count()
}

fn first_line_containing(body: &str, needle_lower: &str) -> Option<String> {
    body.lines()
        .find(|line| line.to_ascii_lowercase().contains(needle_lower))
        .map(|line| {
            let trimmed = line.trim();
            let mut out: String = trimmed.chars().take(160).collect();
            if trimmed.chars().count() > 160 {
                out.push('…');
            }
            out
        })
}

/// Błędy, po których dokumentacja gwarantuje, że **nic** nie zostało zaimportowane.
fn is_error_that_imports_nothing(body_lower: &str) -> bool {
    const NOTHING_IMPORTED: &[&str] = &[
        "error: file not saved",
        "error: the system is down",
        "error: uploads with empty file extensions are not allowed",
        "error: the form field filename did not contain a file",
        "error: missing adifdata",
        "error: missing eqsl_user",
        "error: missing eqsl_pswd",
    ];
    if NOTHING_IMPORTED
        .iter()
        .any(|needle| body_lower.contains(needle))
    {
        return true;
    }
    // „No match on eQSL_User/eQSL_Pswd” bez kwalifikatora daty jest błędem
    // wywołującego program (nic nie zaimportowano). Wariant z „for date …”
    // zatrzymuje przetwarzanie na rekordzie, więc jest niejednoznaczny.
    body_lower.contains("error: no match on eqsl_user/eqsl_pswd")
        && !body_lower.contains("for date")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::upload_plan::BatchOutcome;
    use std::io::{Read, Write};

    const SECRET: &str = "TEST_SECRET_DO_NOT_USE_EQSL";

    // ---- klasyfikacja (czysta, bez sieci) ----

    #[test]
    fn http_200_with_full_result_is_a_records_result() {
        let body = "<!-- Reply form eQSL.cc ADIF Real-time Interface --><BR>Result: 3 out of 3 records added<BR>";
        assert_eq!(
            classify_eqsl_response(200, body),
            EqslClassification::RecordsResult { added: 3, total: 3 }
        );
    }

    #[test]
    fn result_line_is_parsed_case_insensitively() {
        assert_eq!(
            parse_records_result("RESULT: 12 OUT OF 12 RECORDS ADDED"),
            Some((12, 12))
        );
        assert_eq!(
            parse_records_result("result: 1 out of 2 records added"),
            Some((1, 2))
        );
    }

    #[test]
    fn result_with_missing_counts_is_not_parsed() {
        assert_eq!(parse_records_result("Result: 200"), None);
        assert_eq!(parse_records_result("Result: 0 records"), None);
        assert_eq!(parse_records_result("nothing here"), None);
    }

    #[test]
    fn non_200_status_is_fatal_even_with_success_text() {
        let body = "Result: 5 out of 5 records added";
        assert!(matches!(
            classify_eqsl_response(500, body),
            EqslClassification::FatalNothingImported { .. }
        ));
        assert!(matches!(
            classify_eqsl_response(503, body),
            EqslClassification::FatalNothingImported { .. }
        ));
    }

    #[test]
    fn documented_nothing_imported_errors_are_fatal() {
        for body in [
            "Error: File not saved",
            "Error: The system is down until 12:00",
            "Error: Uploads with empty file extensions are not allowed",
            "Error: The form field Filename did not contain a file.",
            "Error: Missing ADIFData parameter",
            "Error: Missing eQSL_User",
            "Error: Missing eQSL_Pswd",
            "Error: No match on eQSL_User/eQSL_Pswd",
        ] {
            assert!(
                matches!(
                    classify_eqsl_response(200, body),
                    EqslClassification::FatalNothingImported { .. }
                ),
                "body={body}"
            );
        }
    }

    #[test]
    fn stop_at_record_error_is_ambiguous_not_fatal_nothing() {
        let body = "Error: No match on eQSL_User/eQSL_Pswd for date 20260101 12:00";
        assert!(matches!(
            classify_eqsl_response(200, body),
            EqslClassification::FatalAmbiguous { .. }
        ));
    }

    #[test]
    fn warnings_only_is_a_partial_outcome() {
        let body =
            "Warning: Y=2026 M=01 D=01 Bad record: Duplicate<BR>Warning: Bad QSO Date: 20260231";
        match classify_eqsl_response(200, body) {
            EqslClassification::WarningsOnly { rejected, .. } => assert_eq!(rejected, 2),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reply_page_without_result_is_never_success() {
        let body = include_str_marker_only();
        assert!(matches!(
            classify_eqsl_response(200, &body),
            EqslClassification::NoResult { .. }
        ));
    }

    fn include_str_marker_only() -> String {
        format!("<!-- {EQSL_REPLY_MARKER} --><HTML><BODY></BODY></HTML>")
    }

    #[test]
    fn unrecognised_body_is_never_success() {
        assert!(matches!(
            classify_eqsl_response(200, "<html>hello</html>"),
            EqslClassification::NoResult { .. }
        ));
    }

    #[test]
    fn information_only_body_is_unknown_not_success() {
        let body = "Information: Received 123 bytes<BR>Information: Records were posted to multiple accounts for callsign TST1AA";
        assert!(matches!(
            classify_eqsl_response(200, body),
            EqslClassification::NoResult { .. }
        ));
    }

    // ---- mapowanie na wynik paczki ----

    #[test]
    fn full_result_for_exact_expected_count_is_accepted() {
        let classification = EqslClassification::RecordsResult { added: 5, total: 5 };
        assert_eq!(
            eqsl_batch_outcome(&classification, 5),
            BatchOutcome::Accepted
        );
    }

    #[test]
    fn full_result_for_wrong_expected_count_is_not_accepted() {
        // Usługa widziała mniej rekordów niż wysłano — ładunek nie dotarł w całości.
        let classification = EqslClassification::RecordsResult { added: 4, total: 4 };
        assert!(matches!(
            eqsl_batch_outcome(&classification, 5),
            BatchOutcome::Failed { .. }
        ));
    }

    #[test]
    fn partial_result_is_partial_not_accepted() {
        let classification = EqslClassification::RecordsResult { added: 2, total: 5 };
        assert_eq!(
            eqsl_batch_outcome(&classification, 5),
            BatchOutcome::PartiallyAccepted {
                rejected: 3,
                detail: "eQSL: dodano 2 z 5".to_string()
            }
        );
    }

    #[test]
    fn zero_added_is_no_records_processed() {
        let classification = EqslClassification::RecordsResult { added: 0, total: 4 };
        assert!(matches!(
            eqsl_batch_outcome(&classification, 4),
            BatchOutcome::NoRecordsProcessed { .. }
        ));
    }

    #[test]
    fn fatal_nothing_is_rejected_and_ambiguous_is_unknown() {
        assert!(matches!(
            eqsl_batch_outcome(
                &EqslClassification::FatalNothingImported { detail: "x".into() },
                3
            ),
            BatchOutcome::Rejected { .. }
        ));
        assert!(matches!(
            eqsl_batch_outcome(
                &EqslClassification::FatalAmbiguous { detail: "x".into() },
                3
            ),
            BatchOutcome::Unknown { .. }
        ));
        assert!(matches!(
            eqsl_batch_outcome(&EqslClassification::NoResult { detail: "x".into() }, 3),
            BatchOutcome::Unknown { .. }
        ));
    }

    #[test]
    fn empty_expected_count_never_yields_accepted() {
        let classification = EqslClassification::RecordsResult { added: 0, total: 0 };
        assert!(!eqsl_batch_outcome(&classification, 0).is_accepted());
    }

    // ---- redakcja ----

    #[test]
    fn passwords_are_redacted_from_any_text() {
        let text = format!("echo {SECRET} and {SECRET}");
        let redacted = redact_secret(&text, SECRET);
        assert!(!redacted.contains(SECRET));
        assert_eq!(redacted.matches("<redacted>").count(), 2);
    }

    #[test]
    fn empty_secret_does_not_rewrite_text() {
        assert_eq!(redact_secret("abc", ""), "abc");
    }

    // ---- serwer loopback (transport testowy tylko w #[cfg(test)]) ----

    struct LoopbackServer {
        base_url: String,
        handle: Option<std::thread::JoinHandle<()>>,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl LoopbackServer {
        fn start(response_status: &str, response_body: String, max_requests: usize) -> Self {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("loopback bind 127.0.0.1");
            let addr = listener.local_addr().expect("loopback addr");
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop_inner = stop.clone();
            let status_line = format!("HTTP/1.1 {response_status}");
            let handle = std::thread::spawn(move || {
                listener
                    .set_nonblocking(false)
                    .expect("loopback blocking mode");
                for _ in 0..max_requests {
                    if stop_inner.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                    let Ok((mut stream, _)) = listener.accept() else {
                        break;
                    };
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    read_http_request(&mut stream);
                    let response = format!(
                        "{status_line}\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                        response_body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            });
            Self {
                base_url: format!("http://{addr}/"),
                handle: Some(handle),
                stop,
            }
        }
    }

    impl Drop for LoopbackServer {
        fn drop(&mut self) {
            self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = std::net::TcpStream::connect(
                self.base_url
                    .trim_start_matches("http://")
                    .trim_end_matches('/'),
            );
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn read_http_request(stream: &mut std::net::TcpStream) {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    if let Some(header_end) = find_subslice(&buffer, b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
                        let content_length = head
                            .lines()
                            .find_map(|line| {
                                let lower = line.to_ascii_lowercase();
                                lower
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        let body_len = buffer.len().saturating_sub(header_end + 4);
                        if body_len >= content_length {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    #[tokio::test]
    async fn loopback_success_response_is_classified_as_accepted() {
        let server = LoopbackServer::start(
            "200 OK",
            format!("<!-- {EQSL_REPLY_MARKER} --><BR>Result: 2 out of 2 records added<BR>"),
            4,
        );
        let client = EqslClient::new("TSTCALL", SECRET).with_test_endpoint(server.base_url.clone());
        let submission = client
            .upload_adif("<EOR>")
            .await
            .expect("loopback response");
        assert_eq!(submission.http_status, 200);
        let classification = classify_eqsl_response(submission.http_status, &submission.body);
        assert_eq!(
            eqsl_batch_outcome(&classification, 2),
            BatchOutcome::Accepted
        );
    }

    #[test]
    fn loopback_error_page_is_not_success() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let server = LoopbackServer::start(
                "200 OK",
                "<!-- Reply form eQSL.cc ADIF Real-time Interface -->Error: Missing ADIFData parameter"
                    .to_string(),
                4,
            );
            let client =
                EqslClient::new("TSTCALL", SECRET).with_test_endpoint(server.base_url.clone());
            let submission = client.upload_adif("<EOR>").await.expect("loopback response");
            let classification = classify_eqsl_response(submission.http_status, &submission.body);
            assert!(matches!(
                eqsl_batch_outcome(&classification, 3),
                BatchOutcome::Rejected { .. }
            ));
        });
    }

    #[test]
    fn loopback_http_500_is_not_success() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let server = LoopbackServer::start("500 Internal Server Error", "oops".to_string(), 4);
            let client =
                EqslClient::new("TSTCALL", SECRET).with_test_endpoint(server.base_url.clone());
            let submission = client
                .upload_adif("<EOR>")
                .await
                .expect("loopback response");
            assert_eq!(submission.http_status, 500);
            let classification = classify_eqsl_response(submission.http_status, &submission.body);
            assert!(!eqsl_batch_outcome(&classification, 1).is_accepted());
        });
    }

    #[test]
    fn loopback_echoed_password_is_never_returned() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let server = LoopbackServer::start(
                "200 OK",
                format!("Error: No match on eQSL_User/eQSL_Pswd password={SECRET}"),
                4,
            );
            let client =
                EqslClient::new("TSTCALL", SECRET).with_test_endpoint(server.base_url.clone());
            let submission = client
                .upload_adif("<EOR>")
                .await
                .expect("loopback response");
            assert!(!submission.body.contains(SECRET));
            assert!(submission.body.contains("<redacted>"));
        });
    }

    #[test]
    fn loopback_1000_requests_all_classified_without_false_success() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let server = LoopbackServer::start(
                "200 OK",
                "<!-- Reply form eQSL.cc ADIF Real-time Interface --><BR>Result: 1 out of 1 records added<BR>"
                    .to_string(),
                1000,
            );
            let client =
                EqslClient::new("TSTCALL", SECRET).with_test_endpoint(server.base_url.clone());
            let mut accepted = 0usize;
            for _ in 0..1000 {
                let submission = client.upload_adif("<EOR>").await.expect("loopback response");
                let classification =
                    classify_eqsl_response(submission.http_status, &submission.body);
                if eqsl_batch_outcome(&classification, 1).is_accepted() {
                    accepted += 1;
                }
            }
            assert_eq!(accepted, 1000);
        });
    }

    #[test]
    fn production_endpoint_is_the_documented_processor() {
        assert_eq!(
            EQSL_IMPORT_ENDPOINT,
            "https://www.eqsl.cc/qslcard/ImportADIF.cfm"
        );
        assert!(!EQSL_IMPORT_ENDPOINT.to_ascii_lowercase().ends_with(".txt"));
        assert!(EQSL_IMPORT_ENDPOINT.starts_with("https://"));
    }

    #[test]
    fn default_client_targets_the_documented_processor() {
        let client = EqslClient::new("TSTCALL", SECRET);
        assert_eq!(client.upload_endpoint(), EQSL_IMPORT_ENDPOINT);
    }
}
