use crate::core::qso::QsoRecord;
use std::collections::HashSet;

pub type ScoringFn = fn(&[QsoRecord], u32, u8) -> (u32, u32, u32);

pub struct ContestRule {
    pub name: &'static str,
    pub exchange_format: &'static str,
    pub bands: &'static [&'static str],
    pub scoring_fn: ScoringFn,
}

pub fn calculate_score(rule: &ContestRule, qsos: &[QsoRecord], my_dxcc: u32, my_cqzone: u8) -> (u32, u32, u32) {
    (rule.scoring_fn)(qsos, my_dxcc, my_cqzone)
}

pub fn detect_duplicate(_rule: &ContestRule, qso: &QsoRecord, log: &[QsoRecord]) -> bool {
    for prev in log {
        if prev.callsign == qso.callsign && prev.band == qso.band {
            return true;
        }
    }
    false
}

fn score_sp_dx(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(dxcc) = q.dxcc {
            mults.insert(dxcc);
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_cqww(qsos: &[QsoRecord], my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if q.dxcc == Some(my_dxcc) {
            pts += 1; // 1 pt/same continent / same dxcc approximation
        } else {
            pts += 3; // 3 pts/diff DXCC
        }
        if let Some(z) = q.cqz {
            mults.insert(format!("Z{}", z));
        }
        if let Some(d) = q.dxcc {
            mults.insert(format!("D{}", d));
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_arrl_dx(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3; // 3 pts per QSO
        if let Some(state) = &q.state {
            mults.insert(state.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_wpx(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 2; // 1/2/3 pts approx
        let prefix: String = q.callsign.chars().filter(|c| c.is_alphanumeric()).take(3).collect();
        mults.insert(prefix);
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_vhf(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 100; // 1pt/km approx
        if let Some(grid) = &q.gridsquare {
            mults.insert(grid.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_iaru(qsos: &[QsoRecord], _my_dxcc: u32, my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if q.cqz == Some(my_cqzone as u32) {
            pts += 1;
        } else {
            pts += 3;
        }
        if let Some(z) = q.cqz {
            mults.insert(z.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_wae(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_iota(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if let Some(i) = &q.iota {
            pts += 15;
            mults.insert(i.clone());
        } else {
            pts += 3;
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_field_day(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += if q.mode == "CW" || q.mode == "DIGI" { 2 } else { 1 };
        if let Some(state) = &q.state {
            mults.insert(state.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_sac(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_eu_hf(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if q.continent.as_deref() == Some("EU") {
            pts += 1;
        } else {
            pts += 2;
        }
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_ok_om(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_king_spain(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += if q.continent.as_deref() == Some("EU") { 1 } else { 3 };
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_all_asian(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_jidx(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_rda(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 2;
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_marconi(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_bartg(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 6;
        if let Some(d) = q.dxcc {
            mults.insert(format!("{}-{}", d, q.band));
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_na_rtty(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

fn score_ukrainian(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let m = mults.len() as u32;
    (pts, m, pts * m)
}

pub const RULES: &[ContestRule] = &[
    ContestRule { name: "SP DX Contest", exchange_format: "RST + Serial", bands: &["80m", "40m", "20m", "15m", "10m"], scoring_fn: score_sp_dx },
    ContestRule { name: "CQ World Wide DX Contest (CW)", exchange_format: "RST + CQ Zone", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_cqww },
    ContestRule { name: "CQ World Wide DX Contest (SSB)", exchange_format: "RST + CQ Zone", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_cqww },
    ContestRule { name: "CQ WPX Contest", exchange_format: "RST + Serial", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_wpx },
    ContestRule { name: "ARRL International DX", exchange_format: "RST + Power", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_arrl_dx },
    ContestRule { name: "VHFUHF (SP)", exchange_format: "RST + Grid", bands: &["2m", "70cm"], scoring_fn: score_vhf },
    ContestRule { name: "IARU HF Championship", exchange_format: "RST + Zone/HQ", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_iaru },
    ContestRule { name: "WAE DX Contest", exchange_format: "RST + Serial", bands: &["80m", "40m", "20m", "15m", "10m"], scoring_fn: score_wae },
    ContestRule { name: "RSGB IOTA Contest", exchange_format: "RST + Serial + IOTA", bands: &["80m", "40m", "20m", "15m", "10m"], scoring_fn: score_iota },
    ContestRule { name: "ARRL Field Day", exchange_format: "Category + ARRL Section", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_field_day },
    ContestRule { name: "Scandinavian Activity Contest", exchange_format: "RST + Serial", bands: &["80m", "40m", "20m", "15m", "10m"], scoring_fn: score_sac },
    ContestRule { name: "EU HF Championship", exchange_format: "RST + Year of 1st License", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_eu_hf },
    ContestRule { name: "OK/OM DX Contest", exchange_format: "RST + District", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_ok_om },
    ContestRule { name: "King of Spain DX Contest", exchange_format: "RST + Province", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_king_spain },
    ContestRule { name: "All Asian DX Contest", exchange_format: "RST + Age", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_all_asian },
    ContestRule { name: "JIDX Contest", exchange_format: "RST + Prefecture", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_jidx },
    ContestRule { name: "RDA Contest", exchange_format: "RST + District", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_rda },
    ContestRule { name: "Marconi Memorial HF", exchange_format: "RST + Serial", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_marconi },
    ContestRule { name: "BARTG HF RTTY", exchange_format: "RST + Serial + Time", bands: &["80m", "40m", "20m", "15m", "10m"], scoring_fn: score_bartg },
    ContestRule { name: "NA RTTY Sprint", exchange_format: "RST + Serial + Name + QTH", bands: &["80m", "40m", "20m"], scoring_fn: score_na_rtty },
    ContestRule { name: "Ukrainian DX Contest", exchange_format: "RST + Oblast", bands: &["160m", "80m", "40m", "20m", "15m", "10m"], scoring_fn: score_ukrainian },
];
