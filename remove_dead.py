import re

with open('src/gui/app_layout.rs', 'r', encoding='utf-8') as f:
    content = f.read()

funcs_to_remove = [
    'get_tiles_in_column',
    'set_tile_order',
    'move_tile_column',
    'move_tile_order',
    'move_tile_to',
    'normalize_tile_orders',
    'render_tiles_in_column',
    'render_tiles_in_column_tabbed'
]

for func in funcs_to_remove:
    start_idx = content.find(f'pub fn {func}(')
    if start_idx == -1:
        print(f"Not found: {func}")
        continue
    
    # backtrack to include attributes or comments? No, just spaces and pub fn is fine.
    # Actually let's backtrack to the beginning of the line
    line_start = content.rfind('\n', 0, start_idx)
    if line_start != -1:
        start_idx = line_start + 1

    brace_idx = content.find('{', start_idx)
    brace_count = 1
    end_idx = brace_idx + 1
    
    while brace_count > 0 and end_idx < len(content):
        if content[end_idx] == '{':
            brace_count += 1
        elif content[end_idx] == '}':
            brace_count -= 1
        end_idx += 1
        
    content = content[:start_idx] + content[end_idx:]

with open('src/gui/app_layout.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Removed dead functions.")
