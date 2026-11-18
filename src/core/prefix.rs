// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, RwLock};

/// Informacje o kraju, strefach i prefiksie dla danego znaku wywoławczego
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefixInfo {
    pub callsign: String,
    pub country: String,
    pub arrl_prefix: String,
    pub dxcc: u32,
    pub cqz: u32,
    pub ituz: u32,
    pub continent: String,
    pub latitude: f64,
    pub longitude: f64,
    pub wpx_prefix: String,
    /// Nazwa jednostki administracyjnej (prowincji, stanu, oblastu), jeśli znana.
    pub province: Option<String>,
    /// Kod jednostki administracyjnej z reguły użytkownika, jeśli znany.
    #[serde(default)]
    pub subdivision_code: Option<String>,
    /// Dokładność współrzędnych — GUI i API nie mogą jej zawyżać.
    #[serde(default)]
    pub location_precision: LocationPrecision,
}

/// Źródło i dokładność współrzędnych w [`PrefixInfo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LocationPrecision {
    /// Brak wiarygodnych współrzędnych.
    #[default]
    Unknown,
    /// Środek encji DXCC (pliki krajów).
    EntityCentroid,
    /// Współrzędne z nadpisania `<lat/lon>` w pliku krajów.
    CountryFileOverride,
    /// Współrzędne z reguły regionalnej zaimportowanej przez użytkownika.
    UserRegionalRule,
    /// Współrzędne z jawnego nadpisania użytkownika.
    UserOverride,
    /// Dokładna lokalizacja z danych QSO (lokator / współrzędne). Ustawiana
    /// przez warstwę QSO, nigdy przez dopasowanie prefiksu.
    ExactQsoLocation,
}

/// Warstwa reguł użytkownika, której dotyczy konflikt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleLayer {
    UserOverride,
    UserRegional,
}

/// Konflikt wykryty podczas rozstrzygania znaku: reguła nie została użyta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerConflict {
    /// Kilka reguł wzorca o tym samym najwyższym priorytecie.
    AmbiguousPatterns { layer: RuleLayer, lines: Vec<usize> },
    /// Reguła regionalna wskazuje inną encję niż plik krajów (źródło nadrzędne).
    EntityMismatch {
        layer: RuleLayer,
        line: usize,
        rule_dxcc: u32,
        resolved_dxcc: u32,
    },
}

/// Wynik rozstrzygnięcia znaku wraz z konfliktami reguł użytkownika.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolution {
    pub info: Option<PrefixInfo>,
    pub conflicts: Vec<LayerConflict>,
}

/// Źródło reguł aktualnie załadowanych do [`PrefixMatcher`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefixSource {
    /// Brak danych referencyjnych (stan początkowy).
    #[default]
    None,
    /// Pliki krajów `cty.dat` (+ opcjonalnie `cty.csv` z kodami encji ADIF).
    CountryFiles { entity_codes: bool },
}

/// Podsumowanie wczytania plików krajów.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CountryFileReport {
    pub entities: usize,
    pub prefixes: usize,
    pub exact_calls: usize,
    pub issues: Vec<crate::core::reference::cty::CtyIssue>,
    pub csv_issues: Vec<crate::core::reference::cty::CtyIssue>,
    pub entity_codes: Option<crate::core::reference::cty::EntityCodeReport>,
    /// Prefiksy główne encji dodane jako niejawne aliasy (I6).
    pub implicit_primary_prefixes: usize,
    /// Domyślne atrybuty encji zastąpione wartościami z oficjalnej listy DXCC (D-ENT).
    pub official_attribute_changes: Vec<crate::core::reference::cty::OfficialAttributeChange>,
}

#[derive(Default)]
struct RuleSet {
    cty: Option<crate::core::reference::cty::CtyData>,
    regional: Option<crate::core::reference::rules::RuleBook>,
    overrides: Option<crate::core::reference::rules::RuleBook>,
    source: PrefixSource,
}

/// Silnik dopasowywania znaków do danych DXCC i stref
pub struct PrefixMatcher {
    /// Reguły za blokadą RW — umożliwia przeładowanie danych bez restartu
    /// (współdzielony `Arc<PrefixMatcher>` w GUI i API pozostaje ten sam).
    rules: RwLock<RuleSet>,
    /// Cache wyników `lookup()` — eliminuje wielokrotne skanowanie setek regexów
    /// przy renderowaniu spotów klastra i mapy świata w każdej klatce.
    cache: Mutex<HashMap<String, Option<PrefixInfo>>>,
}

fn cty_match_to_info(
    clean: &str,
    lookup_call: &str,
    m: crate::core::reference::cty::CtyMatch,
) -> PrefixInfo {
    PrefixInfo {
        callsign: clean.to_string(),
        country: m.entity_name,
        arrl_prefix: m.primary_prefix,
        dxcc: m.dxcc.unwrap_or(0),
        cqz: m.cq_zone,
        ituz: m.itu_zone,
        continent: m.continent,
        latitude: m.latitude,
        longitude: m.longitude,
        // Jak w ścieżce starszej bazy: dla znaku dokładnego WPX liczony z pełnego
        // znaku, dla prefiksu — ze znaku po normalizacji.
        wpx_prefix: extract_wpx_prefix(if m.exact { clean } else { lookup_call }),
        province: None,
        subdivision_code: None,
        location_precision: if m.coordinates_overridden {
            LocationPrecision::CountryFileOverride
        } else {
            LocationPrecision::EntityCentroid
        },
    }
}

fn is_operational_suffix(s: &str) -> bool {
    matches!(
        s,
        "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
    )
}

/// Ekstrahuje prefiks WPX (np. SP6INA -> SP6, 3Z100POL -> 3Z100, W1AW -> W1, SP6INA/1 -> SP1, DL/SP6INA -> DL0)
pub fn extract_wpx_prefix(call: &str) -> String {
    let clean = call.trim().to_uppercase();
    let is_operational_suffix = |s: &str| -> bool {
        matches!(
            s,
            "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
        )
    };

    let parts: Vec<&str> = clean
        .split('/')
        .filter(|p| !is_operational_suffix(p) && !p.is_empty())
        .collect();

    if parts.is_empty() {
        return clean;
    }

    let base_call = if parts.len() >= 2 {
        let p0 = parts[0];
        let p1 = parts[1];

        if p0.len() <= 4 && p0.len() < p1.len() && !p0.chars().all(|c| c.is_ascii_digit()) {
            p0
        } else if p1.len() == 1 && p1.chars().all(|c| c.is_ascii_digit()) {
            let prefix0 = extract_wpx_base(p0);
            let mut prefix_chars: Vec<char> = prefix0.chars().collect();
            if let Some(pos) = prefix_chars.iter().rposition(char::is_ascii_digit) {
                if let Some(digit) = p1.chars().next() {
                    prefix_chars[pos] = digit;
                }
                return prefix_chars.into_iter().collect();
            }
            return format!("{prefix0}{p1}");
        } else if p1.len() <= 4 && p1.len() < p0.len() && !p1.chars().all(|c| c.is_ascii_digit()) {
            p1
        } else if p0.len() >= p1.len() {
            p0
        } else {
            p1
        }
    } else {
        parts[0]
    };

    extract_wpx_base(base_call)
}

fn extract_wpx_base(call: &str) -> String {
    let bytes = call.as_bytes();
    let mut seen_alpha = false;
    let mut last_digit_after_alpha = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_alphabetic() {
            seen_alpha = true;
        } else if b.is_ascii_digit() && seen_alpha {
            last_digit_after_alpha = Some(i);
        }
    }

    if let Some(idx) = last_digit_after_alpha {
        call[..=idx].to_string()
    } else {
        let mut res = call.to_string();
        res.push('0');
        res
    }
}

impl PrefixMatcher {
    fn with_rules(rules: RuleSet) -> Self {
        Self {
            rules: RwLock::new(rules),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Tworzy pusty silnik dopasowywania prefiksów (np. gdy brak bazy danych)
    pub fn empty() -> Self {
        Self::with_rules(RuleSet::default())
    }

    /// Ładuje reguły z plików krajów: `cty.dat` (prefiksy, znaki dokładne,
    /// strefy, kontynent, współrzędne) oraz opcjonalnie `cty.csv` (kody encji
    /// ADIF DXCC). Bez `cty.csv` pole `dxcc` ma wartość 0 (nieznane).
    pub fn load_from_country_files(
        dat_path: &Path,
        csv_path: Option<&Path>,
    ) -> std::result::Result<(Self, CountryFileReport), crate::core::reference::cty::CtyError> {
        use crate::core::reference::cty;
        let mut data = cty::CtyData::load(dat_path)?;
        let mut report = CountryFileReport::default();
        if let Some(csv) = csv_path {
            let (rows, csv_issues) = cty::load_cty_csv(csv)?;
            report.entity_codes = Some(data.apply_entity_codes(&rows));
            report.csv_issues = csv_issues;
            // D-ENT: oficjalne atrybuty encji (lista ARRL DXCC) mają pierwszeństwo.
            report.official_attribute_changes = data.apply_official_entity_attributes();
        }
        report.implicit_primary_prefixes = data.implicit_primary_count();
        report.entities = data.entities().len();
        report.prefixes = data.prefix_count();
        report.exact_calls = data.exact_count();
        report.issues.clone_from(&data.issues);
        let entity_codes = report.entity_codes.is_some();
        Ok((
            Self::with_rules(RuleSet {
                cty: Some(data),
                source: PrefixSource::CountryFiles { entity_codes },
                ..RuleSet::default()
            }),
            report,
        ))
    }

    /// Zastępuje reguły regułami z `other` i czyści cache. Pozwala przeładować
    /// dane referencyjne we współdzielonym `Arc<PrefixMatcher>` bez restartu.
    pub fn replace_rules_from(&self, other: Self) {
        let new_rules = other
            .rules
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *self
            .rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = new_rules;
        self.clear_cache();
    }

    /// Aktualne źródło reguł.
    pub fn source(&self) -> PrefixSource {
        self.rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .source
    }

    /// `true`, gdy załadowano jakiekolwiek dane referencyjne.
    pub fn has_data(&self) -> bool {
        self.source() != PrefixSource::None
    }

    /// Znaki dokładne (wyjątki pełnych znaków) z aktualnego źródła, posortowane.
    pub fn exception_calls(&self) -> Vec<PrefixInfo> {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out: Vec<PrefixInfo> = if let Some(cty) = &rules.cty {
            cty.exact_calls()
                .into_iter()
                .map(|m| {
                    let call = m.matched.clone();
                    cty_match_to_info(&call, &call, m)
                })
                .collect()
        } else {
            Vec::new()
        };
        out.sort_by(|a, b| a.callsign.cmp(&b.callsign));
        out
    }

    /// Usuwa wyłącznie końcowe sufiksy operacyjne (/P, /M, /QRP…), które nie
    /// zmieniają lokalizacji stacji. Prefiksy gościnne i zmiany okręgu (/9)
    /// pozostają bez zmian.
    pub fn strip_operational_suffixes(clean: &str) -> &str {
        let mut end = clean.len();
        while let Some(pos) = clean[..end].rfind('/') {
            if is_operational_suffix(&clean[pos + 1..end]) {
                end = pos;
            } else {
                break;
            }
        }
        &clean[..end]
    }

    /// Normalizuje znak wywoławczy dla potrzeb dopasowania prefiksu (obsługa /P, /M, prefiksów gościnnych itp.)
    pub fn normalize_call_for_prefix(clean: &str) -> &str {
        let is_operational_suffix = |s: &str| -> bool {
            matches!(
                s,
                "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
            )
        };

        let mut parts: Vec<&str> = clean.split('/').collect();
        if parts.len() == 1 {
            return clean;
        }

        // Usuń końcowe sufiksy operacyjne (/P, /M itp.)
        while parts.len() > 1 && is_operational_suffix(parts.last().unwrap()) {
            parts.pop();
        }

        if parts.len() == 1 {
            return parts[0];
        }

        let p0 = parts[0];
        let p1 = parts[1];

        // 1. Przypadek stacji gościnnej na początku: DL/SP6INA, OE3/SP6INA, 3D2/SP6INA -> prefiks gościnny (DL, OE3)
        if p0.len() <= 4 && p0.len() < p1.len() && !p0.chars().all(|c| c.is_ascii_digit()) {
            return p0;
        }

        // 2. Przypadek stacji gościnnej na końcu: SP6INA/W6, SP6INA/DL
        if p1.len() <= 4 && p1.len() < p0.len() && !p1.chars().all(|c| c.is_ascii_digit()) {
            return p1;
        }

        // 2b. Przenośne stacje rosyjskie zmieniające okręg/kontynent (np. UA3ABC/9 -> UA9, RA9AAA/3 -> UA3)
        if p1.len() == 1
            && p1.as_bytes()[0].is_ascii_digit()
            && (p0.starts_with('R') || p0.starts_with('U'))
        {
            match p1 {
                "8" | "9" | "0" => return "UA9",
                "2" => return "UA2",
                "1" | "3" | "4" | "5" | "6" | "7" => return "UA3",
                _ => {}
            }
        }

        // 3. Domyślnie bierzemy dłuższą część (znak główny)
        if p0.len() >= p1.len() { p0 } else { p1 }
    }

    /// Wyszukuje kraj DXCC, strefy CQ/ITU i dane dla podanego znaku.
    ///
    /// Wyniki są zapamiętywane w cache (klucz = znak po normalizacji wielkości
    /// liter), dzięki czemu powtarzane wywołania dla tych samych znaków (np. przy
    /// renderowaniu spotów klastra co klatkę) nie skanują ponownie setek regexów.
    pub fn lookup(&self, callsign: &str) -> Option<PrefixInfo> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        if let Some(cached) = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&clean)
        {
            return cached.clone();
        }

        let result = self.lookup_uncached(&clean);

        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(clean, result.clone());

        result
    }

    /// Czyści cache wyników wyszukiwania (wywołaj po przeładowaniu reguł prefiksów).
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn lookup_uncached(&self, clean: &str) -> Option<PrefixInfo> {
        self.resolve_clean(clean).info
    }

    /// Rozstrzyga znak bez użycia cache i zwraca też konflikty reguł
    /// użytkownika (reguły, które z powodu konfliktu nie zostały zastosowane).
    pub fn lookup_detailed(&self, callsign: &str) -> Resolution {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return Resolution::default();
        }
        self.resolve_clean(&clean)
    }

    /// Kolejność warstw:
    /// 1. jawne nadpisanie użytkownika;
    /// 2. znak dokładny z plików krajów;
    /// 3. dokładna reguła regionalna użytkownika;
    /// 4. najdłuższy prefiks z plików krajów, doprecyzowany regułą regionalną
    ///    użytkownika;
    /// 5. sama reguła użytkownika z kodem encji;
    /// 6. brak wyniku.
    fn resolve_clean(&self, clean: &str) -> Resolution {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let lookup_call = Self::normalize_call_for_prefix(clean);
        let base = Self::strip_operational_suffixes(clean);
        let mut conflicts = Vec::new();

        let mut info =
            Self::resolve_without_overrides(&rules, clean, lookup_call, base, &mut conflicts);

        if let Some(book) = &rules.overrides {
            let rule = book.match_exact(clean, base).or_else(|| {
                pick_rule(
                    book.match_prefix(lookup_call),
                    RuleLayer::UserOverride,
                    &mut conflicts,
                )
            });
            if let Some(rule) = rule {
                info = apply_rule(
                    info,
                    rule,
                    rules.cty.as_ref(),
                    clean,
                    lookup_call,
                    LocationPrecision::UserOverride,
                );
            }
        }

        Resolution { info, conflicts }
    }

    fn resolve_without_overrides(
        rules: &RuleSet,
        clean: &str,
        lookup_call: &str,
        base: &str,
        conflicts: &mut Vec<LayerConflict>,
    ) -> Option<PrefixInfo> {
        let regional = rules.regional.as_ref();

        if let Some(cty) = &rules.cty {
            // 2. Znak dokładny z plików krajów.
            let exact = cty
                .resolve_exact(clean)
                .or_else(|| (base != clean).then(|| cty.resolve_exact(base)).flatten());
            if let Some(m) = exact {
                return Some(cty_match_to_info(clean, lookup_call, m));
            }
            let prefix_info = cty
                .resolve_prefix(lookup_call)
                .map(|m| cty_match_to_info(clean, lookup_call, m));

            // 3. Dokładna reguła regionalna użytkownika (może zmienić encję).
            if let Some(rule) = regional.and_then(|b| b.match_exact(clean, base)) {
                return apply_rule(
                    prefix_info,
                    rule,
                    Some(cty),
                    clean,
                    lookup_call,
                    LocationPrecision::UserRegionalRule,
                );
            }

            // 4. Najdłuższy prefiks + doprecyzowanie regionalne.
            if let Some(info) = prefix_info {
                if let Some(book) = regional
                    && let Some(rule) = pick_rule(
                        book.match_prefix(lookup_call),
                        RuleLayer::UserRegional,
                        conflicts,
                    )
                {
                    match rule.attributes.dxcc {
                        Some(d) if d != info.dxcc => {
                            conflicts.push(LayerConflict::EntityMismatch {
                                layer: RuleLayer::UserRegional,
                                line: rule.line,
                                rule_dxcc: d,
                                resolved_dxcc: info.dxcc,
                            });
                        }
                        _ => {
                            return apply_rule(
                                Some(info),
                                rule,
                                Some(cty),
                                clean,
                                lookup_call,
                                LocationPrecision::UserRegionalRule,
                            );
                        }
                    }
                }
                return Some(info);
            }

            // 5. Sama reguła regionalna z kodem encji.
            if let Some(book) = regional
                && let Some(rule) = pick_rule(
                    book.match_prefix(lookup_call),
                    RuleLayer::UserRegional,
                    conflicts,
                )
            {
                return apply_rule(
                    None,
                    rule,
                    Some(cty),
                    clean,
                    lookup_call,
                    LocationPrecision::UserRegionalRule,
                );
            }
            return None;
        }

        // Bez plików krajów: same reguły regionalne użytkownika.
        let book = regional?;
        if let Some(rule) = book.match_exact(clean, base) {
            return apply_rule(
                None,
                rule,
                None,
                clean,
                lookup_call,
                LocationPrecision::UserRegionalRule,
            );
        }
        let rule = pick_rule(
            book.match_prefix(lookup_call),
            RuleLayer::UserRegional,
            conflicts,
        )?;
        apply_rule(
            None,
            rule,
            None,
            clean,
            lookup_call,
            LocationPrecision::UserRegionalRule,
        )
    }

    /// Dołącza reguły regionalne użytkownika (zastępuje poprzednie).
    #[must_use]
    pub fn with_regional_rules(self, book: crate::core::reference::rules::RuleBook) -> Self {
        self.rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .regional = Some(book);
        self.clear_cache();
        self
    }

    /// Dołącza jawne nadpisania użytkownika (zastępuje poprzednie).
    #[must_use]
    pub fn with_overrides(self, book: crate::core::reference::rules::RuleBook) -> Self {
        self.rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .overrides = Some(book);
        self.clear_cache();
        self
    }

    /// Opis załadowanych warstw.
    pub fn layers(&self) -> LayerSummary {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        LayerSummary {
            source: rules.source,
            country_files: rules.cty.is_some(),
            regional_rules: rules
                .regional
                .as_ref()
                .map_or(0, crate::core::reference::rules::RuleBook::len),
            override_rules: rules
                .overrides
                .as_ref()
                .map_or(0, crate::core::reference::rules::RuleBook::len),
        }
    }
}

/// Liczba reguł w poszczególnych warstwach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayerSummary {
    pub source: PrefixSource,
    pub country_files: bool,
    pub regional_rules: usize,
    pub override_rules: usize,
}

fn pick_rule<'a>(
    m: crate::core::reference::rules::RuleMatch<'a>,
    layer: RuleLayer,
    conflicts: &mut Vec<LayerConflict>,
) -> Option<&'a crate::core::reference::rules::Rule> {
    use crate::core::reference::rules::RuleMatch;
    match m {
        RuleMatch::None => None,
        RuleMatch::Matched(r) => Some(r),
        RuleMatch::Conflict(rs) => {
            conflicts.push(LayerConflict::AmbiguousPatterns {
                layer,
                lines: rs.iter().map(|r| r.line).collect(),
            });
            None
        }
    }
}

/// Nakłada atrybuty reguły na wynik niższych warstw. Kod encji z reguły
/// zmienia encję (nazwa, prefiks i wartości domyślne z pliku krajów, jeśli
/// dostępny); dokładność współrzędnych rośnie tylko, gdy reguła je podaje.
fn apply_rule(
    base: Option<PrefixInfo>,
    rule: &crate::core::reference::rules::Rule,
    cty: Option<&crate::core::reference::cty::CtyData>,
    clean: &str,
    lookup_call: &str,
    precision: LocationPrecision,
) -> Option<PrefixInfo> {
    let a = &rule.attributes;
    let mut info = match (base, a.dxcc) {
        (Some(b), Some(d)) if b.dxcc != d => entity_info(cty, d, clean, lookup_call),
        (Some(b), _) => b,
        (None, Some(d)) => entity_info(cty, d, clean, lookup_call),
        (None, None) => return None,
    };
    if let Some(z) = a.cq_zone {
        info.cqz = z;
    }
    if let Some(z) = a.itu_zone {
        info.ituz = z;
    }
    if let Some(c) = &a.continent {
        info.continent.clone_from(c);
    }
    if let Some((lat, lon)) = a.coordinates {
        info.latitude = lat;
        info.longitude = lon;
        info.location_precision = precision;
    }
    if a.subdivision_name.is_some() {
        info.province.clone_from(&a.subdivision_name);
    }
    if a.subdivision_code.is_some() {
        info.subdivision_code.clone_from(&a.subdivision_code);
    }
    Some(info)
}

/// Dane encji o podanym kodzie z plików krajów; bez nich — tylko kod, bez
/// współrzędnych (dokładność `Unknown`).
fn entity_info(
    cty: Option<&crate::core::reference::cty::CtyData>,
    dxcc: u32,
    clean: &str,
    lookup_call: &str,
) -> PrefixInfo {
    let entity = cty.and_then(|c| c.entity_for_dxcc(dxcc));
    match entity {
        Some(e) => PrefixInfo {
            callsign: clean.to_string(),
            country: e.name.clone(),
            arrl_prefix: e.primary_prefix.clone(),
            dxcc,
            cqz: e.cq_zone,
            ituz: e.itu_zone,
            continent: e.continent.clone(),
            latitude: e.latitude,
            longitude: e.longitude,
            wpx_prefix: extract_wpx_prefix(lookup_call),
            province: None,
            subdivision_code: None,
            location_precision: LocationPrecision::EntityCentroid,
        },
        None => PrefixInfo {
            callsign: clean.to_string(),
            country: String::new(),
            arrl_prefix: String::new(),
            dxcc,
            cqz: 0,
            ituz: 0,
            continent: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            wpx_prefix: extract_wpx_prefix(lookup_call),
            province: None,
            subdivision_code: None,
            location_precision: LocationPrecision::Unknown,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wpx() {
        assert_eq!(extract_wpx_prefix("SP6INA"), "SP6");
        assert_eq!(extract_wpx_prefix("3Z100POL"), "3Z100");
        assert_eq!(extract_wpx_prefix("W1AW"), "W1");
        assert_eq!(extract_wpx_prefix("DL1ABC/P"), "DL1");
        assert_eq!(extract_wpx_prefix("SP6INA/1"), "SP1");
        assert_eq!(extract_wpx_prefix("SP6INA/W6"), "W6");
        assert_eq!(extract_wpx_prefix("DL/SP6INA/P"), "DL0");
    }

    #[test]
    fn strip_operational_suffixes_keeps_location_parts() {
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/P"),
            "QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/QRP/M"),
            "QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/9"),
            "QA1AB/9"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QB/QA1AB"),
            "QB/QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QB/QA1AB/P"),
            "QB/QA1AB"
        );
        assert_eq!(PrefixMatcher::strip_operational_suffixes("QA1AB"), "QA1AB");
    }

    #[test]
    fn test_normalize_call_for_prefix() {
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/P"),
            "SP6INA"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/M"),
            "SP6INA"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/QRP"),
            "SP6INA"
        );
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("DL/SP6INA"), "DL");
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("OE3/SP6INA"),
            "OE3"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("3D2/SP6INA/P"),
            "3D2"
        );
    }

    // Syntetyczne pliki krajów (blok prefiksów Q nie jest przydzielony żadnemu krajowi).
    const SYNTH_DAT: &str = "\
Alpha Land:               14:  27:  EU:   50.00:   -20.00:    -1.0:  QA:
    QA,QA2(15)[28],=QA1EXACT{AS};
Beta Isles:               3:   6:   NA:   20.00:   100.00:     8.0:  QB:
    QB;
";
    const SYNTH_CSV: &str = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA QA2(15)[28] =QA1EXACT{AS};
QB,Beta Isles,902,NA,3,6,20.00,100.00,8.0,QB;
";

    fn write_country_files(dir: &Path, dat: &str, csv: Option<&str>) {
        std::fs::write(dir.join("cty.dat"), dat).unwrap();
        if let Some(csv) = csv {
            std::fs::write(dir.join("cty.csv"), csv).unwrap();
        }
    }

    #[test]
    fn country_files_resolve_through_call_normalisation() {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        let (m, report) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        assert_eq!(report.entities, 2);
        assert_eq!(report.exact_calls, 1);
        assert_eq!(report.entity_codes.as_ref().unwrap().matched, 2);
        assert_eq!(
            m.source(),
            PrefixSource::CountryFiles { entity_codes: true }
        );

        let info = m.lookup("qa5xyz/p").unwrap();
        assert_eq!(info.callsign, "QA5XYZ/P");
        assert_eq!(info.country, "Alpha Land");
        assert_eq!(info.arrl_prefix, "QA");
        assert_eq!(info.dxcc, 901);
        assert_eq!((info.cqz, info.ituz), (14, 27));
        assert_eq!(info.continent, "EU");
        assert!((info.latitude - 50.0).abs() < 1e-9);
        assert!((info.longitude - 20.0).abs() < 1e-9);
        assert_eq!(info.wpx_prefix, "QA5");
        assert_eq!(info.province, None);

        // Prefiks gościnny: QB/QA5XYZ -> QB.
        assert_eq!(m.lookup("QB/QA5XYZ").unwrap().country, "Beta Isles");
        // Nadpisanie stref dla dłuższego prefiksu.
        let z = m.lookup("QA2AA").unwrap();
        assert_eq!((z.cqz, z.ituz), (15, 28));
        // Znak dokładny z nadpisaniem kontynentu — także z sufiksem operacyjnym.
        assert_eq!(m.lookup("QA1EXACT").unwrap().continent, "AS");
        assert_eq!(m.lookup("QA1EXACT/P").unwrap().continent, "AS");
        assert_eq!(m.lookup("QA1EXACT/QRP/M").unwrap().continent, "AS");
        // Zmiana okręgu i prefiks gościnny nie dziedziczą znaku dokładnego.
        assert_eq!(m.lookup("QA1EXACT/2").unwrap().continent, "EU");
        assert_eq!(m.lookup("QB/QA1EXACT").unwrap().country, "Beta Isles");
        assert_eq!(m.lookup("X1ABC"), None);

        let ex = m.exception_calls();
        assert_eq!(ex.len(), 1);
        assert_eq!(ex[0].callsign, "QA1EXACT");
        assert_eq!(ex[0].dxcc, 901);
    }

    #[test]
    fn country_files_without_csv_report_unknown_entity_code() {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, None);
        let (m, report) =
            PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).unwrap();
        assert!(report.entity_codes.is_none());
        assert_eq!(
            m.source(),
            PrefixSource::CountryFiles {
                entity_codes: false
            }
        );
        assert_eq!(m.lookup("QA5XYZ").unwrap().dxcc, 0);
    }

    #[test]
    fn missing_country_files_are_errors_and_matcher_stays_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).is_err());
        write_country_files(dir.path(), SYNTH_DAT, None);
        assert!(
            PrefixMatcher::load_from_country_files(
                &dir.path().join("cty.dat"),
                Some(&dir.path().join("cty.csv"))
            )
            .is_err()
        );
        let m = PrefixMatcher::empty();
        assert!(!m.has_data());
        assert_eq!(m.source(), PrefixSource::None);
        assert_eq!(m.exception_calls(), Vec::<PrefixInfo>::new());
    }

    #[test]
    fn replace_rules_reloads_shared_matcher_and_clears_cache() {
        let dir = tempfile::tempdir().unwrap();
        let shared = std::sync::Arc::new(PrefixMatcher::empty());
        assert_eq!(shared.lookup("QA5XYZ"), None); // trafia do cache jako None

        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        let (fresh, _) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        shared.replace_rules_from(fresh);
        assert!(shared.has_data());
        assert_eq!(shared.lookup("QA5XYZ").unwrap().dxcc, 901);

        // Kolejna wersja danych zastępuje poprzednią.
        write_country_files(dir.path(), &SYNTH_DAT.replace("QA,QA2", "QA,QA5,QA2"), None);
        let (v2, _) =
            PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).unwrap();
        shared.replace_rules_from(v2);
        assert_eq!(shared.lookup("QA5XYZ").unwrap().dxcc, 0);
        assert_eq!(shared.lookup("QA5XYZ").unwrap().arrl_prefix, "QA");
    }

    fn rule_book(body: &str) -> crate::core::reference::rules::RuleBook {
        let text = format!(
            "#splogbook-rules,1\nrule_type,callsign_or_prefix,dxcc_entity_code,subdivision_code,subdivision_name,cq_zone,itu_zone,continent,latitude,longitude,priority\n{body}"
        );
        let parsed = crate::core::reference::rules::parse_rules(&text);
        assert!(parsed.is_valid(), "{:?}", parsed.issues);
        crate::core::reference::rules::RuleBook::from_rules(parsed.rules)
    }

    fn cty_matcher() -> PrefixMatcher {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap()
        .0
    }

    #[test]
    fn country_file_results_report_entity_or_override_precision() {
        let m = cty_matcher();
        assert_eq!(
            m.lookup("QA5XYZ").unwrap().location_precision,
            LocationPrecision::EntityCentroid
        );
        let empty = PrefixMatcher::empty();
        assert_eq!(empty.lookup("QA5XYZ"), None);
    }

    #[test]
    fn regional_prefix_rule_refines_zones_coordinates_and_region() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "prefix,QA7,901,QA-N,Alpha North,16,29,,55.5,22.5,0\n\
             prefix,QA8,,,Alpha Zone Only,17,,,,,0\n",
        ));
        let r = m.lookup("QA7AB/P").unwrap();
        assert_eq!(r.dxcc, 901);
        assert_eq!(r.country, "Alpha Land");
        assert_eq!((r.cqz, r.ituz), (16, 29));
        assert_eq!(r.province.as_deref(), Some("Alpha North"));
        assert_eq!(r.subdivision_code.as_deref(), Some("QA-N"));
        assert_eq!((r.latitude, r.longitude), (55.5, 22.5));
        assert_eq!(r.location_precision, LocationPrecision::UserRegionalRule);

        // Reguła bez współrzędnych nie podnosi deklarowanej dokładności.
        let z = m.lookup("QA8AB").unwrap();
        assert_eq!(z.cqz, 17);
        assert_eq!(z.province.as_deref(), Some("Alpha Zone Only"));
        assert_eq!(z.location_precision, LocationPrecision::EntityCentroid);
        assert_eq!((z.latitude, z.longitude), (50.0, 20.0));
    }

    #[test]
    fn regional_rule_cannot_change_entity_resolved_by_country_files() {
        let m = cty_matcher().with_regional_rules(rule_book("prefix,QA7,902,,,16,,,,,0\n"));
        let res = m.lookup_detailed("QA7AB");
        let info = res.info.unwrap();
        assert_eq!(info.dxcc, 901);
        assert_eq!(info.cqz, 14);
        assert_eq!(
            res.conflicts,
            vec![LayerConflict::EntityMismatch {
                layer: RuleLayer::UserRegional,
                line: 3,
                rule_dxcc: 902,
                resolved_dxcc: 901,
            }]
        );
    }

    #[test]
    fn country_file_exact_call_ranks_above_regional_exact_rule() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "exact,QA1EXACT,,,,20,,,,,0\nexact,QA5XYZ,902,,,21,,,,,0\n",
        ));
        // Znak dokładny z pliku krajów wygrywa z dokładną regułą regionalną.
        assert_eq!(m.lookup("QA1EXACT").unwrap().cqz, 14);
        // Dokładna reguła regionalna wygrywa z prefiksem i może zmienić encję.
        let r = m.lookup("QA5XYZ").unwrap();
        assert_eq!(r.dxcc, 902);
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.cqz, 21);
        assert_eq!(r.location_precision, LocationPrecision::EntityCentroid);
    }

    #[test]
    fn user_override_ranks_above_every_other_layer() {
        let m = cty_matcher()
            .with_regional_rules(rule_book("prefix,QA,,,,16,,,,,0\n"))
            .with_overrides(rule_book(
                "exact,QA1EXACT,902,,,7,,,1.5,2.5,0\nprefix,QA9,,,Override Region,,,,,,0\n",
            ));
        let r = m.lookup("QA1EXACT/P").unwrap();
        assert_eq!(r.dxcc, 902);
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.cqz, 7);
        assert_eq!((r.latitude, r.longitude), (1.5, 2.5));
        assert_eq!(r.location_precision, LocationPrecision::UserOverride);
        let p = m.lookup("QA9ZZ").unwrap();
        assert_eq!(p.province.as_deref(), Some("Override Region"));
        assert_eq!(p.cqz, 16); // strefa z reguły regionalnej pozostaje
        assert_eq!(m.layers().override_rules, 2);
        assert_eq!(m.layers().regional_rules, 1);
    }

    #[test]
    fn override_can_resolve_a_call_no_source_knows() {
        let m = cty_matcher().with_overrides(rule_book("prefix,QZ,902,,,,,,,,0\n"));
        let r = m.lookup("QZ1AB").unwrap();
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.location_precision, LocationPrecision::EntityCentroid);
        // Bez plików krajów encja jest znana tylko z kodu — bez współrzędnych.
        let bare = PrefixMatcher::empty().with_overrides(rule_book("prefix,QZ,902,,,,,,,,0\n"));
        let b = bare.lookup("QZ1AB").unwrap();
        assert_eq!(b.dxcc, 902);
        assert_eq!(b.location_precision, LocationPrecision::Unknown);
    }

    #[test]
    fn ambiguous_patterns_are_reported_and_not_applied() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "pattern,QA[0-9].*,,,,30,,,,,1\npattern,QA5.*,,,,31,,,,,1\n",
        ));
        let res = m.lookup_detailed("QA5AB");
        assert_eq!(res.info.unwrap().cqz, 14);
        assert_eq!(
            res.conflicts,
            vec![LayerConflict::AmbiguousPatterns {
                layer: RuleLayer::UserRegional,
                lines: vec![3, 4],
            }]
        );
    }

    #[test]
    fn country_file_load_applies_implicit_primaries_and_official_attributes() {
        let dir = tempfile::tempdir().unwrap();
        let dat = "\
Ant Synth:                13:  74:  SA:  -90.00:     0.00:     0.0:  QY:
    QY0,QY1;
";
        let csv = "QY,Ant Synth,13,SA,13,74,-90.00,0.00,0.0,QY0 QY1;\n";
        write_country_files(dir.path(), dat, Some(csv));
        let (m, report) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        assert_eq!(report.implicit_primary_prefixes, 1);
        assert_eq!(report.official_attribute_changes.len(), 1);
        // Prefiks gościnny równy prefiksowi głównemu (`QY/SP1AA` → `QY`).
        let g = m.lookup("QY/QA1AA").unwrap();
        assert_eq!((g.dxcc, g.continent.as_str()), (13, "AN"));
        assert_eq!(m.lookup("QY5AB").unwrap().arrl_prefix, "QY");
    }

    #[test]
    fn test_lookup_caches_and_clear_cache() {
        let matcher = PrefixMatcher::empty();

        // Brak reguł -> None, ale wynik trafia do cache.
        assert_eq!(matcher.lookup("SP6INA"), None);
        assert_eq!(matcher.lookup("sp6ina"), None); // ścieżka cache (normalizacja wielkości liter)
        assert_eq!(matcher.lookup(""), None);

        // clear_cache() musi być bezpieczne i nie zmieniać poprawności wyników.
        matcher.clear_cache();
        assert_eq!(matcher.lookup("SP6INA"), None);
    }
}
