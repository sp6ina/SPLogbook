// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use super::schema::{ORDER_CHRONO_DESC, QSO_SQL, collect_qsos, qso_from_row};
use crate::core::qso::QsoRecord;
use crate::core::text::escape_like;
use rusqlite::Result;

/// Zaawansowany filtr wyszukiwania łączności
#[derive(Debug, Clone, Default)]
pub struct AdvancedQsoFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub bands: Vec<String>,
    pub modes: Vec<String>,
    pub lotw_confirmed: Option<bool>,
    pub eqsl_confirmed: Option<bool>,
    pub qsl_rcvd: Option<bool>,
    pub callsign_query: Option<String>,
    pub journal_id: Option<String>,
}

impl LogDatabase {
    /// Wyszukuje duplikaty łączności w całym logu
    /// Grupuje po znaku, paśmie i emisji (opcjonalnie również po dacie)
    pub fn find_duplicate_qsos(&self, match_same_day: bool) -> Result<Vec<Vec<QsoRecord>>> {
        let partition_by = if match_same_day {
            "callsign, band, mode, qso_date"
        } else {
            "callsign, band, mode"
        };

        let query_sql = format!(
            "SELECT {} FROM (
                SELECT {},
                       COUNT(*) OVER (PARTITION BY {partition_by}) AS dup_cnt
                FROM qso_records
            )
            WHERE dup_cnt > 1
            ORDER BY callsign ASC, band ASC, mode ASC, qso_date ASC, time_on ASC, id ASC",
            QSO_SQL.select_columns, QSO_SQL.select_columns
        );

        let mut stmt = self.conn.prepare(&query_sql)?;
        let rows = stmt.query_map([], qso_from_row)?;

        let mut groups: Vec<Vec<QsoRecord>> = Vec::new();
        let mut current_key: Option<(String, String, String, String)> = None;
        let mut current_cluster: Vec<QsoRecord> = Vec::new();

        for row in rows {
            let qso = row?;
            let key = (
                qso.callsign.clone(),
                qso.band.clone(),
                qso.mode.clone(),
                if match_same_day {
                    qso.qso_date.clone()
                } else {
                    String::new()
                },
            );
            if current_key.as_ref() == Some(&key) {
                current_cluster.push(qso);
            } else {
                if current_cluster.len() > 1 {
                    groups.push(std::mem::take(&mut current_cluster));
                } else {
                    current_cluster.clear();
                }
                current_key = Some(key);
                current_cluster.push(qso);
            }
        }
        if current_cluster.len() > 1 {
            groups.push(current_cluster);
        }

        Ok(groups)
    }

    /// Wyszukiwanie łączności z wieloma kryteriami (zaawansowane filtrowanie).
    /// Wszystkie wartości pochodzące od użytkownika są przekazywane jako parametry
    /// wiązane (nie string-concat), a wzorce LIKE mają escapowane znaki wieloznaczne
    /// `%`/`_`, żeby wyszukiwane teksty użytkownika nie działały jak wildcardy.
    pub fn search_qsos_advanced(&self, filter: &AdvancedQsoFilter) -> Result<Vec<QsoRecord>> {
        let mut sql = format!(
            "SELECT {} FROM qso_records WHERE 1=1",
            QSO_SQL.select_columns
        );
        let mut conditions: Vec<String> = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref jid) = filter.journal_id {
            conditions.push(format!("journal_id = ?{}", values.len() + 1));
            values.push(Box::new(jid.clone()));
        }
        if let Some(ref from) = filter.date_from {
            if !from.is_empty() {
                conditions.push(format!("qso_date >= ?{}", values.len() + 1));
                values.push(Box::new(from.clone()));
            }
        }
        if let Some(ref to) = filter.date_to {
            if !to.is_empty() {
                conditions.push(format!("qso_date <= ?{}", values.len() + 1));
                values.push(Box::new(to.clone()));
            }
        }
        if !filter.bands.is_empty() {
            let placeholders: Vec<String> = filter
                .bands
                .iter()
                .map(|b| {
                    values.push(Box::new(b.clone()));
                    format!("?{}", values.len())
                })
                .collect();
            conditions.push(format!("band IN ({})", placeholders.join(",")));
        }
        if !filter.modes.is_empty() {
            let placeholders: Vec<String> = filter
                .modes
                .iter()
                .map(|m| {
                    values.push(Box::new(m.clone()));
                    format!("?{}", values.len())
                })
                .collect();
            conditions.push(format!("mode IN ({})", placeholders.join(",")));
        }
        if let Some(lotw) = filter.lotw_confirmed {
            conditions.push(if lotw {
                "lotw_qsl_rcvd = 'Y'".to_string()
            } else {
                "(lotw_qsl_rcvd IS NULL OR lotw_qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(eqsl) = filter.eqsl_confirmed {
            conditions.push(if eqsl {
                "eqsl_qsl_rcvd = 'Y'".to_string()
            } else {
                "(eqsl_qsl_rcvd IS NULL OR eqsl_qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(qsl) = filter.qsl_rcvd {
            conditions.push(if qsl {
                "qsl_rcvd = 'Y'".to_string()
            } else {
                "(qsl_rcvd IS NULL OR qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(ref query) = filter.callsign_query {
            let query_upper = query.trim().to_uppercase();
            if !query_upper.is_empty() {
                // Escapuje % i _ (znaki specjalne LIKE) znakiem ucieczki '\', żeby wpisany
                // przez użytkownika tekst nie działał jak wzorzec wildcard.
                let escaped = escape_like(&query_upper);
                let like_pattern = format!("%{escaped}%");
                values.push(Box::new(like_pattern.clone()));
                let p1 = values.len();
                values.push(Box::new(like_pattern.clone()));
                let p2 = values.len();
                values.push(Box::new(like_pattern));
                let p3 = values.len();
                conditions.push(format!(
                    "(callsign LIKE ?{p1} ESCAPE '\\' OR name LIKE ?{p2} ESCAPE '\\' OR comment LIKE ?{p3} ESCAPE '\\')"
                ));
            }
        }

        for c in conditions {
            sql.push_str(" AND ");
            sql.push_str(&c);
        }

        sql.push_str(&format!(" ORDER BY {ORDER_CHRONO_DESC}"));

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(std::convert::AsRef::as_ref).collect();
        collect_qsos(&mut stmt, param_refs.as_slice())
    }
}
