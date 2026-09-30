import re

with open('src/gui/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

funcs_to_remove = [
    'render_column1',
    'render_column2',
    'render_column3'
]

for func in funcs_to_remove:
    start_idx = content.find(f'pub fn {func}(')
    if start_idx != -1:
        line_start = content.rfind('\n', 0, start_idx)
        if line_start != -1: start_idx = line_start + 1
        brace_idx = content.find('{', start_idx)
        brace_count = 1
        end_idx = brace_idx + 1
        while brace_count > 0 and end_idx < len(content):
            if content[end_idx] == '{': brace_count += 1
            elif content[end_idx] == '}': brace_count -= 1
            end_idx += 1
        content = content[:start_idx] + content[end_idx:]

lines = content.split('\n')
new_lines = []
skip = False
for line in lines:
    if 'pub left_column_width: f32,' in line: continue
    if 'pub right_column_width: f32,' in line: continue
    if 'pub dragging_tile: Option<String>,' in line: continue
    if 'pub tabbed_columns: bool,' in line: continue
    if 'pub active_tab: [usize; 3],' in line: continue
    if '/// Tryb zakładek w kolumnach dokowanego układu (zapis w konfiguracji).' in line: continue
    if '/// Indeks aktywnej zakładki w każdej z trzech kolumn (stan sesji).' in line: continue
    
    if 'tabbed_columns: app_config.tabbed_columns,' in line: continue
    if 'active_tab: [0; 3],' in line: continue
    if 'left_column_width: app_config.left_column_width,' in line: continue
    if 'right_column_width: app_config.right_column_width,' in line: continue
    if 'dragging_tile: None,' in line: continue

    if 'tabbed_columns: self.tabbed_columns,' in line: continue
    if 'left_column_width: self.left_column_width,' in line: continue
    if 'right_column_width: self.right_column_width,' in line: continue

    new_lines.append(line)

content = '\n'.join(new_lines)

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
print("Updated app.rs safely")
