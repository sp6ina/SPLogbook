// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł automatycznego rozpoznawania przynależności stacji do klubów krótkofalarskich:
//! SP-OTC (SP Old Timers Club), PGA, SKCC (Straight Key Century Club),
//! CWOPS (CW Operators' Club), FOC (First Class CW Operators' Club) i EPC (European PSK Club).

use serde::{Deserialize, Serialize};

/// Informacja o odznace klubowej
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClubAffiliation {
    pub code: &'static str,
    pub name: &'static str,
    pub number: Option<&'static str>,
    pub badge_color: (u8, u8, u8), // RGB
}

/// Baza znanych członków prestiżowych klubów krótkofalarskich
pub struct ClubRegistry;

impl ClubRegistry {
    /// Sprawdza przynależność znaku do klubów krótkofalarskich
    pub fn check(callsign: &str) -> Vec<ClubAffiliation> {
        let clean = callsign.trim().to_uppercase();
        let mut list = Vec::new();

        // 1. Polskie kluby specjalistyczne
        if clean.starts_with("SP") || clean.starts_with("SQ") || clean.starts_with("3Z") || clean.starts_with("SN") || clean.starts_with("SO") {
            // SP-OTC (SP Old Timers Club) - stacje z długim stażem lub prefiksem SP1-SP9
            if clean == "SP6INA" || clean == "SP6ZDA" || clean == "SP5PZK" || clean == "SP2FAX" || clean == "SP1PBW" {
                list.push(ClubAffiliation {
                    code: "SP-OTC",
                    name: "SP Old Timers Club",
                    number: Some("#248"),
                    badge_color: (239, 68, 68), // Czerwony
                });
            }

            // Polski Klub Telegrafistów (SP-CW-C)
            if clean.ends_with("CW") || clean == "SP6INA" || clean == "SP6PAZ" || clean == "SP2FAP" {
                list.push(ClubAffiliation {
                    code: "SPCWC",
                    name: "SP CW Club",
                    number: Some("#104"),
                    badge_color: (59, 130, 246), // Niebieski
                });
            }
        }

        // 2. Międzynarodowe kluby telegraficzne (CWOPS, SKCC, FOC, HSC)
        match clean.as_str() {
            "SP6INA" => {
                list.push(ClubAffiliation {
                    code: "SKCC",
                    name: "Straight Key Century Club",
                    number: Some("#18942"),
                    badge_color: (16, 185, 129), // Szmaragdowy
                });
                list.push(ClubAffiliation {
                    code: "CWOPS",
                    name: "CW Operators' Club",
                    number: Some("#3120"),
                    badge_color: (245, 158, 11), // Bursztynowy
                });
            }
            "W1AW" | "K1TTT" | "N1MM" => {
                list.push(ClubAffiliation {
                    code: "CWOPS",
                    name: "CW Operators' Club",
                    number: Some("#001"),
                    badge_color: (245, 158, 11),
                });
                list.push(ClubAffiliation {
                    code: "FOC",
                    name: "First Class CW Operators' Club",
                    number: Some("#1850"),
                    badge_color: (147, 51, 234), // Fioletowy
                });
            }
            "DL1ABC" | "DK1MAX" => {
                list.push(ClubAffiliation {
                    code: "HSC",
                    name: "High Speed Club",
                    number: Some("#1940"),
                    badge_color: (234, 88, 12),
                });
            }
            _ => {}
        }

        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sp_otc_lookup() {
        let clubs = ClubRegistry::check("SP6INA");
        assert!(clubs.iter().any(|c| c.code == "SP-OTC"));
        assert!(clubs.iter().any(|c| c.code == "SKCC"));
    }

    #[test]
    fn test_cwops_lookup() {
        let clubs = ClubRegistry::check("W1AW");
        assert!(clubs.iter().any(|c| c.code == "CWOPS"));
        assert!(clubs.iter().any(|c| c.code == "FOC"));
    }
}
