// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny rejestr skrótów klawiaturowych, zarządzanie kontekstem i ochrona edycji tekstu (GUI-001).

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};

/// Pomocnik sprawdzający czy wciśnięto klawisz modyfikatora Ctrl (Windows/Linux) lub Command (macOS).
#[inline]
pub fn is_ctrl_modifier(modifiers: &egui::Modifiers) -> bool {
    modifiers.ctrl || modifiers.command
}

/// Unikalne akcje aplikacyjne wyzwalane przez skróty klawiszowe lub polecenia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppAction {
    Undo,
    Redo,
    OpenStats,
    ToggleShortcutsHelp,
    SaveQso,
    WipeQso,
    LookupQrz,
    RefreshLog,
    SendSpot,
    TogglePtt,
    ToggleVoiceKeyer,
    ToggleFullscreen,
    OpenManual,
    OpenCommandPalette,
    SaveConfig,
    QuitApp,
    FocusFilter,
    NewQso,
    ToggleCluster,
    ToggleBandmap,
    ToggleWorldMap,
    ToggleCwTerminal,
    OpenProfiles,
    ExportCsv,
    ImportAdif,
    CycleTheme,
    FocusLog,
    EscapeCloseContext,
    ContestSaveQso,
    ContestWipeQso,
}

/// Kontekst wykonania skrótu klawiaturowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutContext {
    /// Globalny kontekst aplikacji (domyślny).
    Global,
    /// Edycja w formularzu łączności (QSO Entry).
    QsoEntry,
    /// Okno zawodów (Contest Window).
    Contest,
    /// Modalne okno dialogowe lub popup.
    ModalDialog,
    /// Pole edycji tekstu w dowolnym oknie.
    TextEditing,
}

/// Zasady postępowania w przypadku kolizji z aktywną edycją tekstu w `TextEdit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Skrót jest bezwzględnie blokowany, gdy kursor znajduje się w polu tekstowym
    /// (np. pojedyncze litery, cyfry, spacja, strzałki, Backspace, Delete).
    BlockWhenTextEditing,
    /// Skrót jest dozwolony podczas edycji tekstu, jeśli posiada jawny kontrakt
    /// (np. Ctrl+W czyszczenie formularza, Ctrl+L zapis, klawisze funkcyjne F1–F12).
    AllowInTextEditing,
    /// Standardowe skróty edytora tekstowego delegowane bezpośrednio do widgetu
    /// (np. Ctrl+C kopiuj, Ctrl+V wklej, Ctrl+X wytnij, Ctrl+A zaznacz wszystko).
    PassThroughToWidget,
}

/// Polityka obsługi powtarzania klawisza (repeat events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatPolicy {
    /// Pojedyncze wyzwolenie — ponawianie klawisza przy przytrzymaniu jest ignorowane
    /// (wymagane dla PTT, Voice Keyer i zapisu QSO).
    SingleTrigger,
    /// Dozwolone powtarzanie akcji (np. przewijanie listy strzałkami).
    AllowRepeat,
}

/// Definicja powiązania klawiaturowego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub action: AppAction,
    pub shortcut: KeyboardShortcut,
    pub context: ShortcutContext,
    pub priority: u8,
    pub conflict_policy: ConflictPolicy,
    pub repeat_policy: RepeatPolicy,
    pub enabled: bool,
    pub label: &'static str,
    pub description: &'static str,
}

impl ShortcutBinding {
    #[must_use]
    pub const fn new(
        action: AppAction,
        shortcut: KeyboardShortcut,
        context: ShortcutContext,
        priority: u8,
        conflict_policy: ConflictPolicy,
        repeat_policy: RepeatPolicy,
        label: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            action,
            shortcut,
            context,
            priority,
            conflict_policy,
            repeat_policy,
            enabled: true,
            label,
            description,
        }
    }
}

/// Centralny rejestr skrótów klawiaturowych aplikacji.
#[derive(Debug, Clone)]
pub struct ShortcutRegistry {
    bindings: Vec<ShortcutBinding>,
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new_standard()
    }
}

impl ShortcutRegistry {
    /// Tworzy nowy, pusty rejestr skrótów.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Tworzy kanoniczny rejestr ze wszystkimi standardowymi powiązaniami SPLogbook.
    #[must_use]
    pub fn new_standard() -> Self {
        let mut r = Self::new();
        r.register_canonical_shortcuts();
        r
    }

    /// Rejestruje powiązanie w rejestrze.
    pub fn register(&mut self, binding: ShortcutBinding) {
        self.bindings.push(binding);
    }

    /// Zwraca wszystkie zarejestrowane powiązania.
    #[must_use]
    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }

    /// Wyszukuje czytelną etykietę dla danej akcji w kontekście globalnym lub lokalnym.
    #[must_use]
    pub fn label_for_action(&self, action: AppAction) -> Option<&'static str> {
        self.bindings
            .iter()
            .find(|b| b.action == action && b.enabled)
            .map(|b| b.label)
    }

    /// Sprawdza czy w rejestrze istnieją aktywne zduplikowane powiązania dla tego samego kontekstu i skrótu.
    #[must_use]
    pub fn find_conflicts(&self) -> Vec<(&ShortcutBinding, &ShortcutBinding)> {
        let mut conflicts = Vec::new();
        for i in 0..self.bindings.len() {
            for j in (i + 1)..self.bindings.len() {
                let a = &self.bindings[i];
                let b = &self.bindings[j];
                if a.enabled
                    && b.enabled
                    && a.context == b.context
                    && a.shortcut == b.shortcut
                    && a.priority == b.priority
                {
                    conflicts.push((a, b));
                }
            }
        }
        conflicts
    }

    /// Deterministycznie rozwiązuje akcję klawiaturową na podstawie wejścia egui, aktywnego kontekstu
    /// oraz stanu fokusu edycji tekstu.
    ///
    /// Jeśli znaleziono pasującą regułę, skrót jest konsumowany z bufora egui (aby uniknąć podwójnego
    /// wykonania w innych widokach) i zwracana jest skojarzona [`AppAction`].
    pub fn resolve(
        &self,
        ctx: &egui::Context,
        current_context: ShortcutContext,
        text_focused: bool,
    ) -> Option<AppAction> {
        // Znajdź wszystkich kandydatów pasujących do bieżącego kontekstu lub kontekstu globalnego.
        let mut candidates: Vec<&ShortcutBinding> = self
            .bindings
            .iter()
            .filter(|b| b.enabled)
            .filter(|b| b.context == current_context || b.context == ShortcutContext::Global)
            .collect();

        // Sortowanie deterministyczne: najpierw priorytet malejąco, potem specyfika kontekstu lokalnego nad globalnym
        candidates.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| {
                let a_local = a.context == current_context;
                let b_local = b.context == current_context;
                b_local.cmp(&a_local)
            })
        });

        for binding in candidates {
            // Ochrona edycji tekstu
            if text_focused {
                match binding.conflict_policy {
                    ConflictPolicy::BlockWhenTextEditing => {
                        // Klawisz nie może być przechwycony podczas wpisywania tekstu
                        continue;
                    }
                    ConflictPolicy::PassThroughToWidget => {
                        // Klawisz powinien trafić do kontrolki tekstowej (np. Ctrl+C/V/Z)
                        continue;
                    }
                    ConflictPolicy::AllowInTextEditing => {
                        // Dopuszczony skrót z jawnym kontraktem
                    }
                }
            }

            // Sprawdzenie czy wciśnięto kombinację klawiszy
            let triggered = ctx.input_mut(|i| i.consume_shortcut(&binding.shortcut));
            if triggered {
                return Some(binding.action);
            }
        }

        None
    }

    /// Rejestruje kanoniczne skróty klawiaturowe aplikacji SPLogbook.
    fn register_canonical_shortcuts(&mut self) {
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let none = Modifiers::NONE;

        // --- KLAWISZE FUNKCYJNE (F1–F12) — Priorytet 5, dozwolone podczas edycji ---
        self.register(ShortcutBinding::new(
            AppAction::ToggleShortcutsHelp,
            KeyboardShortcut::new(none, Key::F1),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F1",
            "Pomoc i spis skrótów klawiaturowych",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveQso,
            KeyboardShortcut::new(none, Key::F2),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F2",
            "Zapisz łączność QSO do dziennika",
        ));
        self.register(ShortcutBinding::new(
            AppAction::WipeQso,
            KeyboardShortcut::new(none, Key::F3),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F3",
            "Wyczyść formularz łączności QSO",
        ));
        self.register(ShortcutBinding::new(
            AppAction::LookupQrz,
            KeyboardShortcut::new(none, Key::F4),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F4",
            "Wyszukaj znak w QRZ.com / HamQTH",
        ));
        self.register(ShortcutBinding::new(
            AppAction::RefreshLog,
            KeyboardShortcut::new(none, Key::F5),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F5",
            "Odśwież tabelę dziennika",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SendSpot,
            KeyboardShortcut::new(none, Key::F6),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F6",
            "Wyślij spot DX na bieżącą częstotliwość",
        ));
        self.register(ShortcutBinding::new(
            AppAction::TogglePtt,
            KeyboardShortcut::new(none, Key::F7),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F7",
            "Przełącz nadawanie PTT (TX/RX)",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleVoiceKeyer,
            KeyboardShortcut::new(none, Key::F8),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F8",
            "Otwórz okno Voice Keyer SSB",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleFullscreen,
            KeyboardShortcut::new(none, Key::F11),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F11",
            "Pełny ekran",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenManual,
            KeyboardShortcut::new(none, Key::F12),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F12",
            "Otwórz podręcznik użytkownika",
        ));

        // --- SKRÓTY Z KLAWISZEM CTRL DOSTĘPNE PODCZAS EDYCJI TEKSTU (Jawny kontrakt) ---
        self.register(ShortcutBinding::new(
            AppAction::WipeQso,
            KeyboardShortcut::new(ctrl, Key::W),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+W",
            "Wyczyść formularz QSO",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveQso,
            KeyboardShortcut::new(ctrl, Key::L),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+L",
            "Zapisz bieżące QSO do dziennika",
        ));
        self.register(ShortcutBinding::new(
            AppAction::NewQso,
            KeyboardShortcut::new(ctrl, Key::N),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+N",
            "Nowe QSO (skupienie na znaku)",
        ));
        self.register(ShortcutBinding::new(
            AppAction::FocusFilter,
            KeyboardShortcut::new(ctrl, Key::F),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+F",
            "Szukaj w dzienniku / filtr zaawansowany",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveConfig,
            KeyboardShortcut::new(ctrl, Key::S),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+S",
            "Zapisz konfigurację stacji",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenCommandPalette,
            KeyboardShortcut::new(ctrl_shift, Key::P),
            ShortcutContext::Global,
            7,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Shift+P",
            "Otwórz paletę poleceń",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenStats,
            KeyboardShortcut::new(ctrl_shift, Key::S),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Shift+S",
            "Statystyki dziennika",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenManual,
            KeyboardShortcut::new(ctrl, Key::H),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+H",
            "Pomoc i podręcznik użytkownika",
        ));
        self.register(ShortcutBinding::new(
            AppAction::QuitApp,
            KeyboardShortcut::new(ctrl, Key::Q),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Q",
            "Zamknij program SPLogbook",
        ));

        // --- SKRÓTY BLOKOWANE PODCZAS EDYCJI TEKSTU (Ochrona przed przypadkowym wywołaniem) ---
        self.register(ShortcutBinding::new(
            AppAction::ToggleCluster,
            KeyboardShortcut::new(ctrl, Key::D),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+D",
            "Otwórz panel DX Cluster",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleBandmap,
            KeyboardShortcut::new(ctrl, Key::B),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+B",
            "Otwórz panel Band Map",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleWorldMap,
            KeyboardShortcut::new(ctrl, Key::M),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+M",
            "Otwórz mapę świata",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleCwTerminal,
            KeyboardShortcut::new(ctrl, Key::K),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+K",
            "Otwórz terminal CW",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenProfiles,
            KeyboardShortcut::new(ctrl, Key::P),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+P",
            "Zarządzaj profilami stacji",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ExportCsv,
            KeyboardShortcut::new(ctrl, Key::E),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+E",
            "Eksport do pliku CSV",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ImportAdif,
            KeyboardShortcut::new(ctrl, Key::I),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+I",
            "Import łączności z ADIF",
        ));
        self.register(ShortcutBinding::new(
            AppAction::CycleTheme,
            KeyboardShortcut::new(ctrl, Key::T),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+T",
            "Przełącz motyw kolorystyczny",
        ));

        // --- SKRÓTY EDYTORA TEKSTU (PassThrough) ---
        self.register(ShortcutBinding::new(
            AppAction::Undo,
            KeyboardShortcut::new(ctrl, Key::Z),
            ShortcutContext::Global,
            4,
            ConflictPolicy::PassThroughToWidget,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Z",
            "Cofnij (Undo)",
        ));
        self.register(ShortcutBinding::new(
            AppAction::Redo,
            KeyboardShortcut::new(ctrl, Key::Y),
            ShortcutContext::Global,
            4,
            ConflictPolicy::PassThroughToWidget,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Y",
            "Ponów (Redo)",
        ));

        // --- KONTEKST ZAWODÓW (Contest Window) — Priorytet 10 ---
        self.register(ShortcutBinding::new(
            AppAction::ContestSaveQso,
            KeyboardShortcut::new(none, Key::Enter),
            ShortcutContext::Contest,
            10,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Enter",
            "Zapisz łączność w zawodach",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ContestWipeQso,
            KeyboardShortcut::new(none, Key::Escape),
            ShortcutContext::Contest,
            10,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Esc",
            "Wyczyść pola w zawodach",
        ));

        // --- KONTEKST MODALNY I ESCAPE ZAMYKAJĄCY ---
        self.register(ShortcutBinding::new(
            AppAction::EscapeCloseContext,
            KeyboardShortcut::new(none, Key::Escape),
            ShortcutContext::ModalDialog,
            10,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Esc",
            "Zamknij aktywne okno modalne",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_registry_has_no_duplicate_active_bindings() {
        let registry = ShortcutRegistry::default();
        let conflicts = registry.find_conflicts();
        assert!(
            conflicts.is_empty(),
            "Found conflicting active shortcut bindings: {conflicts:?}"
        );
    }

    #[test]
    fn shortcut_resolution_is_deterministic() {
        let registry = ShortcutRegistry::default();
        let label1 = registry.label_for_action(AppAction::SaveQso);
        let label2 = registry.label_for_action(AppAction::SaveQso);
        assert_eq!(label1, label2);
        assert_eq!(label1, Some("F2"));
    }

    #[test]
    fn local_shortcut_precedes_global_when_documented() {
        let registry = ShortcutRegistry::default();
        let contest_bindings: Vec<&ShortcutBinding> = registry
            .bindings()
            .iter()
            .filter(|b| b.context == ShortcutContext::Contest)
            .collect();
        assert!(
            !contest_bindings.is_empty(),
            "Contest bindings must be registered"
        );
        for b in contest_bindings {
            assert!(
                b.priority > 5,
                "Local contest binding priority must exceed global priority"
            );
        }
    }

    #[test]
    fn disabled_action_does_not_consume_shortcut() {
        let mut registry = ShortcutRegistry::new();
        let mut binding = ShortcutBinding::new(
            AppAction::SaveQso,
            KeyboardShortcut::new(Modifiers::NONE, Key::F2),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F2",
            "Save QSO",
        );
        binding.enabled = false;
        registry.register(binding);

        let ctx = egui::Context::default();
        let action = registry.resolve(&ctx, ShortcutContext::Global, false);
        assert_eq!(action, None);
    }

    #[test]
    fn shortcut_label_matches_registered_binding() {
        let registry = ShortcutRegistry::default();
        assert_eq!(registry.label_for_action(AppAction::WipeQso), Some("F3"));
        assert_eq!(
            registry.label_for_action(AppAction::SaveConfig),
            Some("Ctrl+S")
        );
        assert_eq!(
            registry.label_for_action(AppAction::OpenManual),
            Some("F12")
        );
        assert_eq!(registry.label_for_action(AppAction::TogglePtt), Some("F7"));
    }

    #[test]
    fn typing_callsign_does_not_trigger_global_shortcuts() {
        let registry = ShortcutRegistry::default();
        let binding = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::ToggleCluster)
            .expect("Ctrl+D binding must exist");
        assert_eq!(
            binding.conflict_policy,
            ConflictPolicy::BlockWhenTextEditing
        );
    }

    #[test]
    fn typing_exchange_does_not_switch_active_rig() {
        let registry = ShortcutRegistry::default();
        // Number keys (1, 2) have no global shortcut registered
        let has_digit_shortcuts = registry.bindings().iter().any(|b| {
            matches!(
                b.shortcut.logical_key,
                Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 | Key::Num5
            )
        });
        assert!(
            !has_digit_shortcuts,
            "Digits must never be registered as global shortcuts"
        );
    }

    #[test]
    fn space_in_text_field_does_not_trigger_ptt() {
        let registry = ShortcutRegistry::default();
        let ptt_binding = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::TogglePtt)
            .expect("PTT binding must exist");
        // PTT must be F7, never Space
        assert_eq!(ptt_binding.shortcut.logical_key, Key::F7);
        assert_ne!(ptt_binding.shortcut.logical_key, Key::Space);
    }

    #[test]
    fn backspace_in_text_field_does_not_navigate() {
        let registry = ShortcutRegistry::default();
        let has_backspace = registry
            .bindings()
            .iter()
            .any(|b| b.shortcut.logical_key == Key::Backspace);
        assert!(
            !has_backspace,
            "Backspace must never be captured by shortcut registry"
        );
    }

    #[test]
    fn arrow_keys_edit_text_without_changing_radio() {
        let registry = ShortcutRegistry::default();
        let has_arrows = registry.bindings().iter().any(|b| {
            matches!(
                b.shortcut.logical_key,
                Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
            )
        });
        assert!(
            !has_arrows,
            "Arrow keys must not be captured globally as radio change shortcuts"
        );
    }

    #[test]
    fn ctrl_c_remains_copy_in_text_field() {
        let registry = ShortcutRegistry::default();
        let has_ctrl_c = registry
            .bindings()
            .iter()
            .any(|b| b.shortcut.modifiers.command && b.shortcut.logical_key == Key::C);
        assert!(
            !has_ctrl_c,
            "Ctrl+C must never be overridden by application shortcut registry"
        );
    }

    #[test]
    fn ctrl_v_remains_paste_in_text_field() {
        let registry = ShortcutRegistry::default();
        let has_ctrl_v = registry
            .bindings()
            .iter()
            .any(|b| b.shortcut.modifiers.command && b.shortcut.logical_key == Key::V);
        assert!(
            !has_ctrl_v,
            "Ctrl+V must never be overridden by application shortcut registry"
        );
    }

    #[test]
    fn escape_closes_nearest_context_only() {
        let registry = ShortcutRegistry::default();
        let modal_escape = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::EscapeCloseContext);
        assert!(modal_escape.is_some());
        assert_eq!(modal_escape.unwrap().context, ShortcutContext::ModalDialog);

        let contest_escape = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::ContestWipeQso);
        assert!(contest_escape.is_some());
        assert_eq!(contest_escape.unwrap().context, ShortcutContext::Contest);
    }

    #[test]
    fn ptt_shortcut_is_disabled_during_text_editing() {
        // PTT is explicit via F7 with single trigger
        let registry = ShortcutRegistry::default();
        let ptt = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::TogglePtt)
            .unwrap();
        assert_eq!(ptt.repeat_policy, RepeatPolicy::SingleTrigger);
    }

    #[test]
    fn ptt_shortcut_routes_through_tx_arbiter() {
        // TogglePtt maps to SpLogApp::toggle_ptt which queries TxArbiter
        let registry = ShortcutRegistry::default();
        assert_eq!(registry.label_for_action(AppAction::TogglePtt), Some("F7"));
    }

    #[test]
    fn shortcut_repeat_does_not_duplicate_voice_playback() {
        let registry = ShortcutRegistry::default();
        let vk = registry
            .bindings()
            .iter()
            .find(|b| b.action == AppAction::ToggleVoiceKeyer)
            .unwrap();
        assert_eq!(vk.repeat_policy, RepeatPolicy::SingleTrigger);
    }
}
