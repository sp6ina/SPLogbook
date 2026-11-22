// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Band plan editor: mode segments of every band with a multi-band preview.
//! Band edges stay fixed; only the segments inside a band can change.

use crate::core::bandplan::{
    AMATEUR_BANDS, BandPlan, BandPlanError, MAX_LABEL_CHARS, MAX_PLAN_FILE_BYTES,
    MAX_SEGMENTS_PER_BAND, PlanSegment, SegmentMode, get_band_by_name,
};
use crate::core::i18n::{Key, Language, Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::bandmap::segment_colors;
use crate::gui::state::ToastSeverity;
use crate::gui::theme::{Palette, ThemeExt};
use eframe::egui;

/// What the window asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BandPlanEditorAction {
    None,
    Apply(BandPlan),
    Import,
    Export(BandPlan),
}

/// Bands shown in the preview when the editor opens for the first time.
const DEFAULT_PREVIEW: [&str; 6] = ["80m", "40m", "20m", "15m", "10m", "2m"];
/// New segments span this much, or up to the band edge.
const NEW_SEGMENT_HZ: u64 = 10_000;
const PREVIEW_BAR_HEIGHT: f32 = 22.0;
const PREVIEW_NAME_WIDTH: f32 = 48.0;

pub struct BandPlanEditor {
    pub is_open: bool,
    draft: BandPlan,
    selected: String,
    preview: Vec<String>,
    issue: Option<BandPlanError>,
    dirty: bool,
}

impl Default for BandPlanEditor {
    fn default() -> Self {
        Self {
            is_open: false,
            draft: BandPlan::iaru_region1().clone(),
            selected: "20m".to_string(),
            preview: DEFAULT_PREVIEW.iter().map(ToString::to_string).collect(),
            issue: None,
            dirty: false,
        }
    }
}

pub fn mode_label(mode: SegmentMode, lang: Language) -> &'static str {
    let k = match mode {
        SegmentMode::Cw => key::BANDPLAN_MODE_CW,
        SegmentMode::Data => key::BANDPLAN_MODE_DATA,
        SegmentMode::Ssb => key::BANDPLAN_MODE_SSB,
        SegmentMode::Fm => key::BANDPLAN_MODE_FM,
        SegmentMode::Beacon => key::BANDPLAN_MODE_BEACON,
    };
    tr(k, lang)
}

/// Translated description of a rejected plan.
pub fn issue_text(issue: &BandPlanError, lang: Language) -> String {
    let (k, band, row): (Key, &str, Option<usize>) = match issue {
        BandPlanError::UnknownBand(band) => (key::BANDPLAN_ERR_UNKNOWN_BAND, band, None),
        BandPlanError::DuplicateBand(band) => (key::BANDPLAN_ERR_DUPLICATE_BAND, band, None),
        BandPlanError::TooManySegments { band } => (key::BANDPLAN_ERR_TOO_MANY, band, None),
        BandPlanError::EmptyLabel { band, index } => {
            (key::BANDPLAN_ERR_EMPTY_LABEL, band, Some(*index))
        }
        BandPlanError::LabelTooLong { band, index } => {
            (key::BANDPLAN_ERR_LABEL_TOO_LONG, band, Some(*index))
        }
        BandPlanError::EmptyRange { band, index } => {
            (key::BANDPLAN_ERR_EMPTY_RANGE, band, Some(*index))
        }
        BandPlanError::OutsideBand { band, index } => {
            (key::BANDPLAN_ERR_OUTSIDE_BAND, band, Some(*index))
        }
        BandPlanError::Overlap { band, index } => (key::BANDPLAN_ERR_OVERLAP, band, Some(*index)),
        BandPlanError::TooLarge => (key::BANDPLAN_ERR_TOO_LARGE, "", None),
        BandPlanError::Format(_) => (key::BANDPLAN_ERR_FORMAT, "", None),
    };
    Msg::new(k)
        .arg("band", band)
        .arg("row", row.map_or(0, |r| r + 1).to_string())
        .arg("max", MAX_SEGMENTS_PER_BAND.to_string())
        .arg("chars", MAX_LABEL_CHARS.to_string())
        .text(lang)
}

fn khz(hz: u64) -> f64 {
    hz as f64 / 1000.0
}

fn hz(khz: f64) -> u64 {
    (khz * 1000.0).round().max(0.0) as u64
}

impl BandPlanEditor {
    /// Opens the window on a copy of the plan in use.
    pub fn open_with(&mut self, plan: &BandPlan) {
        self.set_draft(plan.clone());
        self.dirty = false;
        self.is_open = true;
    }

    /// Replaces the draft, e.g. after an import; the change still needs Apply.
    pub fn set_draft(&mut self, plan: BandPlan) {
        self.draft = plan;
        self.dirty = true;
        self.check();
    }

    /// Marks the draft as saved.
    pub fn mark_applied(&mut self) {
        self.dirty = false;
    }

    pub fn draft(&self) -> &BandPlan {
        &self.draft
    }

    pub fn issue(&self) -> Option<&BandPlanError> {
        self.issue.as_ref()
    }

    fn check(&mut self) {
        self.issue = self.draft.validate().err();
    }

    fn edited(&mut self) {
        self.dirty = true;
        self.check();
    }

    /// Selects a band for editing and makes sure it is in the preview.
    pub fn select_band(&mut self, name: &str) {
        if get_band_by_name(name).is_some_and(|b| b.name == name) {
            self.selected = name.to_string();
            if !self.preview.iter().any(|p| p == name) {
                self.preview.push(name.to_string());
            }
        }
    }

    /// Adds a segment after the last one of the selected band.
    pub fn add_segment(&mut self) {
        let Some(def) = get_band_by_name(&self.selected) else {
            return;
        };
        let Some(band) = self.draft.band_mut(def.name) else {
            return;
        };
        if band.segments.len() >= MAX_SEGMENTS_PER_BAND {
            return;
        }
        let mut start = band
            .segments
            .last()
            .map_or(def.min_freq_hz, |seg| seg.end_hz);
        if start >= def.max_freq_hz {
            start = def.min_freq_hz;
        }
        band.segments.push(PlanSegment {
            label: "CW".to_string(),
            start_hz: start,
            end_hz: (start + NEW_SEGMENT_HZ).min(def.max_freq_hz),
            mode: SegmentMode::Cw,
        });
        self.edited();
    }

    pub fn remove_segment(&mut self, index: usize) {
        if let Some(band) = self.draft.band_mut(&self.selected)
            && index < band.segments.len()
        {
            band.segments.remove(index);
            self.edited();
        }
    }

    pub fn reset_selected_band(&mut self) {
        let name = self.selected.clone();
        self.draft.reset_band(&name);
        self.edited();
    }

    pub fn reset_all(&mut self) {
        self.draft = BandPlan::iaru_region1().clone();
        self.edited();
    }

    /// Sorts the draft and returns it if it is valid; otherwise keeps the problem
    /// for display, pointing at the sorted rows.
    pub fn try_apply(&mut self) -> Option<BandPlan> {
        self.draft.normalize();
        self.check();
        self.issue.is_none().then(|| self.draft.clone())
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: Language) -> BandPlanEditorAction {
        if !self.is_open {
            return BandPlanEditorAction::None;
        }
        let mut action = BandPlanEditorAction::None;
        let mut open = self.is_open;
        egui::Window::new(tr(key::BANDPLAN_TITLE, lang))
            .id(egui::Id::new("bandplan_editor"))
            .open(&mut open)
            .default_width(900.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                self.toolbar(ui, lang, &mut action);
                self.status_line(ui, &palette, lang);
                ui.separator();
                ui.columns(2, |cols| {
                    self.band_list(&mut cols[0], lang);
                    self.segment_table(&mut cols[1], &palette, lang);
                });
                ui.separator();
                ui.strong(tr(key::BANDPLAN_PREVIEW, lang));
                self.preview(ui, &palette, lang);
            });
        self.is_open = open;
        action
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, lang: Language, action: &mut BandPlanEditorAction) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.dirty, egui::Button::new(tr(key::BANDPLAN_APPLY, lang)))
                .clicked()
                && let Some(plan) = self.try_apply()
            {
                *action = BandPlanEditorAction::Apply(plan);
            }
            if ui.button(tr(key::BANDPLAN_RESET_BAND, lang)).clicked() {
                self.reset_selected_band();
            }
            if ui.button(tr(key::BANDPLAN_RESET_ALL, lang)).clicked() {
                self.reset_all();
            }
            ui.separator();
            if ui.button(tr(key::BANDPLAN_IMPORT, lang)).clicked() {
                *action = BandPlanEditorAction::Import;
            }
            if ui
                .add_enabled(
                    self.issue.is_none(),
                    egui::Button::new(tr(key::BANDPLAN_EXPORT, lang)),
                )
                .clicked()
            {
                let mut plan = self.draft.clone();
                plan.normalize();
                *action = BandPlanEditorAction::Export(plan);
            }
        });
    }

    fn status_line(&self, ui: &mut egui::Ui, palette: &Palette, lang: Language) {
        ui.horizontal(|ui| {
            if let Some(issue) = &self.issue {
                ui.colored_label(palette.danger, format!("⚠ {}", issue_text(issue, lang)));
            } else {
                ui.colored_label(palette.success, tr(key::BANDPLAN_VALID, lang));
            }
            if self.dirty {
                ui.label(
                    egui::RichText::new(tr(key::BANDPLAN_UNSAVED, lang)).color(palette.warning),
                );
            }
        });
    }

    fn band_list(&mut self, ui: &mut egui::Ui, lang: Language) {
        ui.strong(tr(key::BANDPLAN_BANDS, lang));
        ui.label(
            egui::RichText::new(tr(key::BANDPLAN_BANDS_HINT, lang))
                .small()
                .color(ui.palette().text_muted),
        );
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("bandplan_bands")
            .max_height(260.0)
            .show(ui, |ui| {
                for def in AMATEUR_BANDS {
                    ui.horizontal(|ui| {
                        let mut shown = self.preview.iter().any(|p| p == def.name);
                        if ui
                            .checkbox(&mut shown, "")
                            .on_hover_text(tr(key::BANDPLAN_IN_PREVIEW, lang))
                            .changed()
                        {
                            if shown {
                                self.preview.push(def.name.to_string());
                            } else {
                                self.preview.retain(|p| p != def.name);
                            }
                        }
                        let count = self.draft.band(def.name).map_or(0, |b| b.segments.len());
                        let text = format!(
                            "{}  {:.3}–{:.3} MHz  ({count})",
                            def.name,
                            def.min_freq_hz as f64 / 1e6,
                            def.max_freq_hz as f64 / 1e6
                        );
                        if ui
                            .selectable_label(self.selected == def.name, text)
                            .clicked()
                        {
                            select = Some(def.name);
                        }
                    });
                }
            });
        if let Some(name) = select {
            self.select_band(name);
        }
    }

    fn segment_table(&mut self, ui: &mut egui::Ui, palette: &Palette, lang: Language) {
        let Some(def) = get_band_by_name(&self.selected) else {
            return;
        };
        ui.strong(
            Msg::new(key::BANDPLAN_SEGMENTS_OF)
                .arg("band", def.name)
                .text(lang),
        );
        let flagged = self
            .issue
            .as_ref()
            .and_then(BandPlanError::location)
            .filter(|(band, _)| *band == def.name)
            .and_then(|(_, row)| row);
        let (min_khz, max_khz) = (khz(def.min_freq_hz), khz(def.max_freq_hz));
        let mut changed = false;
        let mut remove = None;
        let Some(band) = self.draft.band_mut(def.name) else {
            return;
        };
        egui::ScrollArea::vertical()
            .id_salt("bandplan_segments")
            .max_height(230.0)
            .show(ui, |ui| {
                egui::Grid::new("bandplan_segment_grid")
                    .striped(true)
                    .num_columns(6)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.strong(tr(key::BANDPLAN_COL_LABEL, lang));
                        ui.strong(tr(key::BANDPLAN_COL_START, lang));
                        ui.strong(tr(key::BANDPLAN_COL_END, lang));
                        ui.strong(tr(key::BANDPLAN_COL_MODE, lang));
                        ui.label("");
                        ui.end_row();
                        for (index, seg) in band.segments.iter_mut().enumerate() {
                            if flagged == Some(index) {
                                ui.colored_label(palette.danger, "⚠");
                            } else {
                                ui.label(format!("{}", index + 1));
                            }
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut seg.label)
                                        .char_limit(MAX_LABEL_CHARS)
                                        .desired_width(110.0),
                                )
                                .changed();
                            for edge in [&mut seg.start_hz, &mut seg.end_hz] {
                                let mut value = khz(*edge);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut value)
                                            .range(min_khz..=max_khz)
                                            .speed(0.5)
                                            .max_decimals(3)
                                            .suffix(" kHz"),
                                    )
                                    .changed()
                                {
                                    *edge = hz(value);
                                    changed = true;
                                }
                            }
                            egui::ComboBox::from_id_salt(("bandplan_mode", index))
                                .selected_text(mode_label(seg.mode, lang))
                                .show_ui(ui, |ui| {
                                    for mode in SegmentMode::ALL {
                                        changed |= ui
                                            .selectable_value(
                                                &mut seg.mode,
                                                mode,
                                                mode_label(mode, lang),
                                            )
                                            .changed();
                                    }
                                });
                            if ui
                                .small_button("🗑")
                                .on_hover_text(tr(key::BTN_DELETE, lang))
                                .clicked()
                            {
                                remove = Some(index);
                            }
                            ui.end_row();
                        }
                    });
            });
        let full = band.segments.len() >= MAX_SEGMENTS_PER_BAND;
        if changed {
            self.edited();
        }
        if let Some(index) = remove {
            self.remove_segment(index);
        }
        if ui
            .add_enabled(
                !full,
                egui::Button::new(tr(key::BANDPLAN_ADD_SEGMENT, lang)),
            )
            .clicked()
        {
            self.add_segment();
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui, palette: &Palette, lang: Language) {
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("bandplan_preview")
            .show(ui, |ui| {
                for def in AMATEUR_BANDS
                    .iter()
                    .filter(|d| self.preview.iter().any(|p| p == d.name))
                {
                    ui.horizontal(|ui| {
                        let name = egui::RichText::new(def.name).monospace();
                        let name = if self.selected == def.name {
                            name.strong().color(palette.accent)
                        } else {
                            name
                        };
                        ui.add_sized(
                            [PREVIEW_NAME_WIDTH, PREVIEW_BAR_HEIGHT],
                            egui::Label::new(name),
                        );
                        let size = egui::vec2(ui.available_width(), PREVIEW_BAR_HEIGHT);
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        let segments = self.draft.band(def.name).map_or(&[][..], |b| &b.segments);
                        paint_band(
                            ui,
                            rect,
                            def.min_freq_hz,
                            def.max_freq_hz,
                            segments,
                            palette,
                        );
                        if self.selected == def.name {
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.5, palette.accent),
                                egui::StrokeKind::Outside,
                            );
                        }
                        let hovered = response.hover_pos().and_then(|pos| {
                            let span = (def.max_freq_hz - def.min_freq_hz) as f32;
                            let fraction = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                            let freq = def.min_freq_hz + (fraction * span) as u64;
                            segments
                                .iter()
                                .find(|s| (s.start_hz..=s.end_hz).contains(&freq))
                                .map(|s| (freq, s))
                        });
                        let response = if let Some((freq, seg)) = hovered {
                            response.on_hover_text(format!(
                                "{:.3} MHz\n{}: {:.3}–{:.3} MHz, {}",
                                freq as f64 / 1e6,
                                seg.label,
                                seg.start_hz as f64 / 1e6,
                                seg.end_hz as f64 / 1e6,
                                mode_label(seg.mode, lang)
                            ))
                        } else {
                            response
                        };
                        if response.clicked() {
                            select = Some(def.name);
                        }
                    });
                }
            });
        ui.horizontal_wrapped(|ui| {
            for mode in SegmentMode::ALL {
                let (fill, _) = segment_colors(&palette.segments, mode);
                ui.colored_label(fill, "■");
                ui.label(mode_label(mode, lang));
            }
        });
        if let Some(name) = select {
            self.select_band(name);
        }
    }
}

/// Draws one band with its segments scaled to the band edges.
fn paint_band(
    ui: &egui::Ui,
    rect: egui::Rect,
    min_hz: u64,
    max_hz: u64,
    segments: &[PlanSegment],
    palette: &Palette,
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, palette.surface_bg);
    let span = (max_hz - min_hz) as f32;
    let x = |freq: u64| {
        rect.min.x + ((freq.saturating_sub(min_hz)) as f32 / span).clamp(0.0, 1.0) * rect.width()
    };
    for seg in segments {
        let seg_rect = egui::Rect::from_min_max(
            egui::pos2(x(seg.start_hz), rect.min.y),
            egui::pos2(x(seg.end_hz), rect.max.y),
        );
        let (fill, text) = segment_colors(&palette.segments, seg.mode);
        painter.rect_filled(seg_rect, 1.0, fill);
        let font = egui::FontId::monospace(10.0);
        let galley = painter.layout_no_wrap(seg.label.clone(), font, text);
        if galley.size().x + 4.0 < seg_rect.width() {
            painter.galley(seg_rect.center() - galley.size() / 2.0, galley, text);
        }
    }
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, palette.border),
        egui::StrokeKind::Inside,
    );
}

/// Reads a band plan file, refusing files larger than [`MAX_PLAN_FILE_BYTES`].
fn read_plan_file(path: &std::path::Path) -> Result<BandPlan, BandPlanError> {
    let len = std::fs::metadata(path)
        .map_err(|e| BandPlanError::Format(e.to_string()))?
        .len();
    if len > MAX_PLAN_FILE_BYTES {
        return Err(BandPlanError::TooLarge);
    }
    let text = std::fs::read_to_string(path).map_err(|e| BandPlanError::Format(e.to_string()))?;
    BandPlan::from_json(&text)
}

impl SpLogApp {
    /// Opens the editor on the plan in use.
    pub fn open_bandplan_editor(&mut self) {
        let plan = self.config.active_bandplan().clone();
        self.gui.dialogs.bandplan_editor.open_with(&plan);
    }

    /// Stores the applied plan; the built-in plan is stored as "no user plan".
    pub fn apply_bandplan(&mut self, plan: BandPlan) {
        self.config.bandplan = (plan != *BandPlan::iaru_region1()).then_some(plan);
        self.save_station_config();
        self.gui.dialogs.bandplan_editor.mark_applied();
    }

    /// Carries out what the band plan editor asked for.
    pub fn handle_bandplan_action(&mut self, action: BandPlanEditorAction) {
        let lang = self.gui.appearance.language;
        match action {
            BandPlanEditorAction::None => {}
            BandPlanEditorAction::Apply(plan) => {
                self.apply_bandplan(plan);
                self.gui
                    .chrome
                    .push_status_toast(ToastSeverity::Success, tr(key::BANDPLAN_SAVED, lang));
            }
            BandPlanEditorAction::Import => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(key::BANDPLAN_FILE_FILTER, lang), &["json"])
                    .set_title(tr(key::BANDPLAN_IMPORT, lang))
                    .pick_file()
                else {
                    return;
                };
                match read_plan_file(&path) {
                    Ok(plan) => {
                        self.gui.dialogs.bandplan_editor.set_draft(plan);
                        self.gui.chrome.push_status_toast(
                            ToastSeverity::Info,
                            tr(key::BANDPLAN_IMPORTED, lang),
                        );
                    }
                    Err(e) => {
                        log::warn!("band plan not imported: {e}");
                        self.gui
                            .chrome
                            .push_status_toast(ToastSeverity::Error, issue_text(&e, lang));
                    }
                }
            }
            BandPlanEditorAction::Export(plan) => {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr(key::BANDPLAN_FILE_FILTER, lang), &["json"])
                    .set_title(tr(key::BANDPLAN_EXPORT, lang))
                    .set_file_name("splogbook-bandplan.json")
                    .save_file()
                else {
                    return;
                };
                let written = plan
                    .to_json()
                    .map_err(|e| e.to_string())
                    .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()));
                let (severity, k) = match written {
                    Ok(()) => (ToastSeverity::Success, key::BANDPLAN_EXPORTED),
                    Err(e) => {
                        log::warn!("band plan not exported: {e}");
                        (ToastSeverity::Error, key::BANDPLAN_EXPORT_FAILED)
                    }
                };
                self.gui.chrome.push_status_toast(severity, tr(k, lang));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> BandPlanEditor {
        let mut editor = BandPlanEditor::default();
        editor.open_with(BandPlan::iaru_region1());
        editor
    }

    #[test]
    fn opens_clean_on_the_plan_in_use() {
        let editor = editor();
        assert!(editor.is_open);
        assert!(!editor.dirty);
        assert_eq!(editor.issue(), None);
        assert_eq!(editor.draft(), BandPlan::iaru_region1());
    }

    #[test]
    fn added_segment_follows_the_last_one_and_stays_in_the_band() {
        let mut editor = editor();
        editor.select_band("20m");
        let band = editor.draft.band_mut("20m").unwrap();
        band.segments.retain(|s| s.end_hz <= 14_100_000);
        let last_end = band.segments.last().unwrap().end_hz;
        editor.add_segment();
        let seg = editor
            .draft
            .band("20m")
            .unwrap()
            .segments
            .last()
            .unwrap()
            .clone();
        assert_eq!(seg.start_hz, last_end);
        assert_eq!(seg.end_hz, last_end + NEW_SEGMENT_HZ);
        assert!(editor.dirty);
        assert_eq!(editor.issue(), None);

        // Przy górnej granicy pasma nowy wycinek kończy się na niej.
        editor.select_band("60m");
        editor.draft.band_mut("60m").unwrap().segments.clear();
        editor.add_segment();
        let seg = &editor.draft.band("60m").unwrap().segments[0];
        assert_eq!(seg.start_hz, 5_351_500);
        assert_eq!(seg.end_hz, 5_361_500);
    }

    #[test]
    fn segment_limit_is_enforced() {
        let mut editor = editor();
        editor.select_band("2m");
        for _ in 0..MAX_SEGMENTS_PER_BAND + 3 {
            editor.add_segment();
        }
        assert_eq!(
            editor.draft.band("2m").unwrap().segments.len(),
            MAX_SEGMENTS_PER_BAND
        );
    }

    #[test]
    fn apply_sorts_rows_and_rejects_overlaps() {
        let mut editor = editor();
        editor.select_band("20m");
        let band = editor.draft.band_mut("20m").unwrap();
        band.segments.reverse();
        editor.edited();
        // Odwrócona kolejność to nakładanie się w kolejności wierszy…
        assert!(matches!(
            editor.issue(),
            Some(BandPlanError::Overlap { .. })
        ));
        // …ale po sortowaniu przy zastosowaniu plan jest poprawny.
        let applied = editor.try_apply().expect("sorted plan is valid");
        assert_eq!(&applied, BandPlan::iaru_region1());

        let band = editor.draft.band_mut("20m").unwrap();
        band.segments[1].start_hz = band.segments[0].start_hz;
        editor.edited();
        assert_eq!(editor.try_apply(), None);
        assert_eq!(
            editor.issue().and_then(BandPlanError::location),
            Some(("20m", Some(1)))
        );
    }

    #[test]
    fn reset_restores_iaru_segments() {
        let mut editor = editor();
        editor.select_band("40m");
        editor.remove_segment(0);
        editor.draft.band_mut("20m").unwrap().segments.clear();
        editor.reset_selected_band();
        assert_eq!(
            editor.draft.band("40m"),
            BandPlan::iaru_region1().band("40m")
        );
        assert_ne!(
            editor.draft.band("20m"),
            BandPlan::iaru_region1().band("20m")
        );
        editor.reset_all();
        assert_eq!(editor.draft(), BandPlan::iaru_region1());
    }

    #[test]
    fn selecting_a_band_adds_it_to_the_preview_once() {
        let mut editor = editor();
        editor.select_band("6m");
        editor.select_band("6m");
        assert_eq!(editor.preview.iter().filter(|b| *b == "6m").count(), 1);
        editor.select_band("11m");
        assert_eq!(editor.selected, "6m");
    }

    #[test]
    fn every_issue_has_a_translated_message() {
        let issues = [
            BandPlanError::UnknownBand("11m".into()),
            BandPlanError::DuplicateBand("20m".into()),
            BandPlanError::TooManySegments { band: "20m".into() },
            BandPlanError::EmptyLabel {
                band: "20m".into(),
                index: 0,
            },
            BandPlanError::LabelTooLong {
                band: "20m".into(),
                index: 0,
            },
            BandPlanError::EmptyRange {
                band: "20m".into(),
                index: 0,
            },
            BandPlanError::OutsideBand {
                band: "20m".into(),
                index: 0,
            },
            BandPlanError::Overlap {
                band: "20m".into(),
                index: 2,
            },
            BandPlanError::TooLarge,
            BandPlanError::Format("x".into()),
        ];
        for lang in [Language::Pl, Language::En, Language::Ru] {
            for issue in &issues {
                let text = issue_text(issue, lang);
                assert_ne!(text, "");
                assert!(!text.contains('{'), "{text}");
            }
        }
        assert!(issue_text(&issues[7], Language::En).contains('3'));
    }

    #[test]
    fn plan_files_are_read_with_a_size_limit() {
        let dir = std::env::temp_dir().join(format!("splog_bandplan_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("plan.json");
        std::fs::write(&good, BandPlan::iaru_region1().to_json().unwrap()).unwrap();
        assert_eq!(read_plan_file(&good).as_ref(), Ok(BandPlan::iaru_region1()));

        let big = dir.join("big.json");
        std::fs::write(&big, vec![b' '; MAX_PLAN_FILE_BYTES as usize + 1]).unwrap();
        assert_eq!(read_plan_file(&big), Err(BandPlanError::TooLarge));
        assert!(matches!(
            read_plan_file(&dir.join("missing.json")),
            Err(BandPlanError::Format(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn window_renders_without_panicking() {
        let ctx = egui::Context::default();
        let mut editor = editor();
        editor.select_band("20m");
        let mut action = BandPlanEditorAction::Import;
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                action = editor.show(ui.ctx(), Language::Pl);
            });
            output.textures_delta.clear();
        }
        assert_eq!(action, BandPlanEditorAction::None);
        assert!(editor.is_open);
    }
}
