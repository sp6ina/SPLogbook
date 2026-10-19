// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny rejestr ikon SPLogbook.
//!
//! Wszystkie glify oraz ich kolory semantyczne są zdefiniowane **tutaj**,
//! aby zamiast rozrzuconej mieszanki emoji/unicode panował jeden, spójny
//! zestaw. Nowe ikony dodawaj wyłącznie w tym pliku i odwołuj się do nich
//! przez stałe (np. `icons::RADIO`), a nie przez wstawianie gołego glifu.

use crate::gui::theme::Palette;
use eframe::egui::RichText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconColor {
    Accent,
    Success,
    Danger,
    Warn,
    Info,
    Neutral,
    Highlight,
    Gold,
    Violet,
}

impl IconColor {
    pub fn resolve(self, palette: &Palette) -> eframe::egui::Color32 {
        match self {
            IconColor::Accent => palette.accent,
            IconColor::Success => palette.success,
            IconColor::Danger => palette.danger,
            IconColor::Warn => palette.warning,
            IconColor::Info => palette.info,
            IconColor::Neutral => palette.text_muted,
            IconColor::Highlight => palette.accent_alt,
            IconColor::Gold => palette.warning,
            IconColor::Violet => palette.accent_alt,
        }
    }
}

// Paleta semantyczna ikon (spójna z theme::Palette)
pub const ACCENT: IconColor = IconColor::Accent;
pub const SUCCESS: IconColor = IconColor::Success;
pub const DANGER: IconColor = IconColor::Danger;
pub const WARN: IconColor = IconColor::Warn;
pub const INFO: IconColor = IconColor::Info;
pub const NEUTRAL: IconColor = IconColor::Neutral;
pub const HIGHLIGHT: IconColor = IconColor::Highlight;
pub const GOLD: IconColor = IconColor::Gold;
pub const VIOLET: IconColor = IconColor::Violet;

// Ikona = glif + rola semantyczna
#[derive(Clone, Copy)]
pub struct Icon {
    pub glyph: &'static str,
    pub color_role: IconColor,
}

impl Icon {
    pub const fn new(glyph: &'static str, color_role: IconColor) -> Self {
        Self { glyph, color_role }
    }

    /// Zwraca goły glif (do użycia w `format!`).
    pub fn as_str(self) -> &'static str {
        self.glyph
    }

    /// Glif poprzedzający etykietę: `"📻 Transceiver"`.
    pub fn label(self, text: impl Into<String>) -> String {
        format!("{} {}", self.glyph, text.into())
    }

    /// Tekst sformatowany (rozmiar).
    pub fn rich(self, size: f32) -> RichText {
        RichText::new(self.glyph).size(size)
    }

    /// Tekst sformatowany z uwzględnieniem aktywnej palety.
    pub fn rich_colored(self, palette: &Palette, size: f32) -> RichText {
        RichText::new(self.glyph)
            .color(self.color_role.resolve(palette))
            .size(size)
    }

    /// Etykieta sformatowana (glif + tekst).
    pub fn rich_label(self, text: impl Into<String>, size: f32) -> RichText {
        RichText::new(format!("{} {}", self.glyph, text.into())).size(size)
    }
}

// Radio / CAT / sprzęt
pub const RADIO: Icon = Icon::new("📻", ACCENT);
pub const VFO_KNOB: Icon = Icon::new("🎛", ACCENT);
pub const ROTOR: Icon = Icon::new("🧭", ACCENT);
pub const SO2R: Icon = Icon::new("⇄", ACCENT);
pub const CW_KEYER: Icon = Icon::new("⚡", WARN);
pub const CW_TERMINAL: Icon = Icon::new("📟", ACCENT);
pub const PTT_MIC: Icon = Icon::new("🎙", ACCENT);
pub const VOICE_KEYER: Icon = Icon::new("🎙", ACCENT);
pub const CAT_LINK: Icon = Icon::new("🔗", INFO);

// Dziennik / łączności
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

// Pasmo / klaster / mapa
pub const CLUSTER: Icon = Icon::new("📡", ACCENT);
pub const BANDMAP: Icon = Icon::new("📶", ACCENT);
pub const SPOT: Icon = Icon::new("📢", ACCENT);
pub const WORLD_MAP: Icon = Icon::new("🗺", ACCENT);
pub const QSY: Icon = Icon::new("📻", ACCENT);
pub const ANTENNA_TUNER: Icon = Icon::new("📶", ACCENT);
pub const SIGNAL_UP: Icon = Icon::new("⬆", SUCCESS);
pub const SIGNAL_DOWN: Icon = Icon::new("⬇", DANGER);

// Propagacja / pogoda kosmiczna
pub const SOLAR: Icon = Icon::new("☀", GOLD);
pub const SUN: Icon = Icon::new("🌞", GOLD);
pub const MOON: Icon = Icon::new("🌙", NEUTRAL);
pub const NEW_MOON: Icon = Icon::new("🌑", NEUTRAL);
pub const SATELLITE: Icon = Icon::new("🛰", ACCENT);
pub const ASTRONOMY: Icon = Icon::new("🔬", VIOLET);
pub const STORM: Icon = Icon::new("🌪", WARN);

// Nagrody / osiągnięcia
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

// Konfiguracja / narzędzia
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

// Status / stany
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

// Różne / funkcje specjalne
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

// Testy
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
        let p = crate::gui::theme::ThemePreset::OperatorDark.palette();
        assert_eq!(SUCCESS.resolve(&p), p.success);
        assert_eq!(DANGER.resolve(&p), p.danger);
        assert_eq!(ACCENT.resolve(&p), p.accent);
    }

    #[test]
    fn rich_text_carries_color_and_size() {
        let rt = RADIO.rich(14.0);
        assert_eq!(rt.text(), "📻");
    }
}
