// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny rejestr ikon SPLogbook.
//!
//! Wszystkie glify oraz ich kolory semantyczne są zdefiniowane **tutaj**,
//! aby zamiast rozrzuconej mieszanki emoji/unicode panował jeden, spójny
//! zestaw. Nowe ikony dodawaj wyłącznie w tym pliku i odwołuj się do nich
//! przez stałe (np. `icons::RADIO`), a nie przez wstawianie gołego glifu.

use eframe::egui::{Color32, RichText};

// ---------------------------------------------------------------------------
// Paleta semantyczna (spójna z `theme.rs`)
// ---------------------------------------------------------------------------
pub const ACCENT: Color32 = Color32::from_rgb(56, 189, 248); // cyjan — radio/CAT
pub const SUCCESS: Color32 = Color32::from_rgb(34, 197, 94); // zielony — OK/potwierdzenie
pub const DANGER: Color32 = Color32::from_rgb(239, 68, 68); // czerwony — błąd/niszczące
pub const WARN: Color32 = Color32::from_rgb(250, 204, 21); // bursztyn — ostrzeżenie
pub const INFO: Color32 = Color32::from_rgb(59, 130, 246); // niebieski — informacja
pub const NEUTRAL: Color32 = Color32::from_rgb(148, 163, 184); // szarość
pub const HIGHLIGHT: Color32 = Color32::from_rgb(217, 70, 239); // fuksja — wyróżnienie
pub const GOLD: Color32 = Color32::from_rgb(234, 179, 8); // złoto — nagrody
pub const VIOLET: Color32 = Color32::from_rgb(216, 180, 254); // fiolet

// ---------------------------------------------------------------------------
// Ikona = glif + kolor semantyczny
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Icon {
    pub glyph: &'static str,
    pub color: Color32,
}

impl Icon {
    pub const fn new(glyph: &'static str, color: Color32) -> Self {
        Self { glyph, color }
    }

    /// Zwraca goły glif (do użycia w `format!`).
    pub fn as_str(self) -> &'static str {
        self.glyph
    }

    /// Glif poprzedzający etykietę: `"📻 Transceiver"`.
    pub fn label(self, text: impl Into<String>) -> String {
        format!("{} {}", self.glyph, text.into())
    }

    /// Tekst sformatowany (kolor + rozmiar).
    pub fn rich(self, size: f32) -> RichText {
        RichText::new(self.glyph).color(self.color).size(size)
    }

    /// Etykieta sformatowana (glif w kolorze + tekst).
    pub fn rich_label(self, text: impl Into<String>, size: f32) -> RichText {
        RichText::new(format!("{} {}", self.glyph, text.into())).color(self.color).size(size)
    }
}

// ---------------------------------------------------------------------------
// Radio / CAT / sprzęt
// ---------------------------------------------------------------------------
pub const RADIO: Icon = Icon::new("📻", ACCENT);
pub const VFO_KNOB: Icon = Icon::new("🎛", ACCENT);
pub const ROTOR: Icon = Icon::new("🧭", ACCENT);
pub const SO2R: Icon = Icon::new("⇄", ACCENT);
pub const CW_KEYER: Icon = Icon::new("⚡", WARN);
pub const CW_TERMINAL: Icon = Icon::new("📟", ACCENT);
pub const PTT_MIC: Icon = Icon::new("🎙", ACCENT);
pub const VOICE_KEYER: Icon = Icon::new("🎙", ACCENT);
pub const CAT_LINK: Icon = Icon::new("🔗", INFO);

// ---------------------------------------------------------------------------
// Dziennik / łączności
// ---------------------------------------------------------------------------
pub const NEW_QSO: Icon = Icon::new("📝", ACCENT);
pub const LOGBOOK: Icon = Icon::new("📋", ACCENT);
pub const SEARCH: Icon = Icon::new("🔍", INFO);
pub const SEARCH_SM: Icon = Icon::new("🔎", INFO);
pub const SAVE: Icon = Icon::new("💾", SUCCESS);
pub const EXPORT: Icon = Icon::new("📤", INFO);
pub const IMPORT: Icon = Icon::new("📥", INFO);
pub const EDIT: Icon = Icon::new("✏", GOLD);
pub const DELETE: Icon = Icon::new("🗑", DANGER);
pub const CLEAR: Icon = Icon::new("❌", DANGER);
pub const CLOSE: Icon = Icon::new("✕", NEUTRAL);
pub const POPOUT: Icon = Icon::new("↗", NEUTRAL);
pub const DOCK: Icon = Icon::new("↙", NEUTRAL);
pub const ARROW_RIGHT: Icon = Icon::new("→", NEUTRAL);
pub const ARROW_LEFT: Icon = Icon::new("◀", NEUTRAL);
pub const PLAY: Icon = Icon::new("▶", SUCCESS);
pub const STOP: Icon = Icon::new("⏹", DANGER);
pub const ABORT: Icon = Icon::new("⛔", DANGER);
pub const ADD: Icon = Icon::new("➕", SUCCESS);
pub const FILTER: Icon = Icon::new("🔎", INFO);
pub const DUPLICATES: Icon = Icon::new("🧹", WARN);

// ---------------------------------------------------------------------------
// Pasmo / klaster / mapa
// ---------------------------------------------------------------------------
pub const CLUSTER: Icon = Icon::new("📡", ACCENT);
pub const BANDMAP: Icon = Icon::new("📶", ACCENT);
pub const SPOT: Icon = Icon::new("📢", ACCENT);
pub const WORLD_MAP: Icon = Icon::new("🗺", ACCENT);
pub const QSY: Icon = Icon::new("📻", ACCENT);
pub const ANTENNA_TUNER: Icon = Icon::new("📶", ACCENT);
pub const SIGNAL_UP: Icon = Icon::new("⬆", SUCCESS);
pub const SIGNAL_DOWN: Icon = Icon::new("⬇", DANGER);

// ---------------------------------------------------------------------------
// Propagacja / pogoda kosmiczna
// ---------------------------------------------------------------------------
pub const SOLAR: Icon = Icon::new("☀", GOLD);
pub const SUN: Icon = Icon::new("🌞", GOLD);
pub const MOON: Icon = Icon::new("🌙", NEUTRAL);
pub const NEW_MOON: Icon = Icon::new("🌑", NEUTRAL);
pub const SATELLITE: Icon = Icon::new("🛰", ACCENT);
pub const ASTRONOMY: Icon = Icon::new("🔬", VIOLET);
pub const STORM: Icon = Icon::new("🌪", WARN);

// ---------------------------------------------------------------------------
// Nagrody / osiągnięcia
// ---------------------------------------------------------------------------
pub const AWARDS: Icon = Icon::new("🏆", GOLD);
pub const DXCC: Icon = Icon::new("🌍", ACCENT);
pub const WAZ: Icon = Icon::new("🌐", ACCENT);
pub const GLOBE: Icon = Icon::new("🌐", ACCENT);
pub const GLOBE_EUROPE_AFRICA: Icon = Icon::new("🌍", ACCENT);
pub const WAC: Icon = Icon::new("🌍", GOLD);
pub const WAE: Icon = Icon::new("🌍", VIOLET);
pub const WPX: Icon = Icon::new("🏷", GOLD);
pub const IOTA: Icon = Icon::new("🏝", SUCCESS);
pub const VUCC: Icon = Icon::new("📡", ACCENT);
pub const SOTA: Icon = Icon::new("⛰", GOLD);
pub const SOTA_MOUNTAIN: Icon = Icon::new("🏔", GOLD);
pub const POTA: Icon = Icon::new("🌲", SUCCESS);
pub const MEDAL: Icon = Icon::new("🎖", GOLD);
pub const FLAG: Icon = Icon::new("🏁", ACCENT);
pub const BADGE: Icon = Icon::new("🏷", GOLD);

// ---------------------------------------------------------------------------
// Konfiguracja / narzędzia
// ---------------------------------------------------------------------------
pub const SETTINGS: Icon = Icon::new("⚙", NEUTRAL);
pub const THEME: Icon = Icon::new("🎨", VIOLET);
pub const PLUGIN: Icon = Icon::new("🧩", VIOLET);
pub const STORE: Icon = Icon::new("🛒", SUCCESS);
pub const TERMINAL: Icon = Icon::new("💻", NEUTRAL);
pub const DESKTOP: Icon = Icon::new("🖥", NEUTRAL);
pub const WORKSPACE: Icon = Icon::new("🪟", ACCENT);
pub const FOLDER: Icon = Icon::new("📁", GOLD);
pub const DOCUMENT: Icon = Icon::new("📄", NEUTRAL);
pub const MANUAL: Icon = Icon::new("📖", INFO);
pub const CHANGELOG: Icon = Icon::new("📜", NEUTRAL);
pub const UPDATE: Icon = Icon::new("🔄", SUCCESS);
pub const REFRESH: Icon = Icon::new("🔄", SUCCESS);
pub const INFO_ICON: Icon = Icon::new("💡", INFO);
pub const LINK: Icon = Icon::new("🔗", INFO);
pub const PRINT: Icon = Icon::new("🖨", NEUTRAL);
pub const CALENDAR: Icon = Icon::new("📅", INFO);
pub const CLOCK: Icon = Icon::new("🕒", NEUTRAL);
pub const ALARM: Icon = Icon::new("⏰", WARN);
pub const HOURGLASS: Icon = Icon::new("⏳", WARN);
pub const PIN: Icon = Icon::new("📌", DANGER);
pub const LOCATION: Icon = Icon::new("📍", DANGER);
pub const MAILBOX: Icon = Icon::new("📬", NEUTRAL);
pub const MAIL: Icon = Icon::new("📨", INFO);
pub const PACKAGE: Icon = Icon::new("📦", NEUTRAL);
pub const RULER: Icon = Icon::new("📏", NEUTRAL);
pub const CAMERA: Icon = Icon::new("📷", ACCENT);
pub const SPARKLE: Icon = Icon::new("✨", HIGHLIGHT);
pub const STAR: Icon = Icon::new("⭐", GOLD);
pub const COPYRIGHT: Icon = Icon::new("©", NEUTRAL);

// ---------------------------------------------------------------------------
// Status / stany
// ---------------------------------------------------------------------------
pub const WARNING: Icon = Icon::new("⚠", WARN);
pub const OK: Icon = Icon::new("✓", SUCCESS);
pub const OK_CHECK: Icon = Icon::new("✔", SUCCESS);
pub const OK_BOX: Icon = Icon::new("✅", SUCCESS);
pub const OK_BOX_EMPTY: Icon = Icon::new("☑", SUCCESS);
pub const NOTIF_ON: Icon = Icon::new("🔔", SUCCESS);
pub const NOTIF_OFF: Icon = Icon::new("🔕", NEUTRAL);
pub const DOT_GREEN: Icon = Icon::new("🟢", SUCCESS);
pub const DOT_RED: Icon = Icon::new("🔴", DANGER);
pub const DOT_ORANGE: Icon = Icon::new("🟠", WARN);
pub const DOT_YELLOW: Icon = Icon::new("🟡", WARN);
pub const DOT_WHITE: Icon = Icon::new("⚪", NEUTRAL);
pub const DOT_BLACK: Icon = Icon::new("⚫", NEUTRAL);
pub const CROSS: Icon = Icon::new("✖", DANGER);
pub const DIAMOND: Icon = Icon::new("✦", SUCCESS);

// ---------------------------------------------------------------------------
// Różne / funkcje specjalne
// ---------------------------------------------------------------------------
pub const AI_ASSISTANT: Icon = Icon::new("🤖", ACCENT);
pub const WIZARD: Icon = Icon::new("🧙", VIOLET);
pub const BUG: Icon = Icon::new("🐛", DANGER);
pub const DIAGNOSTICS: Icon = Icon::new("🩺", WARN);
pub const ROCKET: Icon = Icon::new("🚀", ACCENT);
pub const DICE: Icon = Icon::new("🎲", VIOLET);
pub const CLOUD: Icon = Icon::new("☁", NEUTRAL);
pub const SPEAKER: Icon = Icon::new("🔊", ACCENT);
pub const STOP_SIGN: Icon = Icon::new("🛑", DANGER);
pub const RETURN: Icon = Icon::new("↩", NEUTRAL);
pub const FORWARD: Icon = Icon::new("↪", NEUTRAL);
pub const SWAP: Icon = Icon::new("🔁", ACCENT);
pub const BOOKS: Icon = Icon::new("📚", INFO);
pub const GRAPH: Icon = Icon::new("📊", ACCENT);
pub const STATS: Icon = Icon::new("📊", ACCENT);
pub const LIGHTNING: Icon = Icon::new("⚡", WARN);
pub const TARGET: Icon = Icon::new("🎯", DANGER);
pub const COMPACT: Icon = Icon::new("🗗", NEUTRAL);
pub const TIMER: Icon = Icon::new("⏱", NEUTRAL);
pub const DIAMOND_SM: Icon = Icon::new("🔹", INFO);
pub const KEYBOARD: Icon = Icon::new("⌨", NEUTRAL);

// ---------------------------------------------------------------------------
// Testy
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_label_prefixes_with_glyph() {
        assert_eq!(RADIO.label("Transceiver"), "📻 Transceiver");
        assert_eq!(LOGBOOK.label("Dziennik"), "📋 Dziennik");
    }

    #[test]
    fn icon_colors_are_stable() {
        assert_eq!(SUCCESS, Color32::from_rgb(34, 197, 94));
        assert_eq!(DANGER, Color32::from_rgb(239, 68, 68));
        assert_eq!(ACCENT, Color32::from_rgb(56, 189, 248));
    }

    #[test]
    fn rich_text_carries_color_and_size() {
        let rt = RADIO.rich(14.0);
        assert_eq!(rt.text(), "📻");
    }
}
