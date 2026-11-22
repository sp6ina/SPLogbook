// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Workspace pages: named layouts switched with one click or Ctrl+PgUp/PgDn.
//!
//! The active page is the live layout (panels, modules and the dock tree). Switching
//! first stores the live layout in the active page and then applies the target page.
//! Without pages the workspace behaves as a single implicit page.

use crate::core::i18n::{Msg, key, tr};
use crate::core::station::{WorkspacePage, WorkspaceProfile};
use crate::gui::app::SpLogApp;
use crate::gui::workspace_profiles::{apply_profile, capture_profile};
use eframe::egui;

/// Most pages a workspace may have.
pub const MAX_PAGES: usize = 12;
/// Longest page name, in characters.
pub const MAX_PAGE_NAME_CHARS: usize = 32;

/// Name edited in the rename dialog: page index and the text so far.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageRename {
    pub index: usize,
    pub name: String,
}

/// Trims a page name to [`MAX_PAGE_NAME_CHARS`] characters without control characters.
pub fn clean_page_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_PAGE_NAME_CHARS)
        .collect()
}

impl SpLogApp {
    fn default_page_name(&self, number: usize) -> String {
        Msg::new(key::WORKSPACE_PAGE_DEFAULT_NAME)
            .arg("n", number.to_string())
            .text(self.gui.appearance.language)
    }

    /// The live layout as a page; theme and cluster filter are not part of a page.
    fn capture_page_layout(&self, name: &str) -> WorkspaceProfile {
        let mut layout = capture_profile(self, name);
        layout.theme_preset = None;
        layout.cluster_filter = None;
        layout
    }

    pub fn workspace_pages(&self) -> &[WorkspacePage] {
        &self.config.workspace_pages
    }

    pub fn active_workspace_page(&self) -> usize {
        self.config.active_workspace_page
    }

    /// Adds a page holding a copy of the current layout and switches to it.
    pub fn add_workspace_page(&mut self) {
        if self.config.workspace_pages.len() >= MAX_PAGES {
            return;
        }
        if self.config.workspace_pages.is_empty() {
            let name = self.default_page_name(1);
            let layout = self.capture_page_layout(&name);
            self.config
                .workspace_pages
                .push(WorkspacePage { name, layout });
            self.config.active_workspace_page = 0;
        }
        self.store_active_page();
        let name = self.default_page_name(self.config.workspace_pages.len() + 1);
        let layout = self.capture_page_layout(&name);
        self.config
            .workspace_pages
            .push(WorkspacePage { name, layout });
        self.config.active_workspace_page = self.config.workspace_pages.len() - 1;
        self.save_station_config();
    }

    /// Stores the live layout in the active page.
    fn store_active_page(&mut self) {
        let index = self.config.active_workspace_page;
        if let Some(name) = self
            .config
            .workspace_pages
            .get(index)
            .map(|page| page.name.clone())
        {
            let layout = self.capture_page_layout(&name);
            self.config.workspace_pages[index].layout = layout;
        }
    }

    pub fn switch_workspace_page(&mut self, index: usize) {
        if index == self.config.active_workspace_page || index >= self.config.workspace_pages.len()
        {
            return;
        }
        self.store_active_page();
        self.config.active_workspace_page = index;
        let layout = self.config.workspace_pages[index].layout.clone();
        // `apply_profile` zapisuje konfigurację, więc nowy indeks trafia na dysk.
        apply_profile(self, &layout);
    }

    /// Moves to the previous (`-1`) or next (`1`) page, wrapping around.
    pub fn step_workspace_page(&mut self, step: isize) {
        let count = self.config.workspace_pages.len();
        if count < 2 {
            return;
        }
        let current = self.config.active_workspace_page as isize;
        let target = (current + step).rem_euclid(count as isize) as usize;
        self.switch_workspace_page(target);
    }

    pub fn rename_workspace_page(&mut self, index: usize, name: &str) {
        let name = clean_page_name(name);
        if name.is_empty() {
            return;
        }
        if let Some(page) = self.config.workspace_pages.get_mut(index) {
            page.layout.name.clone_from(&name);
            page.name = name;
            self.save_station_config();
        }
    }

    /// Removes a page; the last remaining page cannot be removed.
    pub fn remove_workspace_page(&mut self, index: usize) {
        let count = self.config.workspace_pages.len();
        if count < 2 || index >= count {
            return;
        }
        if index == self.config.active_workspace_page {
            let neighbour = if index + 1 < count {
                index + 1
            } else {
                index - 1
            };
            self.switch_workspace_page(neighbour);
        }
        self.config.workspace_pages.remove(index);
        if self.config.active_workspace_page > index {
            self.config.active_workspace_page -= 1;
        }
        if self.config.workspace_pages.len() == 1 {
            // Jedna strona to zwykły pulpit bez paska stron.
            self.config.workspace_pages.clear();
            self.config.active_workspace_page = 0;
        }
        self.save_station_config();
    }

    pub fn start_page_rename(&mut self, index: usize) {
        if let Some(page) = self.config.workspace_pages.get(index) {
            self.gui.workspace.page_rename = Some(PageRename {
                index,
                name: page.name.clone(),
            });
        }
    }

    /// Tabs of the pages above the desktop, shown when there are at least two pages.
    pub fn render_workspace_page_bar(&mut self, ui: &mut egui::Ui) {
        if self.config.workspace_pages.len() < 2 {
            return;
        }
        let lang = self.gui.appearance.language;
        let mut switch_to = None;
        let mut rename = None;
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for (index, page) in self.config.workspace_pages.iter().enumerate() {
                let response =
                    ui.selectable_label(index == self.config.active_workspace_page, &page.name);
                if response.clicked() {
                    switch_to = Some(index);
                }
                response.context_menu(|ui| {
                    if ui.button(tr(key::WORKSPACE_PAGE_RENAME, lang)).clicked() {
                        rename = Some(index);
                        ui.close();
                    }
                    if ui.button(tr(key::WORKSPACE_PAGE_REMOVE, lang)).clicked() {
                        remove = Some(index);
                        ui.close();
                    }
                });
            }
            if self.config.workspace_pages.len() < MAX_PAGES
                && ui
                    .small_button("+")
                    .on_hover_text(tr(key::WORKSPACE_PAGE_ADD, lang))
                    .clicked()
            {
                self.add_workspace_page();
            }
        });
        ui.separator();
        if let Some(index) = switch_to {
            self.switch_workspace_page(index);
        }
        if let Some(index) = rename {
            self.start_page_rename(index);
        }
        if let Some(index) = remove {
            self.remove_workspace_page(index);
        }
    }

    /// Small dialog for renaming a page.
    pub fn render_page_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.gui.workspace.page_rename.take() else {
            return;
        };
        let lang = self.gui.appearance.language;
        let mut open = true;
        let mut done = false;
        let mut cancel = false;
        egui::Window::new(tr(key::WORKSPACE_PAGE_RENAME, lang))
            .id(egui::Id::new("workspace_page_rename"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(&mut edit.name)
                        .char_limit(MAX_PAGE_NAME_CHARS)
                        .desired_width(220.0),
                );
                field.request_focus();
                let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui
                        .button(tr(key::WORKSPACE_PAGE_RENAME_APPLY, lang))
                        .clicked()
                        || enter
                    {
                        done = true;
                    }
                    if ui.button(tr(key::BTN_CANCEL, lang)).clicked() {
                        cancel = true;
                    }
                });
            });
        if done {
            self.rename_workspace_page(edit.index, &edit.name);
        } else if open && !cancel {
            self.gui.workspace.page_rename = Some(edit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_names_are_trimmed_and_limited() {
        assert_eq!(clean_page_name("  Contest\n "), "Contest");
        assert_eq!(clean_page_name("\t"), "");
        let long = "Ł".repeat(MAX_PAGE_NAME_CHARS + 5);
        assert_eq!(clean_page_name(&long).chars().count(), MAX_PAGE_NAME_CHARS);
    }
}
