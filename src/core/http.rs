// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wspólne narzędzia HTTP: reużywalny klient `reqwest` oraz mechanizm retry
//! z exponential backoff + jitter dla integracji sieciowych (LoTW, eQSL,
//! ClubLog, HamQTH, HRDLog, Cloudlog, callbook).

use std::sync::OnceLock;
use std::time::Duration;

/// Stały identyfikator User-Agent dla wszystkich zapytań HTTP.
pub const USER_AGENT: &str = concat!(
    "SPLogbook/",
    env!("CARGO_PKG_VERSION"),
    " (SP6INA; contact@splogbook.org)"
);

/// Domyślny timeout zapytań HTTP (bezpieczny dla wolnych usług cloudowych).
const DEFAULT_TIMEOUT_SECS: u64 = 15;

/// Wspólny, reużywany klient HTTP (z własną pulą połączeń i timeoutem).
///
/// Tworzenie nowego `reqwest::Client` przy każdym lookupie/uploadzie marnuje
/// pulę połączeń i zasoby; ta funkcja zwraca jeden współdzielony klient.
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn http_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Buduje klient o niestandardowym timeoutcie (rzadkie przypadki, np. Callook
/// 6s, HamQTH XML 10s). Wspólna konfiguracja user-agenta pozostaje spójna.
pub fn http_client_with_timeout(secs: u64) -> reqwest::Client {
    http_client_with_timeout_millis(secs * 1000)
}

/// Buduje klient o timeoutcie wyrażonym w milisekundach (np. FLDIGI XML-RPC 500ms).
pub fn http_client_with_timeout_millis(millis: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(millis))
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Wykonuje asynchroniczną akcję z powtórzeniami (exponential backoff + jitter).
///
/// `attempts` to liczba prób (min. 1). Opóźnienia: 0.5s, 1s, 2s, 4s... z jitterem
/// do ±20% na bazie nanosów zegara systemowego (bez zależności od `rand`).
pub async fn retry_async<F, Fut, T, E>(mut action: F, attempts: usize) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    if attempts == 0 {
        return action().await;
    }

    for attempt in 0..attempts {
        match action().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt + 1 >= attempts {
                    // Ostatnia próba zakończona błędem — zwracamy go bezpośrednio,
                    // bez przechowywania w `Option` i późniejszego `.expect(...)`.
                    return Err(e);
                }
                let base_ms = 500u64 * (1u64 << attempt.min(6));
                let jitter = jitter_ms(base_ms, attempt);
                tokio::time::sleep(Duration::from_millis(base_ms + jitter)).await;
            }
        }
    }

    // Nieosiągalne: przy `attempts >= 1` pętla zawsze kończy się jawnym powrotem
    // (sukces w dowolnej próbie albo błąd w ostatniej próbie).
    unreachable!("retry_async: pętla zakończyła się bez jawnego powrotu")
}

/// Deterministyczny jitter (±20%) oparty na nanosach zegara systemowego.
fn jitter_ms(base_ms: u64, attempt: usize) -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    let seed = nanos ^ (attempt as u64).wrapping_mul(0x9E37_79B9);
    let pct = (seed % 41) as i64 - 20; // -20..=20
    (base_ms as i64 * pct / 100).max(0) as u64
}

/// Zwraca zredagowany adres URL dla celów diagnostycznych, pozbawiony
/// danych uwierzytelniających (userinfo), parametrów zapytania (query) i fragmentów.
pub fn sanitize_url(url: &reqwest::Url) -> String {
    let mut clean = url.clone();
    let _ = clean.set_username("");
    let _ = clean.set_password(None);
    clean.set_query(None);
    clean.set_fragment(None);
    clean.to_string()
}

/// Zwraca zredagowany ciąg URL ze stringa.
pub fn redacted_url_for_diagnostics(raw: &str) -> String {
    if let Ok(url) = reqwest::Url::parse(raw) {
        sanitize_url(&url)
    } else if let Some((base, _)) = raw.split_once('?') {
        base.to_string()
    } else {
        raw.to_string()
    }
}

/// Bezpieczna redakcja błędów sieciowych `reqwest::Error`.
///
/// Zapobiega wyciekowi sekretów przesyłanych w URL (hasła, tokeny, klucze API, basic auth)
/// do logów, GUI oraz komunikatów o błędach. Zachowuje diagnostykę niesekretną:
/// bezpieczny schemat + host + ścieżkę oraz powód błędu bez surowego URL z parametrami.
pub fn sanitize_network_error(err: &reqwest::Error) -> String {
    if let Some(url) = err.url() {
        let clean = sanitize_url(url);
        let raw = err.to_string();
        raw.replace(url.as_str(), &clean)
    } else {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_http_client_is_shared_singleton() {
        let a = http_client();
        let b = http_client();
        assert!(
            std::ptr::eq(a, b),
            "http_client() musi zwracać ten sam klient"
        );
    }

    #[tokio::test]
    async fn test_retry_async_succeeds_on_first_attempt() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let result = retry_async(
            move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, &'static str>(42)
                }
            },
            3,
        )
        .await;
        assert_eq!(result, Ok(42));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_async_retries_then_succeeds() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let result = retry_async(
            move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                    if n < 3 { Err("fail") } else { Ok("ok") }
                }
            },
            3,
        )
        .await;
        assert_eq!(result, Ok("ok"));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_async_exhausts_attempts() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let result = retry_async(
            move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>("always fail")
                }
            },
            2,
        )
        .await;
        assert_eq!(result, Err("always fail"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn cloud_errors_never_expose_credentials() {
        let fake_pass = "TEST_SECRET_DO_NOT_USE_PASS_123";
        let fake_key = "TEST_SECRET_DO_NOT_USE_KEY_456";
        let fake_bearer = "TEST_SECRET_DO_NOT_USE_BEARER_789";
        let fake_encoded = "p%40ss%2Fw%3Ard";

        // 1. Hasło w query
        let url_query =
            reqwest::Url::parse(&format!("https://example.com/api?password={fake_pass}")).unwrap();
        let sanitized = sanitize_url(&url_query);
        assert!(!sanitized.contains(fake_pass));
        assert_eq!(sanitized, "https://example.com/api");

        // 2. API key w query
        let url_key =
            reqwest::Url::parse(&format!("https://example.com/api?api_key={fake_key}")).unwrap();
        let sanitized_key = sanitize_url(&url_key);
        assert!(!sanitized_key.contains(fake_key));
        assert_eq!(sanitized_key, "https://example.com/api");

        // 3. Basic Auth w URL
        let url_basic =
            reqwest::Url::parse(&format!("https://user:{fake_pass}@example.com/api")).unwrap();
        let sanitized_basic = sanitize_url(&url_basic);
        assert!(!sanitized_basic.contains(fake_pass));
        assert_eq!(sanitized_basic, "https://example.com/api");

        // 4. Sekret zawierający znaki wymagające kodowania URL
        let url_encoded = reqwest::Url::parse(&format!(
            "https://user:{fake_encoded}@example.com/api?pass={fake_encoded}"
        ))
        .unwrap();
        let sanitized_encoded = sanitize_url(&url_encoded);
        assert!(!sanitized_encoded.contains("pass="));
        assert_eq!(sanitized_encoded, "https://example.com/api");

        // 5. Niepoprawny URL
        let raw_invalid = format!("not-a-valid-url?secret={fake_key}");
        let sanitized_invalid = redacted_url_for_diagnostics(&raw_invalid);
        assert!(!sanitized_invalid.contains(fake_key));
        assert_eq!(sanitized_invalid, "not-a-valid-url");

        // 6. Bearer token w diagnostyce
        let bearer_diag = format!("Authorization: Bearer {fake_bearer}");
        assert!(bearer_diag.contains(fake_bearer));
        let sanitized_header = if bearer_diag.starts_with("Authorization:") {
            "Authorization: Bearer <redacted>".to_string()
        } else {
            bearer_diag
        };
        assert!(!sanitized_header.contains(fake_bearer));
        assert!(sanitized_header.contains("<redacted>"));
    }

    #[test]
    fn secret_query_parameters_are_redacted() {
        let fake_secret = "TEST_SECRET_DO_NOT_USE_QUERY_PARAM";
        let mut url = reqwest::Url::parse("https://xmldata.qrz.com/xml/current/").unwrap();
        url.query_pairs_mut()
            .append_pair("username", "SP6INA")
            .append_pair("password", fake_secret)
            .append_pair("agent", "SPLogbook");

        assert!(url.as_str().contains(fake_secret));
        assert!(url.as_str().contains("username=SP6INA"));

        let redacted = sanitize_url(&url);
        assert!(!redacted.contains(fake_secret));
        assert!(!redacted.contains("password"));
        assert_eq!(redacted, "https://xmldata.qrz.com/xml/current/");
    }
}
