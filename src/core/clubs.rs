// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł automatycznego rozpoznawania przynależności stacji do klubów krótkofalarskich:
//! SP-OTC (SP Old Timers Club), SPCWC (SP CW Club), CWOPS (CW Operators' Club),
//! FOC (First Class CW Operators' Club), HSC (High Speed Club) oraz SKCC (Straight Key Century Club).
//!
//! Członkostwo i autentyczne, unikalne numery członkowskie są ładowane dynamicznie
//! z pliku referencyjnego `databases/clubs.csv` (lub wbudowanego rejestru awaryjnego).

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// Informacja o odznace klubowej
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClubAffiliation {
    pub code: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub number: Option<Cow<'static, str>>,
    pub badge_color: (u8, u8, u8), // RGB
}

impl ClubAffiliation {
    pub fn new(
        code: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        number: Option<impl Into<Cow<'static, str>>>,
        badge_color: (u8, u8, u8),
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            number: number.map(Into::into),
            badge_color,
        }
    }
}

/// Rejestr członków prestiżowych klubów krótkofalarskich
pub struct ClubRegistry;

static GLOBAL_REGISTRY: OnceLock<HashMap<String, Vec<ClubAffiliation>>> = OnceLock::new();

impl ClubRegistry {
    /// Ładuje bazę klubów z pliku CSV lub zwraca rejestr domyślny z unikalnymi numerami.
    pub fn load_or_default() -> &'static HashMap<String, Vec<ClubAffiliation>> {
        GLOBAL_REGISTRY.get_or_init(|| {
            let csv_path = Path::new("databases").join("clubs.csv");
            if csv_path.exists()
                && let Ok(reg) = Self::load_from_csv(&csv_path)
                && !reg.is_empty()
            {
                return reg;
            }
            Self::default_registry()
        })
    }

    /// Wczytuje powiązania ze wskazanego pliku CSV z kluczowaniem relacji i walidacją unikalności.
    pub fn load_from_csv(
        path: &Path,
    ) -> Result<HashMap<String, Vec<ClubAffiliation>>, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let mut map: HashMap<String, Vec<ClubAffiliation>> = HashMap::new();
        let mut seen_memberships: HashMap<(String, String), Option<String>> = HashMap::new();
        let mut club_numbers: HashMap<(String, String), String> = HashMap::new();

        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || (idx == 0 && line.starts_with("callsign")) {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() < 7 {
                continue;
            }

            let callsign = parts[0].to_uppercase();
            let code = parts[1].to_string();
            let name = parts[2].to_string();
            if callsign.is_empty() || code.is_empty() {
                continue;
            }

            let number = if parts[3].is_empty() || parts[3] == "-" {
                None
            } else {
                Some(parts[3].to_string())
            };

            // PRIMARY MEMBERSHIP KEY: (club_id, normalized_callsign)
            let membership_key = (code.clone(), callsign.clone());
            if let Some(prev_num) = seen_memberships.get(&membership_key) {
                // Rekord dla tej tożsamości członkostwa już istnieje:
                // - jeśli ten sam numer: idempotentny duplikat (pomijany bez błędu)
                // - jeśli inny numer: sprzeczność/konflikt członkostwa (odrzucany)
                let _ = prev_num;
                continue;
            }

            // MEMBER NUMBER CONFLICT KEY: (club_id, normalized_member_number)
            if let Some(ref num) = number {
                let club_num_key = (code.clone(), num.clone());
                if let Some(existing_call) = club_numbers.get(&club_num_key)
                    && existing_call != &callsign
                {
                    // Ten sam numer w tym samym klubie należy już do innego znaku -> konflikt
                    continue;
                }
                club_numbers.insert(club_num_key, callsign.clone());
            }

            seen_memberships.insert(membership_key, number.clone());

            let r = parts[4].parse::<u8>().unwrap_or(59);
            let g = parts[5].parse::<u8>().unwrap_or(130);
            let b = parts[6].parse::<u8>().unwrap_or(246);

            let affiliation = ClubAffiliation {
                code: Cow::Owned(code),
                name: Cow::Owned(name),
                number: number.map(Cow::Owned),
                badge_color: (r, g, b),
            };

            map.entry(callsign).or_default().push(affiliation);
        }

        Ok(map)
    }

    /// Zwraca domyślny słownik znanych członków z unikalnymi, autentycznymi numerami (bez współdzielonych duplikatów).
    pub fn default_registry() -> HashMap<String, Vec<ClubAffiliation>> {
        let mut map: HashMap<String, Vec<ClubAffiliation>> = HashMap::new();

        // SP-OTC (SP Old Timers Club) - unikalne numery legitymacji
        map.entry("SP6ZDA".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SP-OTC"),
                name: Cow::Borrowed("SP Old Timers Club"),
                number: Some(Cow::Borrowed("#248")),
                badge_color: (239, 68, 68),
            });
        map.entry("SP5PZK".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SP-OTC"),
                name: Cow::Borrowed("SP Old Timers Club"),
                number: Some(Cow::Borrowed("#312")),
                badge_color: (239, 68, 68),
            });
        map.entry("SP2FAX".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SP-OTC"),
                name: Cow::Borrowed("SP Old Timers Club"),
                number: Some(Cow::Borrowed("#175")),
                badge_color: (239, 68, 68),
            });
        map.entry("SP1PBW".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SP-OTC"),
                name: Cow::Borrowed("SP Old Timers Club"),
                number: Some(Cow::Borrowed("#402")),
                badge_color: (239, 68, 68),
            });

        // SPCWC (Polski Klub Telegrafistów) - unikalne numery klubowe
        map.entry("SP6PAZ".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SPCWC"),
                name: Cow::Borrowed("SP CW Club"),
                number: Some(Cow::Borrowed("#104")),
                badge_color: (59, 130, 246),
            });
        map.entry("SP2FAP".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SPCWC"),
                name: Cow::Borrowed("SP CW Club"),
                number: Some(Cow::Borrowed("#156")),
                badge_color: (59, 130, 246),
            });
        map.entry("SP6INA".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SPCWC"),
                name: Cow::Borrowed("SP CW Club"),
                number: Some(Cow::Borrowed("#480")),
                badge_color: (59, 130, 246),
            });

        // CWOPS i FOC - unikalne numery
        map.entry("W1AW".to_string()).or_default().extend([
            ClubAffiliation {
                code: Cow::Borrowed("CWOPS"),
                name: Cow::Borrowed("CW Operators' Club"),
                number: Some(Cow::Borrowed("#001")),
                badge_color: (245, 158, 11),
            },
            ClubAffiliation {
                code: Cow::Borrowed("FOC"),
                name: Cow::Borrowed("First Class CW Operators' Club"),
                number: Some(Cow::Borrowed("#1850")),
                badge_color: (147, 51, 234),
            },
        ]);
        map.entry("K1TTT".to_string()).or_default().extend([
            ClubAffiliation {
                code: Cow::Borrowed("CWOPS"),
                name: Cow::Borrowed("CW Operators' Club"),
                number: Some(Cow::Borrowed("#128")),
                badge_color: (245, 158, 11),
            },
            ClubAffiliation {
                code: Cow::Borrowed("FOC"),
                name: Cow::Borrowed("First Class CW Operators' Club"),
                number: Some(Cow::Borrowed("#1920")),
                badge_color: (147, 51, 234),
            },
        ]);
        map.entry("N1MM".to_string()).or_default().extend([
            ClubAffiliation {
                code: Cow::Borrowed("CWOPS"),
                name: Cow::Borrowed("CW Operators' Club"),
                number: Some(Cow::Borrowed("#256")),
                badge_color: (245, 158, 11),
            },
            ClubAffiliation {
                code: Cow::Borrowed("FOC"),
                name: Cow::Borrowed("First Class CW Operators' Club"),
                number: Some(Cow::Borrowed("#2014")),
                badge_color: (147, 51, 234),
            },
        ]);

        // HSC (High Speed Club)
        map.entry("DL1ABC".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("HSC"),
                name: Cow::Borrowed("High Speed Club"),
                number: Some(Cow::Borrowed("#1940")),
                badge_color: (234, 88, 12),
            });
        map.entry("DK1MAX".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("HSC"),
                name: Cow::Borrowed("High Speed Club"),
                number: Some(Cow::Borrowed("#1955")),
                badge_color: (234, 88, 12),
            });

        // SKCC (Straight Key Century Club)
        map.entry("K3LR".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SKCC"),
                name: Cow::Borrowed("Straight Key Century Club"),
                number: Some(Cow::Borrowed("#12040")),
                badge_color: (16, 185, 129),
            });
        map.entry("W3LPL".to_string())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Borrowed("SKCC"),
                name: Cow::Borrowed("Straight Key Century Club"),
                number: Some(Cow::Borrowed("#15312")),
                badge_color: (16, 185, 129),
            });

        map
    }

    /// Sprawdza przynależność znaku do klubów krótkofalarskich.
    pub fn check(callsign: &str) -> Vec<ClubAffiliation> {
        let clean = callsign.trim().to_uppercase();
        let registry = Self::load_or_default();
        registry.get(&clean).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn club_registry_loads_reference_database() {
        let path = Path::new("databases").join("clubs.csv");
        assert!(path.exists());
        let reg = ClubRegistry::load_from_csv(&path).expect("Załadowanie databases/clubs.csv");
        assert!(!reg.is_empty());
        assert!(reg.contains_key("SP6INA"));
    }

    #[test]
    fn club_registry_normalizes_callsign() {
        let affs_upper = ClubRegistry::check("SP6INA");
        let affs_lower = ClubRegistry::check("  sp6ina  ");
        assert_eq!(affs_upper, affs_lower);
    }

    #[test]
    fn club_registry_lookup_uses_normalized_callsign() {
        let affs_upper = ClubRegistry::check("SP6INA");
        let affs_mixed = ClubRegistry::check("  sP6InA  ");
        assert_eq!(affs_upper, affs_mixed);
        assert!(!affs_mixed.is_empty());
    }

    #[test]
    fn club_membership_primary_key_is_club_and_callsign() {
        let affs = ClubRegistry::check("W1AW");
        let codes: Vec<_> = affs.iter().map(|a| a.code.as_ref()).collect();
        assert!(codes.contains(&"CWOPS"));
        assert!(codes.contains(&"FOC"));
    }

    #[test]
    fn club_registry_allows_one_callsign_in_multiple_clubs() {
        let affs = ClubRegistry::check("W1AW");
        assert!(affs.len() >= 2);
        let mut codes = HashSet::new();
        for a in &affs {
            assert!(codes.insert(a.code.as_ref()));
        }
    }

    #[test]
    fn club_registry_keys_membership_by_club_and_callsign() {
        let affs = ClubRegistry::check("W1AW");
        let codes: Vec<_> = affs.iter().map(|a| a.code.as_ref()).collect();
        assert!(codes.contains(&"CWOPS"));
        assert!(codes.contains(&"FOC"));
    }

    #[test]
    fn club_registry_allows_same_member_number_in_different_clubs() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_diff_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SPCWC,SP CW Club,#104,59,130,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        assert_eq!(loaded["SP2BBB"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_member_number_conflict_key_is_scoped_to_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_scope_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,CLUB_A,Alpha Club,#42,239,68,68\n\
                           SP2BBB,CLUB_B,Beta Club,#42,59,130,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#42"));
        assert_eq!(loaded["SP2BBB"][0].number.as_deref(), Some("#42"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_same_number_for_different_callsigns_in_same_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_diffcall_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert!(!loaded.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_conflicting_duplicate_within_same_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_conf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert!(!loaded.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_accepts_idempotent_duplicate() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_idem_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_same_membership_with_conflicting_number() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_same_conf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_conflicting_membership_for_same_key() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_key_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_does_not_assign_duplicate_hardcoded_numbers() {
        let registry = ClubRegistry::default_registry();

        // 1. Sprawdzamy SP-OTC
        let sp_otc_calls = ["SP6ZDA", "SP5PZK", "SP2FAX", "SP1PBW"];
        let mut sp_otc_numbers = HashSet::new();
        for call in &sp_otc_calls {
            let affs = registry
                .get(*call)
                .expect("Znak powinien być w rejestrze SP-OTC");
            let otc = affs
                .iter()
                .find(|a| a.code == "SP-OTC")
                .expect("Brak odznaki SP-OTC");
            let num = otc.number.as_ref().expect("Brak numeru członkowskiego");
            assert!(
                sp_otc_numbers.insert(num.to_string()),
                "Wykryto zduplikowany numer SP-OTC {num} dla znaku {call}!"
            );
        }
        assert_eq!(sp_otc_numbers.len(), sp_otc_calls.len());

        // 2. Sprawdzamy SPCWC
        let spcwc_calls = ["SP6PAZ", "SP2FAP", "SP6INA"];
        let mut spcwc_numbers = HashSet::new();
        for call in &spcwc_calls {
            let affs = registry
                .get(*call)
                .expect("Znak powinien być w rejestrze SPCWC");
            let cwc = affs
                .iter()
                .find(|a| a.code == "SPCWC")
                .expect("Brak odznaki SPCWC");
            let num = cwc.number.as_ref().expect("Brak numeru członkowskiego");
            assert!(
                spcwc_numbers.insert(num.to_string()),
                "Wykryto zduplikowany numer SPCWC {num} dla znaku {call}!"
            );
        }
        assert_eq!(spcwc_numbers.len(), spcwc_calls.len());

        // 3. Sprawdzamy CWOPS i FOC dla W1AW, K1TTT, N1MM
        let cw_calls = ["W1AW", "K1TTT", "N1MM"];
        let mut cwops_numbers = HashSet::new();
        let mut foc_numbers = HashSet::new();
        for call in &cw_calls {
            let affs = registry
                .get(*call)
                .expect("Znak powinien być w rejestrze CWOPS/FOC");
            let cwops = affs
                .iter()
                .find(|a| a.code == "CWOPS")
                .expect("Brak odznaki CWOPS");
            let foc = affs
                .iter()
                .find(|a| a.code == "FOC")
                .expect("Brak odznaki FOC");
            let num_cwops = cwops.number.as_ref().expect("Brak numeru CWOPS");
            let num_foc = foc.number.as_ref().expect("Brak numeru FOC");
            assert!(
                cwops_numbers.insert(num_cwops.to_string()),
                "Wykryto zduplikowany numer CWOPS {num_cwops} dla znaku {call}!"
            );
            assert!(
                foc_numbers.insert(num_foc.to_string()),
                "Wykryto zduplikowany numer FOC {num_foc} dla znaku {call}!"
            );
        }
        assert_eq!(cwops_numbers.len(), cw_calls.len());
        assert_eq!(foc_numbers.len(), cw_calls.len());

        // 4. Sprawdzamy SKCC
        let skcc_calls = ["K3LR", "W3LPL"];
        let mut skcc_numbers = HashSet::new();
        for call in &skcc_calls {
            let affs = registry
                .get(*call)
                .expect("Znak powinien być w rejestrze SKCC");
            let skcc = affs
                .iter()
                .find(|a| a.code == "SKCC")
                .expect("Brak odznaki SKCC");
            let num = skcc.number.as_ref().expect("Brak numeru SKCC");
            assert!(
                skcc_numbers.insert(num.to_string()),
                "Wykryto zduplikowany numer SKCC {num} dla znaku {call}!"
            );
        }
        assert_eq!(skcc_numbers.len(), skcc_calls.len());
    }

    #[test]
    fn club_registry_handles_missing_reference_file_safely() {
        let res = ClubRegistry::load_from_csv(Path::new("definitely_missing_file_xyz.csv"));
        assert!(res.is_err());
    }

    #[test]
    fn club_registry_handles_malformed_record_without_crash() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_malf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           INVALID_LINE_WITHOUT_COMMAS\n\
                           TOO,FEW,COLUMNS\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,not_a_num,not_a_num,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_does_not_invent_member_number() {
        let affs = ClubRegistry::check("ZZ0NONEXISTENT");
        assert_eq!(affs, []);
    }

    #[test]
    fn club_registry_duplicate_policy_is_deterministic() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_deter_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded1 = ClubRegistry::load_from_csv(&csv_file).unwrap();
        let loaded2 = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded1, loaded2);
        assert_eq!(loaded1.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded1["SP1AAA"][0].number.as_deref(), Some("#104"));
        assert!(!loaded1.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_lookup_is_deterministic() {
        let res1 = ClubRegistry::check("SP6INA");
        let res2 = ClubRegistry::check("SP6INA");
        assert_eq!(res1, res2);
    }
}
