import re

# Fix app_layout.rs docs
with open('src/gui/app_layout.rs', 'r', encoding='utf-8') as f:
    content = f.read()
content = re.sub(r'\s*/// Tryb zakładek: każda kolumna pokazuje pasek zakładek, a pod nim tylko\s*/// jedną aktywną kartę \(nagłówek \+ ciało\)\. Zachowuje akcje odpinania,\s*/// zamykania i przenoszenia między kolumnami\.', '', content, flags=re.DOTALL)
with open('src/gui/app_layout.rs', 'w', encoding='utf-8') as f:
    f.write(content)

# Fix focus_logbook in app.rs
with open('src/gui/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_focus = '''    pub fn focus_logbook(&mut self) {
        if self.panel_log.floating {
            self.panel_log.visible = true;
            return;
        }
        self.panel_log.visible = true;
        let col = self.panel_log.column;
        let tiles = self.get_tiles_in_column(col);
        if let Some(idx) = tiles.iter().position(|t| t == "log") {
            self.active_tab[col] = idx;
        }
    }'''
new_focus = '''    pub fn focus_logbook(&mut self) {
        self.panel_log.visible = true;
    }'''
content = content.replace(old_focus, new_focus)

with open('src/gui/app.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed issues.")
