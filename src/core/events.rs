// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralna magistrala zdarzeń (event bus) aplikacji.
//!
//! Stanowi pojedynczy punkt publikacji/subskrypcji zdarzeń domenowych
//! (nowe QSO, spot DX, zmiana stanu radia, status chmury). Jest używana m.in.
//! przez REST/WebSocket API do przesyłania zdarzeń na żywo do klientów oraz
//! do napędzania powiadomień w interfejsie. Oparta na `tokio::sync::broadcast`,
//! dzięki czemu zdarzenia mogą być konsumowane przez wielu subskrybentów
//! jednocześnie, a wolni odbiorcy nie blokują nadawcy.

use serde::Serialize;
use tokio::sync::broadcast;

/// Pojedyncze zdarzenie publikowane przez aplikację.
///
/// Struktura jest celowo spłaszczona i w pełni serializowalna (JSON), dzięki
/// czemu te same zdarzenia trafiają zarówno do UI, jak i przez WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    /// Zapisano nową łączność w dzienniku.
    QsoLogged {
        callsign: String,
        band: String,
        mode: String,
        frequency_hz: u64,
        time_utc: String,
    },
    /// Odebrano nowy spot z klastra DX.
    DxSpot {
        spotter: String,
        dx_call: String,
        frequency_khz: f64,
        band: String,
        comment: String,
        time_utc: String,
        is_ft8: bool,
        is_skimmer: bool,
    },
    /// Zmienił się stan transceivera (CAT).
    RigState {
        frequency_hz: u64,
        mode: String,
        connected: bool,
    },
    /// Zmienił się stan połączenia z klastrem DX.
    ClusterStatus {
        connected: bool,
    },
    /// Zdarzenie synchronizacji z serwisem zewnętrznym (eQSL, LoTW, Club Log, QRZ…).
    CloudSync {
        service: String,
        status: String,
        detail: String,
    },
    /// Ogólne powiadomienie / toast dla użytkownika.
    Toast {
        level: String,
        message: String,
    },
}

/// Centralna magistrala zdarzeń. Tania w klonowaniu — współdzieli nadajnik.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    /// Tworzy nową magistralę o zadanej pojemności bufora.
    /// Gdy bufor się przepełni, najstarsi subskrybenci otrzymują błąd `Lagged`.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Publikuje zdarzenie do wszystkich subskrybentów.
    /// Ignoruje błąd, gdy nie ma żadnego aktywnego odbiorcy.
    pub fn publish(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    /// Tworzy nowego subskrybenta od bieżącego momentu.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }

    /// Liczba aktywnych odbiorców.
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_bus_delivers_to_subscribers() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        bus.publish(AppEvent::Toast {
            level: "info".into(),
            message: "test".into(),
        });

        // broadcast jest asynchroniczny; odbiór w teście blokującym jest natychmiastowy.
        let ev = rx.try_recv().expect("zdarzenie powinno dotrzeć");
        match ev {
            AppEvent::Toast { level, message } => {
                assert_eq!(level, "info");
                assert_eq!(message, "test");
            }
            other => panic!("nieoczekiwane zdarzenie: {:?}", other),
        }
    }

    #[test]
    fn event_bus_serializes_to_json() {
        let ev = AppEvent::QsoLogged {
            callsign: "SP6INA".into(),
            band: "20m".into(),
            mode: "SSB".into(),
            frequency_hz: 14_250_000,
            time_utc: "2026-01-01T12:00:00Z".into(),
        };
        let json = serde_json::to_string(&ev).expect("serializacja");
        assert!(json.contains("\"type\":\"qso_logged\""));
        assert!(json.contains("\"callsign\":\"SP6INA\""));
    }
}
