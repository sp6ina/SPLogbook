// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kolejka powiadomień (toastów) z ważnością: błędy nie są po cichu nadpisywane
//! przez kolejne zdarzenia, a czas wyświetlania zależy od ważności.

use std::time::{Duration, Instant};

/// Maksymalna liczba jednocześnie widocznych toastów.
pub const MAX_VISIBLE_TOASTS: usize = 4;
/// Limit całej kolejki (widoczne + oczekujące).
pub const MAX_QUEUED_TOASTS: usize = 12;

/// Ważność powiadomienia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToastSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastSeverity {
    /// Czas wyświetlania liczony od chwili, gdy toast staje się widoczny.
    #[must_use]
    pub fn ttl(self) -> Duration {
        match self {
            Self::Info | Self::Success => Duration::from_secs(5),
            Self::Warning => Duration::from_secs(7),
            Self::Error => Duration::from_secs(10),
        }
    }

    /// Mapuje tekstowy poziom z `AppEvent::Toast` (np. "error", "warn") na ważność.
    #[must_use]
    pub fn from_level(level: &str) -> Self {
        match level.trim().to_ascii_lowercase().as_str() {
            "error" | "err" | "fatal" => Self::Error,
            "warn" | "warning" => Self::Warning,
            "success" | "ok" => Self::Success,
            _ => Self::Info,
        }
    }

    /// Ikona tekstowa (ważność nie jest sygnalizowana wyłącznie kolorem).
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Success => "✔",
            Self::Warning => "⚠",
            Self::Error => "✖",
        }
    }
}

/// Pojedyncze powiadomienie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub severity: ToastSeverity,
    pub message: String,
    /// Moment, w którym toast po raz pierwszy stał się widoczny (start odliczania).
    shown_at: Option<Instant>,
}

/// Kolejka toastów w kolejności dodawania; widoczne są pierwsze [`MAX_VISIBLE_TOASTS`].
#[derive(Debug, Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
    next_id: u64,
}

impl ToastQueue {
    /// Dodaje toast (czas bieżący) i zwraca jego identyfikator.
    pub fn push(&mut self, severity: ToastSeverity, message: impl Into<String>) -> u64 {
        self.push_at(severity, message, Instant::now())
    }

    /// Dodaje toast z jawnym czasem (testowalne). Identyczny komunikat o tej samej
    /// ważności nie tworzy duplikatu — odświeża istniejący.
    pub fn push_at(
        &mut self,
        severity: ToastSeverity,
        message: impl Into<String>,
        now: Instant,
    ) -> u64 {
        let message = message.into();
        if let Some(existing) = self
            .toasts
            .iter_mut()
            .find(|t| t.severity == severity && t.message == message)
        {
            if existing.shown_at.is_some() {
                existing.shown_at = Some(now);
            }
            return existing.id;
        }

        // Ważne toasty (ostrzeżenia, błędy) wypychają najstarszy mało ważny widoczny toast.
        if severity >= ToastSeverity::Warning
            && self.toasts.len() >= MAX_VISIBLE_TOASTS
            && let Some(victim) = self
                .toasts
                .iter()
                .take(MAX_VISIBLE_TOASTS)
                .position(|t| t.severity < ToastSeverity::Warning)
        {
            self.toasts.remove(victim);
        }

        // Limit kolejki: najpierw odrzucamy najstarszy najmniej ważny toast.
        if self.toasts.len() >= MAX_QUEUED_TOASTS {
            let lowest = self.toasts.iter().map(|t| t.severity).min();
            if let Some(lowest) = lowest
                && let Some(idx) = self.toasts.iter().position(|t| t.severity == lowest)
            {
                self.toasts.remove(idx);
            }
        }

        self.next_id += 1;
        let id = self.next_id;
        self.toasts.push(Toast {
            id,
            severity,
            message,
            shown_at: None,
        });
        id
    }

    /// Uruchamia odliczanie dla nowo widocznych toastów i usuwa wygasłe.
    pub fn tick(&mut self, now: Instant) {
        loop {
            for toast in self.toasts.iter_mut().take(MAX_VISIBLE_TOASTS) {
                if toast.shown_at.is_none() {
                    toast.shown_at = Some(now);
                }
            }
            let before = self.toasts.len();
            let mut index = 0;
            self.toasts.retain(|toast| {
                let keep = index >= MAX_VISIBLE_TOASTS
                    || toast
                        .shown_at
                        .is_none_or(|t| now.saturating_duration_since(t) < toast.severity.ttl());
                index += 1;
                keep
            });
            // Usunięcie mogło odsłonić kolejne toasty z kolejki — nadaj im czas startu.
            if self.toasts.len() == before {
                break;
            }
        }
    }

    /// Ręczne zamknięcie toastu; zwraca `true`, gdy toast istniał.
    pub fn dismiss(&mut self, id: u64) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.id != id);
        self.toasts.len() != before
    }

    /// Aktualnie widoczne toasty (najstarszy pierwszy).
    #[must_use]
    pub fn visible(&self) -> &[Toast] {
        &self.toasts[..self.toasts.len().min(MAX_VISIBLE_TOASTS)]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Najnowszy toast (do testów i diagnostyki).
    #[must_use]
    pub fn latest(&self) -> Option<&Toast> {
        self.toasts.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_is_not_overwritten_by_later_success_toasts() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        let error_id = queue.push_at(ToastSeverity::Error, "błąd zapisu", t0);
        for i in 0..20 {
            queue.push_at(ToastSeverity::Success, format!("sukces {i}"), t0);
        }
        assert!(
            queue.iter().any(|t| t.id == error_id),
            "błąd musi przetrwać"
        );
        assert!(queue.len() <= MAX_QUEUED_TOASTS);
        assert!(queue.visible().len() <= MAX_VISIBLE_TOASTS);
    }

    #[test]
    fn error_pushes_out_low_severity_when_visible_slots_are_full() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        for i in 0..MAX_VISIBLE_TOASTS {
            queue.push_at(ToastSeverity::Info, format!("info {i}"), t0);
        }
        let error_id = queue.push_at(ToastSeverity::Error, "błąd", t0);
        assert!(
            queue.visible().iter().any(|t| t.id == error_id),
            "błąd musi być widoczny od razu, nie czekać za informacjami"
        );
    }

    #[test]
    fn ttl_differs_by_severity() {
        assert!(ToastSeverity::Info.ttl() < ToastSeverity::Warning.ttl());
        assert!(ToastSeverity::Warning.ttl() < ToastSeverity::Error.ttl());
        assert_eq!(ToastSeverity::Info.ttl(), ToastSeverity::Success.ttl());
        assert_eq!(ToastSeverity::Error.ttl(), Duration::from_secs(10));
    }

    #[test]
    fn toasts_expire_after_their_own_ttl() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        queue.push_at(ToastSeverity::Info, "info", t0);
        queue.push_at(ToastSeverity::Error, "błąd", t0);
        queue.tick(t0);
        assert_eq!(queue.len(), 2);

        queue.tick(t0 + Duration::from_secs(6));
        assert_eq!(queue.len(), 1, "info wygasa po ~5 s, błąd zostaje");
        assert_eq!(queue.latest().unwrap().severity, ToastSeverity::Error);

        queue.tick(t0 + Duration::from_secs(11));
        assert!(queue.is_empty(), "błąd wygasa po ~10 s");
    }

    #[test]
    fn manual_dismiss_removes_only_the_chosen_toast() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        let a = queue.push_at(ToastSeverity::Info, "a", t0);
        let b = queue.push_at(ToastSeverity::Warning, "b", t0);
        let c = queue.push_at(ToastSeverity::Error, "c", t0);
        assert!(queue.dismiss(b));
        assert!(!queue.dismiss(b), "drugie zamknięcie nic nie robi");
        let ids: Vec<u64> = queue.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![a, c]);
    }

    #[test]
    fn queued_toast_starts_its_timer_only_when_it_becomes_visible() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        for i in 0..=MAX_VISIBLE_TOASTS {
            queue.push_at(ToastSeverity::Warning, format!("w{i}"), t0);
        }
        queue.tick(t0);
        assert_eq!(queue.len(), MAX_VISIBLE_TOASTS + 1);
        // Po 8 s pierwsze cztery wygasają, piąty dopiero zaczyna odliczanie.
        queue.tick(t0 + Duration::from_secs(8));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.visible().len(), 1);
    }

    #[test]
    fn identical_message_is_deduplicated() {
        let mut queue = ToastQueue::default();
        let t0 = Instant::now();
        let a = queue.push_at(ToastSeverity::Info, "to samo", t0);
        let b = queue.push_at(ToastSeverity::Info, "to samo", t0);
        assert_eq!(a, b);
        assert_eq!(queue.len(), 1);
    }
}
