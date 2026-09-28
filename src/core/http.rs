// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wspólne narzędzia HTTP: reużywalny klient `reqwest` oraz mechanizm retry
//! z exponential backoff + jitter dla integracji sieciowych (LoTW, eQSL,
//! ClubLog, HamQTH, HRDLog, Cloudlog, callbook).

use std::sync::OnceLock;
use std::time::Duration;

/// Stały identyfikator User-Agent dla wszystkich zapytań HTTP.
pub const USER_AGENT: &str = concat!("SPLogbook/", env!("CARGO_PKG_VERSION"), " (SP6INA; contact@splogbook.org)");

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
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let seed = nanos ^ (attempt as u64).wrapping_mul(0x9E37_79B9);
    let pct = (seed % 41) as i64 - 20; // -20..=20
    (base_ms as i64 * pct / 100).max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_http_client_is_shared_singleton() {
        let a = http_client();
        let b = http_client();
        assert!(std::ptr::eq(a, b), "http_client() musi zwracać ten sam klient");
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
                    if n < 3 {
                        Err("fail")
                    } else {
                        Ok("ok")
                    }
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
}
