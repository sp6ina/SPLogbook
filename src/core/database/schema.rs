// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::borrow::Cow;
use std::sync::LazyLock;

macro_rules! qso_columns {
    ($($field:ident => $column:literal),+ $(,)?) => {
        pub(super) const QSO_DATA_COLUMNS: &[&str] = &[$($column),+];
        pub(super) const QSO_DATA_COLUMN_COUNT: usize = QSO_DATA_COLUMNS.len();
        pub(super) fn bind_params(qso: &QsoRecord) -> [&dyn rusqlite::ToSql; QSO_DATA_COLUMN_COUNT] {
            [$(&qso.$field),+]
        }
        pub(super) fn qso_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<QsoRecord> {
            let mut next = 0usize;
            let mut column = || { let index = next; next += 1; index };
            Ok(QsoRecord { id: Some(row.get(column())?), $($field: row.get(column())?),+ })
        }
        const _: fn(&QsoRecord) = |q| { let QsoRecord { id: _, $($field: _),+ } = q; };
    };
}

qso_columns! {
    callsign => "callsign",
    band => "band",
    mode => "mode",
    submode => "submode",
    qso_date => "qso_date",
    time_on => "time_on",
    time_off => "time_off",
    freq => "freq",
    freq_rx => "freq_rx",
    rst_sent => "rst_sent",
    rst_rcvd => "rst_rcvd",
    name => "name",
    qth => "qth",
    gridsquare => "gridsquare",
    state => "state",
    iota => "iota",
    sota_ref => "sota_ref",
    pota_ref => "pota_ref",
    pga_ref => "pga_ref",
    dxcc => "dxcc",
    country => "country",
    continent => "continent",
    cqz => "cqz",
    ituz => "ituz",
    comment => "comment",
    qsl_via => "qsl_via",
    qsl_manager => "qsl_manager",
    qsl_sent => "qsl_sent",
    qsl_rcvd => "qsl_rcvd",
    qsl_sent_date => "qsl_sent_date",
    qsl_rcvd_date => "qsl_rcvd_date",
    lotw_qsl_sent => "lotw_qsl_sent",
    lotw_qsl_rcvd => "lotw_qsl_rcvd",
    lotw_qslrdate => "lotw_qslrdate",
    eqsl_qsl_sent => "eqsl_qsl_sent",
    eqsl_qsl_rcvd => "eqsl_qsl_rcvd",
    eqsl_qslrdate => "eqsl_qslrdate",
    clublog_upload_status => "clublog_upload_status",
    qrzcom_upload_status => "qrzcom_upload_status",
    sat_name => "sat_name",
    sat_mode => "sat_mode",
    prop_mode => "prop_mode",
    srx => "srx",
    stx => "stx",
    srx_string => "srx_string",
    stx_string => "stx_string",
    my_gridsquare => "my_gridsquare",
    my_state => "my_state",
    my_pota_ref => "my_pota_ref",
    my_sota_ref => "my_sota_ref",
    vucc_grids => "vucc_grids",
    audio_file => "audio_file",
    journal_id => "journal_id",
}

pub(super) struct QsoSql {
    pub select_columns: String,
    pub insert: String,
    pub insert_with_id: String,
    pub update: String,
}

impl QsoSql {
    fn new() -> Self {
        let columns_joined = QSO_DATA_COLUMNS.join(", ");
        let select_columns = format!("id, {columns_joined}");
        let placeholders = (1..=QSO_DATA_COLUMN_COUNT)
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let insert = format!("INSERT INTO qso_records ({columns_joined}) VALUES ({placeholders})");
        let insert_with_id = format!(
            "INSERT INTO qso_records ({columns_joined}, id) VALUES ({placeholders}, ?{})",
            QSO_DATA_COLUMN_COUNT + 1
        );
        let update_set = QSO_DATA_COLUMNS
            .iter()
            .enumerate()
            .map(|(i, col)| format!("{col} = ?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let update = format!(
            "UPDATE qso_records SET {update_set} WHERE id = ?{}",
            QSO_DATA_COLUMN_COUNT + 1
        );
        Self {
            select_columns,
            insert,
            insert_with_id,
            update,
        }
    }
}

pub(super) static QSO_SQL: LazyLock<QsoSql> = LazyLock::new(QsoSql::new);

pub(super) fn for_storage(qso: &QsoRecord) -> Cow<'_, QsoRecord> {
    let call_upper = qso.callsign.to_uppercase();
    let mode_upper = qso.mode.to_uppercase();
    let call_matches = qso.callsign == call_upper;
    let mode_matches = qso.mode == mode_upper;
    let journal_matches = qso.journal_id.is_some();

    if call_matches && mode_matches && journal_matches {
        Cow::Borrowed(qso)
    } else {
        let mut owned = qso.clone();
        if !call_matches {
            owned.callsign = call_upper;
        }
        if !mode_matches {
            owned.mode = mode_upper;
        }
        if !journal_matches {
            owned.journal_id = Some("DEFAULT".to_string());
        }
        Cow::Owned(owned)
    }
}

pub(super) const ORDER_CHRONO_DESC: &str = "REPLACE(qso_date, '-', '') DESC, SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6) DESC, id DESC";
pub(super) const ORDER_CHRONO_ASC: &str = "REPLACE(qso_date, '-', '') ASC, SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6) ASC, id ASC";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QslChannel {
    Lotw,
    Eqsl,
}

impl QslChannel {
    pub(super) const fn rcvd_column(self) -> &'static str {
        match self {
            Self::Lotw => "lotw_qsl_rcvd",
            Self::Eqsl => "eqsl_qsl_rcvd",
        }
    }

    pub(super) const fn rdate_column(self) -> &'static str {
        match self {
            Self::Lotw => "lotw_qslrdate",
            Self::Eqsl => "eqsl_qslrdate",
        }
    }
}

pub(super) fn collect_qsos<P: rusqlite::Params>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
) -> rusqlite::Result<Vec<QsoRecord>> {
    let rows = stmt.query_map(params, qso_from_row)?;
    rows.collect()
}
