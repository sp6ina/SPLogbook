// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Editor of the user's club memberships (same file format as before).

use crate::core::clubs::ClubRow;
use crate::core::i18n::{Language, key, tr};
use eframe::egui;

/// What the window asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClubsEditorAction {
    None,
    Save(Vec<ClubRow>),
    Import,
    Export,
    Reload,
}

#[derive(Default)]
pub struct ClubsEditor {
    pub is_open: bool,
    rows: Vec<ClubRow>,
    filter: String,
    dirty: bool,
}

/// The file format separates fields with commas and has no quoting, so the
/// editor never lets a comma into a field.
fn sanitize(text: &mut String) {
    if text.contains(',') {
        *text = text.replace(',', " ");
    }
}

impl ClubsEditor {
    /// Opens the window with the rows currently in effect.
    pub fn open_with(&mut self, rows: Vec<ClubRow>) {
        self.rows = rows;
        self.dirty = false;
        self.is_open = true;
    }

    /// Replaces the rows after a successful import or save.
    pub fn set_rows(&mut self, rows: Vec<ClubRow>) {
        self.rows = rows;
        self.dirty = false;
    }

    pub fn rows(&self) -> &[ClubRow] {
        &self.rows
    }

    fn matches(&self, r: &ClubRow) -> bool {
        let f = self.filter.trim().to_uppercase();
        f.is_empty()
            || r.callsign.contains(&f)
            || r.code.to_uppercase().contains(&f)
            || r.name.to_uppercase().contains(&f)
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: Language) -> ClubsEditorAction {
        if !self.is_open {
            return ClubsEditorAction::None;
        }
        let mut action = ClubsEditorAction::None;
        let mut open = self.is_open;
        egui::Window::new(tr(key::CLUBS_EDITOR_TITLE, lang))
            .open(&mut open)
            .default_width(760.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(key::CLUBS_FILTER, lang));
                    ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(200.0));
                    if ui.button(tr(key::BTN_ADD, lang)).clicked() {
                        self.rows.push(ClubRow {
                            callsign: String::new(),
                            code: String::new(),
                            name: String::new(),
                            number: None,
                            color: (59, 130, 246),
                        });
                        self.filter.clear();
                        self.dirty = true;
                    }
                    if ui
                        .add_enabled(self.dirty, egui::Button::new(tr(key::CLUBS_SAVE, lang)))
                        .clicked()
                    {
                        action = ClubsEditorAction::Save(self.rows.clone());
                    }
                    if ui.button(tr(key::CLUBS_IMPORT, lang)).clicked() {
                        action = ClubsEditorAction::Import;
                    }
                    if ui.button(tr(key::CLUBS_EXPORT, lang)).clicked() {
                        action = ClubsEditorAction::Export;
                    }
                    if ui.button(tr(key::BTN_REFRESH, lang)).clicked() {
                        action = ClubsEditorAction::Reload;
                    }
                });
                if self.dirty {
                    ui.label(tr(key::CLUBS_UNSAVED, lang));
                }
                ui.separator();
                if self.rows.is_empty() {
                    ui.label(tr(key::CLUBS_EMPTY, lang));
                }
                let mut delete = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("clubs_editor_grid")
                        .striped(true)
                        .num_columns(6)
                        .show(ui, |ui| {
                            ui.strong(tr(key::CLUBS_COL_CALLSIGN, lang));
                            ui.strong(tr(key::CLUBS_COL_CODE, lang));
                            ui.strong(tr(key::CLUBS_COL_NAME, lang));
                            ui.strong(tr(key::CLUBS_COL_NUMBER, lang));
                            ui.strong(tr(key::CLUBS_COL_COLOR, lang));
                            ui.label("");
                            ui.end_row();
                            for idx in 0..self.rows.len() {
                                if !self.matches(&self.rows[idx]) {
                                    continue;
                                }
                                let row = &mut self.rows[idx];
                                let mut changed = false;
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut row.callsign)
                                            .desired_width(90.0),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut row.code)
                                            .desired_width(80.0),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::TextEdit::singleline(&mut row.name)
                                            .desired_width(200.0),
                                    )
                                    .changed();
                                let mut number = row.number.clone().unwrap_or_default();
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut number).desired_width(70.0),
                                    )
                                    .changed()
                                {
                                    sanitize(&mut number);
                                    row.number = (!number.trim().is_empty())
                                        .then(|| number.trim().to_string());
                                    changed = true;
                                }
                                let mut rgb = [row.color.0, row.color.1, row.color.2];
                                if ui.color_edit_button_srgb(&mut rgb).changed() {
                                    row.color = (rgb[0], rgb[1], rgb[2]);
                                    changed = true;
                                }
                                if ui.button(tr(key::BTN_DELETE, lang)).clicked() {
                                    delete = Some(idx);
                                }
                                ui.end_row();
                                if changed {
                                    row.callsign = row.callsign.trim().to_uppercase();
                                    sanitize(&mut row.callsign);
                                    sanitize(&mut row.code);
                                    sanitize(&mut row.name);
                                    self.dirty = true;
                                }
                            }
                        });
                });
                if let Some(idx) = delete {
                    self.rows.remove(idx);
                    self.dirty = true;
                }
            });
        self.is_open = open;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commas_never_reach_the_file() {
        let mut s = "Club, with comma".to_string();
        sanitize(&mut s);
        assert_eq!(s, "Club  with comma");
    }

    #[test]
    fn filter_matches_call_code_and_name() {
        let mut e = ClubsEditor::default();
        e.open_with(vec![ClubRow {
            callsign: "QA1AA".into(),
            code: "QCLUB".into(),
            name: "Test Club".into(),
            number: None,
            color: (1, 2, 3),
        }]);
        for f in ["", "qa1", "qclub", "test"] {
            e.filter = f.into();
            assert!(e.matches(&e.rows()[0]), "{f}");
        }
        e.filter = "zzz".into();
        assert!(!e.matches(&e.rows()[0]));
    }
}
