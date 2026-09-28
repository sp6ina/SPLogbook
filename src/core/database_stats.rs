// Statystyki bazy dziennika — wydzielone z database.rs.
use super::LogDatabase;

impl LogDatabase {
    /// Zwraca liczbę QSO pogrupowaną według miesiąca (YYYY-MM), obsługując formaty YYYYMMDD i YYYY-MM-DD.
    pub fn stats_qso_per_month(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT substr(REPLACE(qso_date, '-', ''), 1, 4) || '-' || substr(REPLACE(qso_date, '-', ''), 5, 2) as month, \
             COUNT(*) as cnt \
             FROM qso_records \
             WHERE length(REPLACE(qso_date, '-', '')) >= 6 \
             GROUP BY month ORDER BY month DESC LIMIT 24",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę QSO pogrupowaną według pasma
    pub fn stats_qso_per_band(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT band, COUNT(*) as cnt FROM qso_records GROUP BY band ORDER BY cnt DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę QSO pogrupowaną według emisji
    pub fn stats_qso_per_mode(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT mode, COUNT(*) as cnt FROM qso_records GROUP BY mode ORDER BY cnt DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę QSO pogrupowaną według kontynentu
    pub fn stats_qso_per_continent(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(NULLIF(TRIM(continent), ''), 'UN') as cont, COUNT(*) as cnt \
             FROM qso_records GROUP BY cont ORDER BY cnt DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę unikalnych znaków wywoławczych oraz unikalnych podmiotów DXCC w logu
    pub fn stats_unique_counts(&self) -> rusqlite::Result<(i64, i64)> {
        let unique_calls: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT UPPER(callsign)) FROM qso_records WHERE callsign <> ''",
            [],
            |r| r.get(0),
        )?;
        let unique_dxcc: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT dxcc) FROM qso_records WHERE dxcc IS NOT NULL AND dxcc > 0",
            [],
            |r| r.get(0),
        )?;
        Ok((unique_calls, unique_dxcc))
    }

    /// Zwraca histogram aktywności wg godziny UTC (0-23)
    pub fn stats_activity_by_hour(&self) -> rusqlite::Result<Vec<(u32, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(substr(REPLACE(time_on, ':', ''), 1, 2) AS INTEGER) as hour, COUNT(*) as cnt \
             FROM qso_records WHERE length(REPLACE(time_on, ':', '')) >= 2 GROUP BY hour ORDER BY hour"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca top N krajów wg liczby QSO
    pub fn stats_top_countries(&self, limit: usize) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(country, 'Unknown') as cty, COUNT(*) as cnt FROM qso_records GROUP BY cty ORDER BY cnt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca statystyki QSL (total, lotw_confirmed, eqsl_confirmed, paper_confirmed)
    pub fn stats_qsl_summary(&self) -> rusqlite::Result<(i64, i64, i64, i64)> {
        let total: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))?;
        let lotw: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE lotw_qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        let eqsl: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE eqsl_qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        let paper: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        Ok((total, lotw, eqsl, paper))
    }

    /// Zwraca top N stref CQ wg liczby QSO
    pub fn stats_top_cq_zones(&self, limit: usize) -> rusqlite::Result<Vec<(u32, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT cqz, COUNT(*) as cnt FROM qso_records WHERE cqz IS NOT NULL GROUP BY cqz ORDER BY cnt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }
}
