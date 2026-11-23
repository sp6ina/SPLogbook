// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Read-only plugin functions: worked before, DXCC of a callsign, band and band plan
//! mode of a frequency, UTC time, interface language and a small per-plugin memory.
//! Every hook call runs within a time budget and a budget of log queries.

use crate::core::bandplan::BandPlan;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant};

/// Longest a single hook call may run before the engine stops it.
pub const HOOK_TIME_BUDGET: Duration = Duration::from_millis(50);
/// Log queries (`worked_before`, `dxcc_of`) allowed in a single hook call.
pub const MAX_QUERIES_PER_CALL: u32 = 200;
/// Keys a plugin may keep in its memory, and the longest value.
pub const MAX_MEMORY_KEYS: usize = 64;
pub const MAX_MEMORY_VALUE_CHARS: usize = 1024;

/// DXCC entity of a callsign as seen by plugins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxccInfo {
    pub country: String,
    pub dxcc: i64,
    pub continent: String,
    pub cq_zone: i64,
    pub itu_zone: i64,
}

/// Questions plugins may ask the application. `None` means the answer is not
/// available right now (e.g. the log is busy), which scripts see as "no".
pub trait PluginQueries: Send + Sync {
    fn worked_before(&self, callsign: &str, band: Option<&str>, mode: Option<&str>)
    -> Option<bool>;
    fn dxcc_of(&self, callsign: &str) -> Option<DxccInfo>;
}

/// Shared state behind the read-only functions.
pub(crate) struct ApiState {
    queries: RwLock<Option<Arc<dyn PluginQueries>>>,
    bandplan: RwLock<Arc<BandPlan>>,
    language: RwLock<String>,
    current_plugin: Mutex<String>,
    deadline: Mutex<Option<Instant>>,
    queries_left: AtomicU32,
    memory: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl Default for ApiState {
    fn default() -> Self {
        Self {
            queries: RwLock::new(None),
            bandplan: RwLock::new(Arc::new(BandPlan::iaru_region1().clone())),
            language: RwLock::new("pl".to_string()),
            current_plugin: Mutex::new(String::new()),
            deadline: Mutex::new(None),
            queries_left: AtomicU32::new(MAX_QUERIES_PER_CALL),
            memory: Mutex::new(HashMap::new()),
        }
    }
}

impl ApiState {
    pub(crate) fn set_queries(&self, queries: Arc<dyn PluginQueries>) {
        *self.queries.write().unwrap_or_else(PoisonError::into_inner) = Some(queries);
    }

    pub(crate) fn set_bandplan(&self, plan: BandPlan) {
        *self
            .bandplan
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Arc::new(plan);
    }

    pub(crate) fn set_language(&self, code: &str) {
        let mut language = self
            .language
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if *language != code {
            *language = code.to_string();
        }
    }

    /// Starts the budget of one hook call made by `plugin`.
    pub(crate) fn begin_call(&self, plugin: &str) {
        plugin.clone_into(
            &mut self
                .current_plugin
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        *self.deadline.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(Instant::now() + HOOK_TIME_BUDGET);
        self.queries_left
            .store(MAX_QUERIES_PER_CALL, Ordering::Relaxed);
    }

    pub(crate) fn end_call(&self) {
        *self.deadline.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    pub(crate) fn out_of_time(&self) -> bool {
        self.deadline
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some_and(|deadline| Instant::now() > deadline)
    }

    /// Takes one query from the budget; `false` once it is used up.
    fn take_query(&self) -> bool {
        self.queries_left
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                left.checked_sub(1)
            })
            .is_ok()
    }

    fn queries(&self) -> Option<Arc<dyn PluginQueries>> {
        if !self.take_query() {
            return None;
        }
        self.queries
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn worked_before(&self, callsign: &str, band: Option<&str>, mode: Option<&str>) -> bool {
        let callsign = callsign.trim().to_uppercase();
        if callsign.is_empty() {
            return false;
        }
        self.queries()
            .and_then(|q| q.worked_before(&callsign, band, mode))
            .unwrap_or(false)
    }

    fn dxcc_of(&self, callsign: &str) -> rhai::Map {
        let mut map = rhai::Map::new();
        if let Some(info) = self.queries().and_then(|q| q.dxcc_of(callsign.trim())) {
            map.insert("country".into(), info.country.into());
            map.insert("dxcc".into(), info.dxcc.into());
            map.insert("continent".into(), info.continent.into());
            map.insert("cq_zone".into(), info.cq_zone.into());
            map.insert("itu_zone".into(), info.itu_zone.into());
        }
        map
    }

    fn bandplan_mode(&self, freq_mhz: f64) -> String {
        if !freq_mhz.is_finite() || freq_mhz <= 0.0 {
            return String::new();
        }
        let hz = (freq_mhz * 1_000_000.0).round() as u64;
        self.bandplan
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .suggested_mode(hz)
            .unwrap_or_default()
            .to_string()
    }

    fn memory_get(&self, key: &str) -> String {
        let plugin = self
            .current_plugin
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        self.memory
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&plugin)
            .and_then(|values| values.get(key))
            .cloned()
            .unwrap_or_default()
    }

    /// Stores a value for the current plugin; a new key over the limit is ignored and an
    /// empty value removes the key.
    fn memory_set(&self, key: &str, value: &str) {
        let plugin = self
            .current_plugin
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let mut memory = self.memory.lock().unwrap_or_else(PoisonError::into_inner);
        let values = memory.entry(plugin).or_default();
        if value.is_empty() {
            values.remove(key);
            return;
        }
        if !values.contains_key(key) && values.len() >= MAX_MEMORY_KEYS {
            return;
        }
        values.insert(
            key.to_string(),
            value.chars().take(MAX_MEMORY_VALUE_CHARS).collect(),
        );
    }
}

/// Registers the read-only functions and stops scripts that run past their budget.
pub(crate) fn register(engine: &mut rhai::Engine, api: &Arc<ApiState>) {
    let a = Arc::clone(api);
    engine.on_progress(move |operations| {
        (operations % 256 == 0 && a.out_of_time())
            .then(|| rhai::Dynamic::from("time budget exceeded".to_string()))
    });

    let a = Arc::clone(api);
    engine.register_fn("worked_before", move |call_sign: &str| {
        a.worked_before(call_sign, None, None)
    });
    let a = Arc::clone(api);
    engine.register_fn(
        "worked_before",
        move |call_sign: &str, band: &str, mode: &str| {
            a.worked_before(call_sign, Some(band), Some(mode))
        },
    );
    let a = Arc::clone(api);
    engine.register_fn("dxcc_of", move |call_sign: &str| a.dxcc_of(call_sign));
    let a = Arc::clone(api);
    engine.register_fn("bandplan_mode", move |freq_mhz: f64| {
        a.bandplan_mode(freq_mhz)
    });
    engine.register_fn("band_of", |freq_mhz: f64| {
        crate::core::bandplan::freq_khz_to_band(freq_mhz * 1000.0)
            .unwrap_or_default()
            .to_string()
    });
    let a = Arc::clone(api);
    engine.register_fn("language", move || {
        a.language
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    });
    engine.register_fn("utc_date", || {
        jiff::Timestamp::now().strftime("%Y-%m-%d").to_string()
    });
    engine.register_fn("utc_time", || {
        jiff::Timestamp::now().strftime("%H:%M:%S").to_string()
    });
    engine.register_fn("utc_hour", || {
        jiff::Timestamp::now()
            .strftime("%H")
            .to_string()
            .parse::<i64>()
            .unwrap_or_default()
    });
    let a = Arc::clone(api);
    engine.register_fn("memory_get", move |key: &str| a.memory_get(key));
    let a = Arc::clone(api);
    engine.register_fn("memory_set", move |key: &str, value: &str| {
        a.memory_set(key, value);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeLog;

    impl PluginQueries for FakeLog {
        fn worked_before(
            &self,
            callsign: &str,
            band: Option<&str>,
            _mode: Option<&str>,
        ) -> Option<bool> {
            Some(callsign == "SP9ABC" && band.is_none_or(|b| b == "20m"))
        }

        fn dxcc_of(&self, callsign: &str) -> Option<DxccInfo> {
            callsign.starts_with("SP").then(|| DxccInfo {
                country: "Poland".to_string(),
                dxcc: 269,
                continent: "EU".to_string(),
                cq_zone: 15,
                itu_zone: 28,
            })
        }
    }

    #[test]
    fn queries_use_the_application_answers_within_the_budget() {
        let api = ApiState::default();
        assert!(
            !api.worked_before("SP9ABC", None, None),
            "bez źródła danych: nie"
        );
        api.set_queries(Arc::new(FakeLog));
        api.begin_call("test");
        assert!(api.worked_before(" sp9abc ", None, None));
        assert!(api.worked_before("SP9ABC", Some("20m"), Some("CW")));
        assert!(!api.worked_before("SP9ABC", Some("40m"), Some("CW")));
        assert_eq!(
            api.dxcc_of("SP9ABC")
                .get("dxcc")
                .map(|v| v.as_int().unwrap()),
            Some(269)
        );
        assert!(api.dxcc_of("K1ABC").is_empty());

        for _ in 0..MAX_QUERIES_PER_CALL {
            let _ = api.worked_before("SP9ABC", None, None);
        }
        assert!(
            !api.worked_before("SP9ABC", None, None),
            "limit zapytań w jednym wywołaniu"
        );
        api.begin_call("test");
        assert!(
            api.worked_before("SP9ABC", None, None),
            "nowe wywołanie, nowy limit"
        );
    }

    #[test]
    fn band_plan_mode_follows_the_active_plan() {
        let api = ApiState::default();
        assert_eq!(api.bandplan_mode(14.025), "CW");
        assert_eq!(api.bandplan_mode(0.0), "");
        assert_eq!(api.bandplan_mode(f64::NAN), "");
        let mut plan = BandPlan::iaru_region1().clone();
        plan.band_mut("20m").unwrap().segments[0].mode = crate::core::bandplan::SegmentMode::Fm;
        api.set_bandplan(plan);
        assert_eq!(api.bandplan_mode(14.025), "FM");
    }

    #[test]
    fn memory_is_per_plugin_and_bounded() {
        let api = ApiState::default();
        api.begin_call("a");
        api.memory_set("rate", "12");
        api.begin_call("b");
        assert_eq!(api.memory_get("rate"), "");
        api.begin_call("a");
        assert_eq!(api.memory_get("rate"), "12");
        api.memory_set("rate", "");
        assert_eq!(api.memory_get("rate"), "");
        for i in 0..MAX_MEMORY_KEYS + 5 {
            api.memory_set(&format!("k{i}"), "x");
        }
        api.memory_set("long", &"y".repeat(MAX_MEMORY_VALUE_CHARS * 2));
        let memory = api.memory.lock().unwrap();
        assert_eq!(memory["a"].len(), MAX_MEMORY_KEYS);
        drop(memory);
        api.memory_set("k0", &"z".repeat(MAX_MEMORY_VALUE_CHARS * 2));
        assert_eq!(api.memory_get("k0").chars().count(), MAX_MEMORY_VALUE_CHARS);
    }

    #[test]
    fn deadline_marks_a_call_that_ran_too_long() {
        let api = ApiState::default();
        assert!(!api.out_of_time(), "poza wywołaniem nie ma limitu");
        *api.deadline.lock().unwrap() = Instant::now().checked_sub(Duration::from_millis(1));
        assert!(api.out_of_time());
        api.end_call();
        assert!(!api.out_of_time());
    }
}
