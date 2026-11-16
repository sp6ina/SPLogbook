// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny rejestr skrótów klawiaturowych, zarządzanie kontekstem i ochrona edycji tekstu (GUI-001).

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};

/// Pomocnik sprawdzający czy wciśnięto klawisz modyfikatora Ctrl (Windows/Linux) lub Command (macOS).
#[inline]
pub fn is_ctrl_modifier(modifiers: &egui::Modifiers) -> bool {
    modifiers.ctrl || modifiers.command
}

/// Błąd walidacji rejestru skrótów klawiaturowych.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutConfigError {
    ConflictingBindings {
        action_a: AppAction,
        action_b: AppAction,
        shortcut: &'static str,
    },
}

impl std::fmt::Display for ShortcutConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictingBindings {
                action_a,
                action_b,
                shortcut,
            } => write!(
                f,
                "shortcut binding conflict {shortcut}: {action_a:?} vs {action_b:?}"
            ),
        }
    }
}

impl std::error::Error for ShortcutConfigError {}

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
    /// Jawna intencja nadawania (PTT ON) — wyłącznie przez centralny dispatcher PTT (F-17).
    PttOn,
    /// Jawna intencja zakończenia nadawania (PTT OFF) — wyłącznie przez centralny dispatcher (F-17).
    PttOff,
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

/// Jawne konteksty fokusu interfejsu użytkownika (GUI-001).
///
/// Gwarantuje, że skróty z jawnym kontraktem (np. Ctrl+W czyszczenie łączności)
/// działają wyłącznie w kontekście formularza QSO (`MainQsoEntry`, `ContestQsoEntry`),
/// a nie przenikają do innych kontrolek edycyjnych (filtry, ustawienia, asystent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActiveFocusContext {
    ModalDialog,
    ContestQsoEntry,
    MainQsoEntry,
    LogFilter,
    SettingsTextEdit,
    OperatorAssistantTextEdit,
    LogTable,
    Global,
}

impl ActiveFocusContext {
    /// Czy kontekst pozwala na wykonanie akcji czyszczenia formularza QSO (Wipe).
    #[must_use]
    pub const fn allows_qso_wipe(&self) -> bool {
        matches!(self, Self::MainQsoEntry | Self::ContestQsoEntry)
    }

    /// Konwertuje kontekst fokusu na odpowiadający kontekst skrótów klawiszowych.
    #[must_use]
    pub const fn to_shortcut_context(&self) -> ShortcutContext {
        match self {
            Self::ModalDialog => ShortcutContext::ModalDialog,
            Self::ContestQsoEntry => ShortcutContext::ContestQsoEntry,
            Self::MainQsoEntry => ShortcutContext::MainQsoEntry,
            Self::LogFilter => ShortcutContext::LogFilter,
            Self::SettingsTextEdit => ShortcutContext::SettingsTextEdit,
            Self::OperatorAssistantTextEdit => ShortcutContext::OperatorAssistantTextEdit,
            Self::LogTable => ShortcutContext::LogTable,
            Self::Global => ShortcutContext::Global,
        }
    }
}

/// Kontekst wykonania skrótu klawiaturowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutContext {
    /// Globalny kontekst aplikacji (domyślny).
    Global,
    /// Modalne okno dialogowe lub popup.
    ModalDialog,
    /// Edycja łączności w oknie zawodów.
    ContestQsoEntry,
    /// Edycja w głównym formularzu łączności (QSO Entry).
    MainQsoEntry,
    /// Pole tekstowe filtra dziennika.
    LogFilter,
    /// Edycja tekstu w oknach ustawień i konfiguracji.
    SettingsTextEdit,
    /// Edycja tekstu w terminalu CW lub notatkach asystenta operatora.
    OperatorAssistantTextEdit,
    /// Tabela łączności dziennika.
    LogTable,
    /// Alias wsteczny dla formularza łączności.
    QsoEntry,
    /// Alias wsteczny dla okna zawodów.
    Contest,
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

/// Polityka dopasowania modyfikatorów (F-17).
///
/// Domyślna semantyka egui (`matches_logically`) **ignoruje** dodatkowe `Shift` i `Alt`, więc
/// kombinacja `Ctrl+F7` pasuje również do `Ctrl+Shift+F7` i `Ctrl+Alt+F7`. Dla skrótów
/// wysokiego ryzyka wymagane jest dopasowanie dokładne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierPolicy {
    /// Semantyka egui 0.36.2: dodatkowe `Shift`/`Alt` są ignorowane.
    Logical,
    /// Wymagane dokładnie te modyfikatory (`Modifiers::matches_exact`) i `repeat == false`.
    ExactRequired,
}

/// Polityka wykonywania skrótu przy otwartym oknie modalnym (F-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalPolicy {
    /// Skrót działa również przy otwartym oknie modalnym (dotychczasowe zachowanie).
    Allow,
    /// Skrót jest bezwzględnie blokowany, gdy aktywne jest okno modalne.
    BlockWhenAnyModal,
}

/// Bramki wejściowe wymagane przez skróty wysokiego ryzyka (F-17).
///
/// Wszystkie pola są wyliczane raz na klatkę z wiarygodnego API egui 0.36.2
/// (`Context::egui_wants_keyboard_input`, `Context::text_edit_focused`, `InputState::focused`)
/// oraz z deterministycznie wykrytego stanu IME i modali.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortcutGates {
    /// Aktywne pole tekstowe albo framework żąda wejścia klawiatury.
    pub text_editing: bool,
    /// Trwa kompozycja IME (lub w tej klatce wystąpiło jakiekolwiek zdarzenie IME).
    pub ime_composing: bool,
    /// Okno aplikacji ma fokus systemowy.
    pub window_focused: bool,
    /// Otwarte jest okno modalne blokujące akcje globalne.
    pub modal_open: bool,
}

impl ShortcutGates {
    /// Wszystkie bramki zamknięte — używane wyłącznie wtedy, gdy nie da się ich odczytać.
    #[must_use]
    pub const fn fail_closed() -> Self {
        Self {
            text_editing: true,
            ime_composing: true,
            window_focused: false,
            modal_open: true,
        }
    }
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
    pub modifier_policy: ModifierPolicy,
    pub modal_policy: ModalPolicy,
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
            modifier_policy: ModifierPolicy::Logical,
            modal_policy: ModalPolicy::Allow,
            enabled: true,
            label,
            description,
        }
    }

    /// Wymaga dokładnego dopasowania modyfikatorów i odrzuca powtórzenia klawisza (F-17).
    #[must_use]
    pub const fn with_exact_modifiers(mut self) -> Self {
        self.modifier_policy = ModifierPolicy::ExactRequired;
        self
    }

    /// Blokuje skrót przy otwartym oknie modalnym (F-17).
    #[must_use]
    pub const fn blocked_when_modal(mut self) -> Self {
        self.modal_policy = ModalPolicy::BlockWhenAnyModal;
        self
    }
}

/// Wykrywa pojedyncze, niepowtórzone naciśnięcie o **dokładnie** podanych modyfikatorach
/// i konsumuje je (F-17).
///
/// Semantyka jest ostrożniejsza od [`egui::InputState::consume_shortcut`]:
/// * `Matches` wyłącznie `Event::Key` z `pressed == true` (release nigdy nie wyzwala akcji),
/// * odrzuca `repeat == true` (autorepeat nie generuje nowych intencji),
/// * wymaga `Modifiers::matches_exact`, więc dodatkowy `Shift`/`Alt`/`Super` nie pasuje,
/// * usuwa dopasowane zdarzenie z buforu klatki, więc to samo zdarzenie nie może zostać
///   obsłużone drugi raz.
fn consume_exact_press(state: &mut egui::InputState, shortcut: KeyboardShortcut) -> bool {
    let expected_modifiers = shortcut.modifiers;
    let expected_key = shortcut.logical_key;
    let mut consumed = false;
    state.events.retain(|event| {
        let is_match = matches!(
            event,
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } if *key == expected_key && modifiers.matches_exact(expected_modifiers)
        );
        consumed |= is_match;
        !is_match
    });
    consumed
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

    /// Weryfikuje czy w rejestrze skrótów nie występują konflikty.
    pub fn validate(&self) -> Result<(), ShortcutConfigError> {
        let conflicts = self.find_conflicts();
        if let Some((a, b)) = conflicts.first() {
            return Err(ShortcutConfigError::ConflictingBindings {
                action_a: a.action,
                action_b: b.action,
                shortcut: a.label,
            });
        }
        Ok(())
    }

    /// Deterministycznie rozwiązuje akcję klawiaturową na podstawie wejścia egui, aktywnego kontekstu
    /// oraz stanu bramek wejściowych (fokus tekstowy, IME, fokus okna, modal).
    ///
    /// Jeśli znaleziono pasującą regułę, skrót jest konsumowany z bufora egui (aby uniknąć podwójnego
    /// wykonania w innych widokach) i zwracana jest skojarzona [`AppAction`].
    ///
    /// Skróty wysokiego ryzyka (F-17, PTT) wymagają **wszystkich** bramek otwartych, dokładnego
    /// dopasowania modyfikatorów i niepowtórzonego naciśnięcia. Bramka, której nie da się odczytać,
    /// jest traktowana jako zamknięta ([`ShortcutGates::fail_closed`]).
    pub fn resolve(
        &self,
        ctx: &egui::Context,
        current_context: ShortcutContext,
        gates: &ShortcutGates,
    ) -> Option<AppAction> {
        let text_focused = gates.text_editing;
        // Znajdź wszystkich kandydatów pasujących do bieżącego kontekstu lub kontekstu globalnego.
        let mut candidates: Vec<&ShortcutBinding> = self
            .bindings
            .iter()
            .filter(|b| b.enabled)
            .filter(|b| {
                b.context == current_context
                    || b.context == ShortcutContext::Global
                    || (b.context == ShortcutContext::QsoEntry
                        && current_context == ShortcutContext::MainQsoEntry)
                    || (b.context == ShortcutContext::Contest
                        && current_context == ShortcutContext::ContestQsoEntry)
                    || (b.context == ShortcutContext::MainQsoEntry
                        && current_context == ShortcutContext::QsoEntry)
                    || (b.context == ShortcutContext::ContestQsoEntry
                        && current_context == ShortcutContext::Contest)
            })
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

            // Ochrona okna modalnego (F-17): skrót wysokiego ryzyka nie działa,
            // gdy otwarte jest okno blokujące akcje globalne.
            if gates.modal_open && binding.modal_policy == ModalPolicy::BlockWhenAnyModal {
                continue;
            }

            // Ochrona fokusu okna i kompozycji IME (F-17). Wymagana wyłącznie przez skróty,
            // które jawnie przyjmują pełny zestaw bramek.
            let requires_all_gates = binding.modifier_policy == ModifierPolicy::ExactRequired;
            if requires_all_gates && (!gates.window_focused || gates.ime_composing) {
                continue;
            }

            // Sprawdzenie czy wciśnięto kombinację klawiszy z uwzględnieniem SingleTrigger
            let triggered = ctx.input_mut(|i| match binding.modifier_policy {
                ModifierPolicy::ExactRequired => consume_exact_press(i, binding.shortcut),
                ModifierPolicy::Logical => {
                    if binding.repeat_policy == RepeatPolicy::SingleTrigger {
                        let is_repeat = i.events.iter().any(|e| match e {
                            egui::Event::Key { key, repeat, .. } => {
                                *key == binding.shortcut.logical_key && *repeat
                            }
                            _ => false,
                        });
                        if is_repeat {
                            return false;
                        }
                    }
                    i.consume_shortcut(&binding.shortcut)
                }
            });
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
            "Keyboard shortcuts help and list",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveQso,
            KeyboardShortcut::new(none, Key::F2),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F2",
            "Save QSO to logbook",
        ));
        self.register(ShortcutBinding::new(
            AppAction::WipeQso,
            KeyboardShortcut::new(none, Key::F3),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F3",
            "Wipe QSO form",
        ));
        self.register(ShortcutBinding::new(
            AppAction::LookupQrz,
            KeyboardShortcut::new(none, Key::F4),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F4",
            "Lookup callsign in QRZ.com / HamQTH",
        ));
        self.register(ShortcutBinding::new(
            AppAction::RefreshLog,
            KeyboardShortcut::new(none, Key::F5),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F5",
            "Refresh logbook table",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SendSpot,
            KeyboardShortcut::new(none, Key::F6),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F6",
            "Send DX spot for current frequency",
        ));
        // --- PTT (F-17): DWIEROZŁĄCZNE, JAWNE INTENCJE Z MODYFIKATOREM ---
        //
        // Pojedynczy klawisz nigdy nie uruchamia PTT. `F7` pozostaje klawiszem mnemonicznym,
        // ale wymaga jawnego modyfikatora, dokładnego dopasowania i wszystkich bramek wejściowych:
        // brak fokusu edycji tekstu, brak aktywnej kompozycji IME, fokus okna, brak modala.
        // `Ctrl+Shift+F7` (TX ON) jest celowo trudniejszy od `Ctrl+F7` (TX OFF): zatrzymanie
        // nadawania nie może być trudniejsze niż jego rozpoczęcie.
        self.register(
            ShortcutBinding::new(
                AppAction::PttOn,
                KeyboardShortcut::new(ctrl_shift, Key::F7),
                ShortcutContext::Global,
                9,
                ConflictPolicy::BlockWhenTextEditing,
                RepeatPolicy::SingleTrigger,
                "Ctrl+Shift+F7",
                "Request PTT ON (transmit) via CAT — explicit TX-intent gate",
            )
            .with_exact_modifiers()
            .blocked_when_modal(),
        );
        self.register(
            ShortcutBinding::new(
                AppAction::PttOff,
                KeyboardShortcut::new(ctrl, Key::F7),
                ShortcutContext::Global,
                8,
                ConflictPolicy::BlockWhenTextEditing,
                RepeatPolicy::SingleTrigger,
                "Ctrl+F7",
                "Request PTT OFF (receive) via CAT",
            )
            .with_exact_modifiers()
            .blocked_when_modal(),
        );
        self.register(ShortcutBinding::new(
            AppAction::ToggleVoiceKeyer,
            KeyboardShortcut::new(none, Key::F8),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F8",
            "Open SSB Voice Keyer window",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleFullscreen,
            KeyboardShortcut::new(none, Key::F11),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F11",
            "Full screen",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenManual,
            KeyboardShortcut::new(none, Key::F12),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "F12",
            "Open user manual",
        ));

        // --- SKRÓTY Z KLAWISZEM CTRL DOSTĘPNE PODCZAS EDYCJI TEKSTU (Jawny kontrakt) ---
        self.register(ShortcutBinding::new(
            AppAction::WipeQso,
            KeyboardShortcut::new(ctrl, Key::W),
            ShortcutContext::MainQsoEntry,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+W",
            "Wipe QSO entry form",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ContestWipeQso,
            KeyboardShortcut::new(ctrl, Key::W),
            ShortcutContext::ContestQsoEntry,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+W",
            "Wipe contest QSO entry form",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveQso,
            KeyboardShortcut::new(ctrl, Key::L),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+L",
            "Save current QSO to logbook",
        ));
        self.register(ShortcutBinding::new(
            AppAction::NewQso,
            KeyboardShortcut::new(ctrl, Key::N),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+N",
            "New QSO (focus callsign)",
        ));
        self.register(ShortcutBinding::new(
            AppAction::FocusFilter,
            KeyboardShortcut::new(ctrl, Key::F),
            ShortcutContext::Global,
            6,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+F",
            "Search in logbook / advanced filter",
        ));
        self.register(ShortcutBinding::new(
            AppAction::SaveConfig,
            KeyboardShortcut::new(ctrl, Key::S),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+S",
            "Save station configuration",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenCommandPalette,
            KeyboardShortcut::new(ctrl_shift, Key::P),
            ShortcutContext::Global,
            7,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Shift+P",
            "Open command palette",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenStats,
            KeyboardShortcut::new(ctrl_shift, Key::S),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Shift+S",
            "Logbook statistics",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenManual,
            KeyboardShortcut::new(ctrl, Key::H),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+H",
            "Help and user manual",
        ));
        self.register(ShortcutBinding::new(
            AppAction::QuitApp,
            KeyboardShortcut::new(ctrl, Key::Q),
            ShortcutContext::Global,
            5,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Q",
            "Exit SPLogbook",
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
            "Open DX Cluster panel",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleBandmap,
            KeyboardShortcut::new(ctrl, Key::B),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+B",
            "Open Band Map panel",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleWorldMap,
            KeyboardShortcut::new(ctrl, Key::M),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+M",
            "Open world map",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ToggleCwTerminal,
            KeyboardShortcut::new(ctrl, Key::K),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+K",
            "Open CW terminal",
        ));
        self.register(ShortcutBinding::new(
            AppAction::OpenProfiles,
            KeyboardShortcut::new(ctrl, Key::P),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+P",
            "Manage station profiles",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ExportCsv,
            KeyboardShortcut::new(ctrl, Key::E),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+E",
            "Export to CSV file",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ImportAdif,
            KeyboardShortcut::new(ctrl, Key::I),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+I",
            "Import QSOs from ADIF",
        ));
        self.register(ShortcutBinding::new(
            AppAction::CycleTheme,
            KeyboardShortcut::new(ctrl, Key::T),
            ShortcutContext::Global,
            3,
            ConflictPolicy::BlockWhenTextEditing,
            RepeatPolicy::SingleTrigger,
            "Ctrl+T",
            "Toggle color theme",
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
            "Undo",
        ));
        self.register(ShortcutBinding::new(
            AppAction::Redo,
            KeyboardShortcut::new(ctrl, Key::Y),
            ShortcutContext::Global,
            4,
            ConflictPolicy::PassThroughToWidget,
            RepeatPolicy::SingleTrigger,
            "Ctrl+Y",
            "Redo",
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
            "Save contest QSO",
        ));
        self.register(ShortcutBinding::new(
            AppAction::ContestWipeQso,
            KeyboardShortcut::new(none, Key::Escape),
            ShortcutContext::Contest,
            10,
            ConflictPolicy::AllowInTextEditing,
            RepeatPolicy::SingleTrigger,
            "Esc",
            "Wipe contest fields",
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
            "Close active modal dialog",
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
        let gates = ShortcutGates {
            window_focused: true,
            ..ShortcutGates::fail_closed()
        };
        let action = registry.resolve(&ctx, ShortcutContext::Global, &gates);
        assert_eq!(action, None);
    }

    /// Bramki w pełni otwarte, bez modala, bez edycji tekstu i bez IME, z fokusem okna.
    fn open_gates() -> ShortcutGates {
        ShortcutGates {
            text_editing: false,
            ime_composing: false,
            window_focused: true,
            modal_open: false,
        }
    }

    /// Buduje wejście egui z jednym zdarzeniem klawiatury.
    ///
    /// Uwaga: egui 0.36.2 sam wylicza flagę `repeat` z `keys_down`
    /// (`egui-0.36.2/src/input_state/mod.rs:400-414`), więc podana wartość jest tylko
    /// wartością wejściową — powtórzenie klawisza odtwarza się dwiema klatkami.
    fn raw_with_key(key: Key, modifiers: Modifiers, pressed: bool, repeat: bool) -> egui::RawInput {
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers,
        });
        raw
    }

    /// Buduje kontekst egui z jedną rozpoczętą klatką i jednym zdarzeniem klawiatury.
    fn ctx_with_key(key: Key, modifiers: Modifiers, pressed: bool, repeat: bool) -> egui::Context {
        let ctx = egui::Context::default();
        ctx.begin_pass(raw_with_key(key, modifiers, pressed, repeat));
        ctx
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
        assert_eq!(
            registry.label_for_action(AppAction::PttOn),
            Some("Ctrl+Shift+F7")
        );
        assert_eq!(
            registry.label_for_action(AppAction::PttOff),
            Some("Ctrl+F7")
        );
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
            .find(|b| b.action == AppAction::PttOn)
            .expect("PTT binding must exist");
        // PTT uses a modified F7, never Space
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

        let contest_escape = registry.bindings().iter().find(|b| {
            b.action == AppAction::ContestWipeQso && b.shortcut.logical_key == Key::Escape
        });
        assert!(contest_escape.is_some());
        assert_eq!(contest_escape.unwrap().context, ShortcutContext::Contest);
    }

    // ======================================================================================
    // F-17 — PTT interaction, keyboard gating and release control
    // ======================================================================================

    /// Zwraca powiązanie PTT dla wskazanej akcji.
    fn ptt_binding(registry: &ShortcutRegistry, action: AppAction) -> &ShortcutBinding {
        registry
            .bindings()
            .iter()
            .find(|b| b.action == action)
            .expect("PTT binding must exist")
    }

    /// Wszystkie akcje PTT (dokładnie dwie: ON i OFF).
    fn ptt_actions() -> [AppAction; 2] {
        [AppAction::PttOn, AppAction::PttOff]
    }

    #[test]
    fn f17_ptt_is_never_a_single_key() {
        let registry = ShortcutRegistry::default();
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);

        let on = ptt_binding(&registry, AppAction::PttOn);
        assert_eq!(
            on.shortcut.modifiers, ctrl_shift,
            "PTT ON must require Ctrl+Shift — a single key is forbidden"
        );

        let off = ptt_binding(&registry, AppAction::PttOff);
        assert_eq!(
            off.shortcut.modifiers, ctrl,
            "PTT OFF must require Ctrl — a single key is forbidden"
        );

        for action in ptt_actions() {
            let modifiers = ptt_binding(&registry, action).shortcut.modifiers;
            assert!(
                modifiers != Modifiers::NONE,
                "{action:?} must never be bound without a modifier"
            );
            assert!(
                !modifiers.ctrl || modifiers.command,
                "{action:?} must use the portable COMMAND modifier"
            );
        }
    }

    #[test]
    fn f17_no_single_key_binding_triggers_any_ptt_action() {
        let registry = ShortcutRegistry::default();
        for binding in registry.bindings() {
            if ptt_actions().contains(&binding.action) {
                continue;
            }
            assert!(
                binding.shortcut.logical_key != Key::F7,
                "no binding other than the two explicit PTT intents may use F7: {binding:?}"
            );
        }
    }

    #[test]
    fn f17_ptt_bindings_require_text_focus_gating() {
        let registry = ShortcutRegistry::default();
        for action in ptt_actions() {
            let binding = ptt_binding(&registry, action);
            assert_eq!(
                binding.conflict_policy,
                ConflictPolicy::BlockWhenTextEditing,
                "{action:?} must be blocked while a text field has focus"
            );
        }
    }

    #[test]
    fn f17_ptt_bindings_require_modal_gating() {
        let registry = ShortcutRegistry::default();
        for action in ptt_actions() {
            let binding = ptt_binding(&registry, action);
            assert_eq!(
                binding.modal_policy,
                ModalPolicy::BlockWhenAnyModal,
                "{action:?} must be blocked while a modal window is open"
            );
        }
    }

    #[test]
    fn f17_ptt_bindings_require_exact_modifiers_and_single_trigger() {
        let registry = ShortcutRegistry::default();
        for action in ptt_actions() {
            let binding = ptt_binding(&registry, action);
            assert_eq!(
                binding.modifier_policy,
                ModifierPolicy::ExactRequired,
                "{action:?} must reject extra modifiers"
            );
            assert_eq!(
                binding.repeat_policy,
                RepeatPolicy::SingleTrigger,
                "{action:?} must ignore key repeat"
            );
        }
    }

    #[test]
    fn f17_bare_f7_does_not_resolve_to_ptt() {
        let registry = ShortcutRegistry::default();
        let ctx = ctx_with_key(Key::F7, Modifiers::NONE, true, false);
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            None,
            "a bare F7 press must never produce a PTT intent"
        );
    }

    #[test]
    fn f17_modified_f7_resolves_to_the_matching_direction() {
        let registry = ShortcutRegistry::default();
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);

        let ctx_on = ctx_with_key(Key::F7, ctrl_shift, true, false);
        assert_eq!(
            registry.resolve(&ctx_on, ShortcutContext::Global, &open_gates()),
            Some(AppAction::PttOn)
        );

        let ctx_off = ctx_with_key(Key::F7, ctrl, true, false);
        assert_eq!(
            registry.resolve(&ctx_off, ShortcutContext::Global, &open_gates()),
            Some(AppAction::PttOff)
        );
    }

    #[test]
    fn f17_unexpected_extra_modifier_is_an_explicit_mismatch() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctrl_alt = Modifiers::COMMAND.plus(Modifiers::ALT);
        let ctrl_shift_alt = ctrl_shift.plus(Modifiers::ALT);
        let shift_only = Modifiers::SHIFT;

        for modifiers in [ctrl_alt, ctrl_shift_alt, shift_only] {
            let ctx = ctx_with_key(Key::F7, modifiers, true, false);
            assert_eq!(
                registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
                None,
                "extra or insufficient modifiers must be an explicit mismatch: {modifiers:?}"
            );
        }
    }

    #[test]
    fn f17_ptt_is_blocked_while_text_editing() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        let gates = ShortcutGates {
            text_editing: true,
            ..open_gates()
        };
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &gates),
            None
        );
    }

    #[test]
    fn f17_ptt_is_blocked_while_ime_is_composing() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        let gates = ShortcutGates {
            ime_composing: true,
            ..open_gates()
        };
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &gates),
            None
        );
    }

    #[test]
    fn f17_ptt_is_blocked_when_window_is_unfocused() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        let gates = ShortcutGates {
            window_focused: false,
            ..open_gates()
        };
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &gates),
            None
        );
    }

    #[test]
    fn f17_ptt_is_blocked_when_a_modal_is_open() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        let gates = ShortcutGates {
            modal_open: true,
            ..open_gates()
        };
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::ModalDialog, &gates),
            None
        );
    }

    #[test]
    fn f17_ptt_repeat_does_not_retrigger() {
        // Realistyczne powtórzenie: pierwsza klatka rejestruje naciśnięcie (egui dodaje klawisz
        // do `keys_down`), druga klatka jest przez egui oznaczana jako `repeat = true`.
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = egui::Context::default();

        ctx.begin_pass(raw_with_key(Key::F7, ctrl_shift, true, false));
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            Some(AppAction::PttOn)
        );

        ctx.begin_pass(raw_with_key(Key::F7, ctrl_shift, true, false));
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            None,
            "a key-repeat event must never produce a second PTT intent"
        );
    }

    #[test]
    fn f17_ptt_release_does_not_trigger_on() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, false, false);
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            None,
            "a release event must never produce a PTT intent"
        );
    }

    #[test]
    fn f17_ptt_shortcut_is_consumed_exactly_once() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        let first = registry.resolve(&ctx, ShortcutContext::Global, &open_gates());
        let second = registry.resolve(&ctx, ShortcutContext::Global, &open_gates());
        assert_eq!(first, Some(AppAction::PttOn));
        assert_eq!(second, None, "the same event must never be consumed twice");
    }

    #[test]
    fn f17_several_f7_events_in_one_frame_produce_one_intent() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        for _ in 0..5 {
            raw.events.push(egui::Event::Key {
                key: Key::F7,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: ctrl_shift,
            });
        }
        ctx.begin_pass(raw);

        let mut intents = 0;
        for _ in 0..5 {
            if registry.resolve(&ctx, ShortcutContext::Global, &open_gates())
                == Some(AppAction::PttOn)
            {
                intents += 1;
            }
        }
        assert_eq!(
            intents, 1,
            "a frame with several F7 presses yields one intent"
        );
    }

    #[test]
    fn f17_pressed_and_released_in_one_frame_produce_one_intent() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key {
            key: Key::F7,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: ctrl_shift,
        });
        raw.events.push(egui::Event::Key {
            key: Key::F7,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: ctrl_shift,
        });
        ctx.begin_pass(raw);

        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            Some(AppAction::PttOn)
        );
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &open_gates()),
            None
        );
    }

    #[test]
    fn f17_fail_closed_gates_block_ptt_entirely() {
        let registry = ShortcutRegistry::default();
        let ctrl_shift = Modifiers::COMMAND.plus(Modifiers::SHIFT);
        let ctx = ctx_with_key(Key::F7, ctrl_shift, true, false);
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &ShortcutGates::fail_closed()),
            None,
            "unreadable gates must fail closed"
        );
    }

    #[test]
    fn f17_ptt_does_not_break_unrelated_global_shortcuts() {
        // Regresja: skróty bez PTT zachowują dotychczasową semantykę (AllowInTextEditing działa).
        let registry = ShortcutRegistry::default();
        let ctx = ctx_with_key(Key::F2, Modifiers::NONE, true, false);
        let gates = ShortcutGates {
            text_editing: true,
            ..open_gates()
        };
        assert_eq!(
            registry.resolve(&ctx, ShortcutContext::Global, &gates),
            Some(AppAction::SaveQso)
        );
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
