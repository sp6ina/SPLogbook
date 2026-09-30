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

# Remove struct fields
content = re.sub(r'\s*pub left_column_width: f32,', '', content)
content = re.sub(r'\s*pub right_column_width: f32,', '', content)
content = re.sub(r'\s*pub dragging_tile: Option<String>,', '', content)
content = re.sub(r'\s*///.*?\n\s*pub tabbed_columns: bool,', '', content, flags=re.DOTALL)
content = re.sub(r'\s*///.*?\n\s*pub active_tab: \[usize; 3\],', '', content, flags=re.DOTALL)

# Remove field assignments in new()
content = re.sub(r'\s*tabbed_columns: app_config\.tabbed_columns,', '', content)
content = re.sub(r'\s*active_tab: \[0; 3\],', '', content)
content = re.sub(r'\s*left_column_width: app_config\.left_column_width,', '', content)
content = re.sub(r'\s*right_column_width: app_config\.right_column_width,', '', content)
content = re.sub(r'\s*dragging_tile: None,', '', content)

# Remove field assignments in save_station_config()
content = re.sub(r'\s*tabbed_columns: self\.tabbed_columns,', '', content)
content = re.sub(r'\s*left_column_width: self\.left_column_width,', '', content)
content = re.sub(r'\s*right_column_width: self\.right_column_width,', '', content)

# Rewrite focus_logbook
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
print("Updated app.rs")
