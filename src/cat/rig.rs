// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Typy domenowe architektury sterowania transceiverem (CAT).
//!
//! Moduł definiuje podstawowe kontrakty danych, poleceń i błędów CAT:
//! - [`RigState`]: pełny stan transceivera,
//! - [`RigCommand`]: domenowe polecenia sterowania radiem,
//! - [`RigCapabilities`]: deklaracja obsługiwanych funkcji backendu,
//! - [`RigError`]: typowany błąd operacji CAT z obsługą i18n,
//! - [`RigStatus`]: agregat bieżącego stanu radia i ostatniego błędu.

use crate::cat::backend::CatBackendKind;
use crate::core::i18n::{Msg, key};

/// Pełny stan transceivera.
#[derive(Debug, Clone, PartialEq)]
pub struct RigState {
    pub frequency_hz: u64,
    pub mode: String,
    pub passband_hz: u32,
    pub vfo: String,    // np. "VFOA", "VFOB", "Main", "Sub"
    pub tx_vfo: String, // VFO używane do nadawania w trybie Split
    pub split_enabled: bool,
    pub tx_frequency_hz: Option<u64>, // Częstotliwość nadawania w trybie Split
    pub s_meter_dbm: f32,             // Sygnał S-Metra w dBm (lub S9+dB)
    pub s_meter_unit: String,         // np. "S9+10dB" lub "S7"
    pub rf_power_watts: f32,          // Moc wyjściowa w Watach
    pub ptt: bool,
    pub rit_hz: i32, // Przesunięcie RIT w Hz
    pub xit_hz: i32, // Przesunięcie XIT w Hz
    pub connected: bool,
}

impl Default for RigState {
    fn default() -> Self {
        Self {
            frequency_hz: 14_025_000,
            mode: "CW".to_string(),
            passband_hz: 500,
            vfo: "VFOA".to_string(),
            tx_vfo: "VFOA".to_string(),
            split_enabled: false,
            tx_frequency_hz: None,
            s_meter_dbm: -100.0,
            s_meter_unit: "S1".to_string(),
            rf_power_watts: 0.0,
            ptt: false,
            rit_hz: 0,
            xit_hz: 0,
            connected: false,
        }
    }
}

/// Domenowe polecenia sterowania transceiverem.
#[derive(Debug, Clone, PartialEq)]
pub enum RigCommand {
    SetFrequency(u64),
    SetSplitFrequency(u64),
    SetMode { mode: String, passband_hz: u32 },
    SetPassband(u32),
    SetSplit { enabled: bool, tx_vfo: String },
    SetPtt(bool),
    SetVfo(String),
    SetRit(i32),
    SetXit(i32),
    SetPower(f32),
}

/// Deklaracja możliwości technicznych danego backendu CAT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RigCapabilities {
    pub frequency: bool,
    pub mode: bool,
    pub passband: bool,
    pub split: bool,
    pub split_frequency: bool,
    pub ptt: bool,
    pub vfo: bool,
    pub rit: bool,
    pub xit: bool,
    pub power: bool,
    pub s_meter: bool,
}

impl RigCapabilities {
    pub const ALL: Self = Self {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        s_meter: true,
    };

    pub const NONE: Self = Self {
        frequency: false,
        mode: false,
        passband: false,
        split: false,
        split_frequency: false,
        ptt: false,
        vfo: false,
        rit: false,
        xit: false,
        power: false,
        s_meter: false,
    };

    pub const fn all() -> Self {
        Self::ALL
    }

    pub const fn none() -> Self {
        Self::NONE
    }

    /// Sprawdza, czy backend obsługuje dane polecenie domenowe.
    pub fn supports(&self, command: &RigCommand) -> bool {
        match command {
            RigCommand::SetFrequency(_) => self.frequency,
            RigCommand::SetSplitFrequency(_) => self.split_frequency,
            RigCommand::SetMode { .. } => self.mode,
            RigCommand::SetPassband(_) => self.passband,
            RigCommand::SetSplit { .. } => self.split,
            RigCommand::SetPtt(_) => self.ptt,
            RigCommand::SetVfo(_) => self.vfo,
            RigCommand::SetRit(_) => self.rit,
            RigCommand::SetXit(_) => self.xit,
            RigCommand::SetPower(_) => self.power,
        }
    }
}

/// Typowane błędy podsystemu CAT.
#[derive(Debug, Clone, PartialEq)]
pub enum RigError {
    Unsupported {
        backend: CatBackendKind,
        command: &'static str,
    },
    NotConnected,
    Busy,
    ServiceStopped,
    Connect {
        endpoint: String,
        detail: String,
    },
    Timeout {
        endpoint: String,
        operation: &'static str,
    },
    Rejected {
        code: i32,
    },
    Protocol {
        detail: String,
    },
}

impl RigError {
    /// Zwraca zlokalizowany komunikat błędu dla warstwy prezentacji (UI/toasty).
    pub fn msg(&self) -> Msg {
        match self {
            Self::Unsupported { backend, command } => Msg::new(key::CAT_ERR_UNSUPPORTED)
                .arg("backend", backend.label())
                .arg("command", *command),
            Self::NotConnected => Msg::new(key::CAT_ERR_NOT_CONNECTED),
            Self::Busy => Msg::new(key::CAT_ERR_BUSY),
            Self::ServiceStopped => Msg::new(key::CAT_ERR_STOPPED),
            Self::Connect { endpoint, detail } => Msg::new(key::CAT_ERR_CONNECT)
                .arg("endpoint", endpoint)
                .arg("detail", detail),
            Self::Timeout {
                endpoint,
                operation,
            } => Msg::new(key::CAT_ERR_TIMEOUT)
                .arg("operation", *operation)
                .arg("endpoint", endpoint),
            Self::Rejected { code } => Msg::new(key::CAT_ERR_REJECTED).arg("code", code),
            Self::Protocol { detail } => Msg::new(key::CAT_ERR_PROTOCOL).arg("detail", detail),
        }
    }
}

impl std::fmt::Display for RigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { backend, command } => {
                write!(
                    f,
                    "CAT backend '{}' does not support command: {command}",
                    backend.as_str()
                )
            }
            Self::NotConnected => write!(f, "CAT is not connected"),
            Self::Busy => write!(f, "CAT command queue is full"),
            Self::ServiceStopped => write!(f, "CAT service has been stopped"),
            Self::Connect { endpoint, detail } => {
                write!(f, "cannot connect to CAT endpoint '{endpoint}': {detail}")
            }
            Self::Timeout {
                endpoint,
                operation,
            } => {
                write!(f, "CAT operation '{operation}' timed out for '{endpoint}'")
            }
            Self::Rejected { code } => {
                write!(f, "CAT command rejected with error code {code}")
            }
            Self::Protocol { detail } => {
                write!(f, "CAT protocol error: {detail}")
            }
        }
    }
}

impl std::error::Error for RigError {}

/// Bieżący status radia łączący odczytany stan i ostatni błąd.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RigStatus {
    pub state: RigState,
    pub last_error: Option<RigError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::i18n::Language;

    #[test]
    fn rig_state_default_unchanged() {
        let state = RigState::default();
        assert_eq!(state.frequency_hz, 14_025_000);
        assert_eq!(state.mode, "CW");
        assert_eq!(state.passband_hz, 500);
        assert_eq!(state.vfo, "VFOA");
        assert_eq!(state.tx_vfo, "VFOA");
        assert!(!state.split_enabled);
        assert_eq!(state.tx_frequency_hz, None);
        assert_eq!(state.s_meter_dbm, -100.0);
        assert_eq!(state.s_meter_unit, "S1");
        assert_eq!(state.rf_power_watts, 0.0);
        assert!(!state.ptt);
        assert_eq!(state.rit_hz, 0);
        assert_eq!(state.xit_hz, 0);
        assert!(!state.connected);
    }

    #[test]
    fn capabilities_support_matrix() {
        let all_caps = RigCapabilities::ALL;
        let none_caps = RigCapabilities::NONE;
        let default_caps = RigCapabilities::default();
        assert_eq!(none_caps, default_caps);

        let commands = [
            RigCommand::SetFrequency(14_074_000),
            RigCommand::SetSplitFrequency(14_075_000),
            RigCommand::SetMode {
                mode: "USB".to_string(),
                passband_hz: 2400,
            },
            RigCommand::SetPassband(500),
            RigCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string(),
            },
            RigCommand::SetPtt(true),
            RigCommand::SetVfo("VFOA".to_string()),
            RigCommand::SetRit(50),
            RigCommand::SetXit(-50),
            RigCommand::SetPower(50.0),
        ];

        for cmd in &commands {
            assert!(
                all_caps.supports(cmd),
                "RigCapabilities::ALL should support {cmd:?}"
            );
            assert!(
                !none_caps.supports(cmd),
                "RigCapabilities::NONE should not support {cmd:?}"
            );
        }

        let matrix = [
            (
                RigCapabilities {
                    frequency: true,
                    ..RigCapabilities::NONE
                },
                &commands[0],
            ),
            (
                RigCapabilities {
                    split_frequency: true,
                    ..RigCapabilities::NONE
                },
                &commands[1],
            ),
            (
                RigCapabilities {
                    mode: true,
                    ..RigCapabilities::NONE
                },
                &commands[2],
            ),
            (
                RigCapabilities {
                    passband: true,
                    ..RigCapabilities::NONE
                },
                &commands[3],
            ),
            (
                RigCapabilities {
                    split: true,
                    ..RigCapabilities::NONE
                },
                &commands[4],
            ),
            (
                RigCapabilities {
                    ptt: true,
                    ..RigCapabilities::NONE
                },
                &commands[5],
            ),
            (
                RigCapabilities {
                    vfo: true,
                    ..RigCapabilities::NONE
                },
                &commands[6],
            ),
            (
                RigCapabilities {
                    rit: true,
                    ..RigCapabilities::NONE
                },
                &commands[7],
            ),
            (
                RigCapabilities {
                    xit: true,
                    ..RigCapabilities::NONE
                },
                &commands[8],
            ),
            (
                RigCapabilities {
                    power: true,
                    ..RigCapabilities::NONE
                },
                &commands[9],
            ),
        ];

        for (target_caps, target_cmd) in &matrix {
            for candidate_cmd in &commands {
                if candidate_cmd == *target_cmd {
                    assert!(
                        target_caps.supports(candidate_cmd),
                        "Expected target capability to support {candidate_cmd:?}"
                    );
                } else {
                    assert!(
                        !target_caps.supports(candidate_cmd),
                        "Expected target capability to reject {candidate_cmd:?}"
                    );
                }
            }
        }

        // Test composite profile (e.g. subset without split/rit/xit/power)
        let basic_caps = RigCapabilities {
            frequency: true,
            mode: true,
            ptt: true,
            s_meter: true,
            ..RigCapabilities::NONE
        };
        assert!(basic_caps.supports(&commands[0]));
        assert!(basic_caps.supports(&commands[2]));
        assert!(basic_caps.supports(&commands[5]));
        assert!(!basic_caps.supports(&commands[1]));
        assert!(!basic_caps.supports(&commands[3]));
        assert!(!basic_caps.supports(&commands[4]));
        assert!(!basic_caps.supports(&commands[6]));
        assert!(!basic_caps.supports(&commands[7]));
        assert!(!basic_caps.supports(&commands[8]));
        assert!(!basic_caps.supports(&commands[9]));
    }

    #[test]
    fn rig_error_messages_have_no_placeholders() {
        let errors = [
            RigError::Unsupported {
                backend: CatBackendKind::Hamlib,
                command: "SetSplit",
            },
            RigError::NotConnected,
            RigError::Busy,
            RigError::ServiceStopped,
            RigError::Connect {
                endpoint: "127.0.0.1:4532".to_string(),
                detail: "connection refused".to_string(),
            },
            RigError::Timeout {
                endpoint: "127.0.0.1:4532".to_string(),
                operation: "get_freq",
            },
            RigError::Rejected { code: -1 },
            RigError::Protocol {
                detail: "invalid response".to_string(),
            },
        ];

        let languages = [
            Language::Pl,
            Language::En,
            Language::De,
            Language::Fr,
            Language::Es,
            Language::It,
            Language::Ru,
        ];

        let placeholder_names = [
            "{backend}",
            "{command}",
            "{endpoint}",
            "{detail}",
            "{operation}",
            "{code}",
        ];

        for err in &errors {
            for &lang in &languages {
                let rendered = err.msg().text(lang);
                assert!(
                    !rendered.is_empty(),
                    "Error message for {err:?} in {lang:?} is empty"
                );
                for placeholder in &placeholder_names {
                    assert!(
                        !rendered.contains(placeholder),
                        "Rendered error message for {err:?} in {lang:?} contains unrendered placeholder '{placeholder}': {rendered}"
                    );
                }
            }
        }
    }

    #[test]
    fn rig_error_display_and_std_error() {
        let errors: [RigError; 8] = [
            RigError::Unsupported {
                backend: CatBackendKind::Hamlib,
                command: "SetPower",
            },
            RigError::NotConnected,
            RigError::Busy,
            RigError::ServiceStopped,
            RigError::Connect {
                endpoint: "127.0.0.1:4532".to_string(),
                detail: "connection reset".to_string(),
            },
            RigError::Timeout {
                endpoint: "127.0.0.1:4532".to_string(),
                operation: "poll",
            },
            RigError::Rejected { code: 42 },
            RigError::Protocol {
                detail: "unexpected EOF".to_string(),
            },
        ];

        for err in &errors {
            let display = format!("{err}");
            assert!(!display.is_empty());
            let std_err: &dyn std::error::Error = err;
            assert_eq!(std_err.to_string(), display);
        }
    }

    #[test]
    fn rig_status_default() {
        let status = RigStatus::default();
        assert_eq!(status.state, RigState::default());
        assert_eq!(status.last_error, None);
    }
}
