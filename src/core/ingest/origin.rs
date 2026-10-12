// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Typy źródeł QSO oraz polityk zapisu w potoku `core::ingest` (PIPE-1).

/// Źródło pochodzenia rekordu QSO trafiającego do potoku zapisu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QsoOrigin {
    Form,
    Contest,
    WsjtX,
    Js8Call,
    LanPeer,
    RestApi,
}

/// Tryb traktowania błędów walidacji przez potok zapisu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Odrzucenie zapisu przy błędzie walidacji (ścieżki interaktywne i zewnętrzne API).
    Reject,
    /// Zapis rekordu mimo błędu walidacji wraz z raportowaniem ostrzeżenia (źródła automatyczne).
    Advisory,
}

/// Zestaw flag efektów ubocznych do wykonania po pomyślnym zapisie QSO w bazie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PostEffects {
    pub plugin_hook: bool,
    pub enqueue_uploads: bool,
    pub broadcast_lan: bool,
    pub broadcast_n1mm: bool,
    pub psk_report: bool,
    pub alert_sound: bool,
    pub propagation_observation: bool,
}

/// Polityka określająca zasady walidacji i skutki uboczne dla danego źródła QSO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestPolicy {
    pub validation: ValidationMode,
    pub effects: PostEffects,
}

impl QsoOrigin {
    /// Zwraca niezmienną politykę zapisu powiązaną z danym źródłem QSO.
    pub const fn policy(self) -> IngestPolicy {
        match self {
            Self::Form => IngestPolicy {
                validation: ValidationMode::Reject,
                effects: PostEffects {
                    plugin_hook: true,
                    enqueue_uploads: true,
                    broadcast_lan: true,
                    broadcast_n1mm: true,
                    psk_report: true,
                    alert_sound: true,
                    propagation_observation: true,
                },
            },
            Self::Contest => IngestPolicy {
                validation: ValidationMode::Reject,
                effects: PostEffects {
                    plugin_hook: true,
                    enqueue_uploads: true,
                    broadcast_lan: true,
                    broadcast_n1mm: true,
                    psk_report: false,
                    alert_sound: false,
                    propagation_observation: false,
                },
            },
            Self::WsjtX | Self::Js8Call => IngestPolicy {
                validation: ValidationMode::Advisory,
                effects: PostEffects {
                    plugin_hook: true,
                    enqueue_uploads: true,
                    broadcast_lan: true,
                    broadcast_n1mm: false,
                    psk_report: false,
                    alert_sound: false,
                    propagation_observation: false,
                },
            },
            Self::LanPeer => IngestPolicy {
                validation: ValidationMode::Advisory,
                effects: PostEffects {
                    plugin_hook: false,
                    enqueue_uploads: false,
                    broadcast_lan: false,
                    broadcast_n1mm: false,
                    psk_report: false,
                    alert_sound: false,
                    propagation_observation: false,
                },
            },
            Self::RestApi => IngestPolicy {
                validation: ValidationMode::Reject,
                effects: PostEffects {
                    plugin_hook: false,
                    enqueue_uploads: false,
                    broadcast_lan: false,
                    broadcast_n1mm: false,
                    psk_report: false,
                    alert_sound: false,
                    propagation_observation: false,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IngestPolicy, PostEffects, QsoOrigin, ValidationMode};

    #[test]
    fn policy_table_is_pinned() {
        const CASES: &[(QsoOrigin, IngestPolicy)] = &[
            (
                QsoOrigin::Form,
                IngestPolicy {
                    validation: ValidationMode::Reject,
                    effects: PostEffects {
                        plugin_hook: true,
                        enqueue_uploads: true,
                        broadcast_lan: true,
                        broadcast_n1mm: true,
                        psk_report: true,
                        alert_sound: true,
                        propagation_observation: true,
                    },
                },
            ),
            (
                QsoOrigin::Contest,
                IngestPolicy {
                    validation: ValidationMode::Reject,
                    effects: PostEffects {
                        plugin_hook: true,
                        enqueue_uploads: true,
                        broadcast_lan: true,
                        broadcast_n1mm: true,
                        psk_report: false,
                        alert_sound: false,
                        propagation_observation: false,
                    },
                },
            ),
            (
                QsoOrigin::WsjtX,
                IngestPolicy {
                    validation: ValidationMode::Advisory,
                    effects: PostEffects {
                        plugin_hook: true,
                        enqueue_uploads: true,
                        broadcast_lan: true,
                        broadcast_n1mm: false,
                        psk_report: false,
                        alert_sound: false,
                        propagation_observation: false,
                    },
                },
            ),
            (
                QsoOrigin::Js8Call,
                IngestPolicy {
                    validation: ValidationMode::Advisory,
                    effects: PostEffects {
                        plugin_hook: true,
                        enqueue_uploads: true,
                        broadcast_lan: true,
                        broadcast_n1mm: false,
                        psk_report: false,
                        alert_sound: false,
                        propagation_observation: false,
                    },
                },
            ),
            (
                QsoOrigin::LanPeer,
                IngestPolicy {
                    validation: ValidationMode::Advisory,
                    effects: PostEffects {
                        plugin_hook: false,
                        enqueue_uploads: false,
                        broadcast_lan: false,
                        broadcast_n1mm: false,
                        psk_report: false,
                        alert_sound: false,
                        propagation_observation: false,
                    },
                },
            ),
            (
                QsoOrigin::RestApi,
                IngestPolicy {
                    validation: ValidationMode::Reject,
                    effects: PostEffects {
                        plugin_hook: false,
                        enqueue_uploads: false,
                        broadcast_lan: false,
                        broadcast_n1mm: false,
                        psk_report: false,
                        alert_sound: false,
                        propagation_observation: false,
                    },
                },
            ),
        ];

        assert_eq!(CASES.len(), 6, "all 6 QsoOrigin variants must be checked");

        for &(origin, expected) in CASES {
            let actual = origin.policy();
            assert_eq!(
                actual.validation, expected.validation,
                "mismatch on validation mode for {origin:?}"
            );
            assert_eq!(
                actual.effects.plugin_hook, expected.effects.plugin_hook,
                "mismatch on plugin_hook for {origin:?}"
            );
            assert_eq!(
                actual.effects.enqueue_uploads, expected.effects.enqueue_uploads,
                "mismatch on enqueue_uploads for {origin:?}"
            );
            assert_eq!(
                actual.effects.broadcast_lan, expected.effects.broadcast_lan,
                "mismatch on broadcast_lan for {origin:?}"
            );
            assert_eq!(
                actual.effects.broadcast_n1mm, expected.effects.broadcast_n1mm,
                "mismatch on broadcast_n1mm for {origin:?}"
            );
            assert_eq!(
                actual.effects.psk_report, expected.effects.psk_report,
                "mismatch on psk_report for {origin:?}"
            );
            assert_eq!(
                actual.effects.alert_sound, expected.effects.alert_sound,
                "mismatch on alert_sound for {origin:?}"
            );
            assert_eq!(
                actual.effects.propagation_observation, expected.effects.propagation_observation,
                "mismatch on propagation_observation for {origin:?}"
            );
            assert_eq!(
                actual, expected,
                "mismatch on full IngestPolicy for {origin:?}"
            );
        }
    }

    #[test]
    fn external_origins_never_reject() {
        const EXTERNAL_ORIGINS: [QsoOrigin; 3] =
            [QsoOrigin::WsjtX, QsoOrigin::Js8Call, QsoOrigin::LanPeer];

        for origin in EXTERNAL_ORIGINS {
            assert_eq!(
                origin.policy().validation,
                ValidationMode::Advisory,
                "external origin {origin:?} must have ValidationMode::Advisory"
            );
        }
    }

    #[test]
    fn post_effects_default_disables_everything() {
        let default_effects = PostEffects::default();
        assert_eq!(
            default_effects,
            PostEffects {
                plugin_hook: false,
                enqueue_uploads: false,
                broadcast_lan: false,
                broadcast_n1mm: false,
                psk_report: false,
                alert_sound: false,
                propagation_observation: false,
            }
        );
    }

    #[test]
    fn lan_peer_has_no_post_effects() {
        assert_eq!(QsoOrigin::LanPeer.policy().effects, PostEffects::default());
    }
}
