import re

def remove_functions(filepath, funcs):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    for func in funcs:
        start_idx = content.find(f'pub fn {func}(')
        if start_idx == -1:
            print(f"Not found: {func} in {filepath}")
            continue
        
        # Backtrack to the beginning of the line
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

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

remove_functions('src/gui/app_layout.rs', [
    'get_tiles_in_column',
    'set_tile_order',
    'move_tile_column',
    'move_tile_order',
    'move_tile_to',
    'normalize_tile_orders',
    'render_tiles_in_column',
    'render_tiles_in_column_tabbed'
])

remove_functions('src/gui/app.rs', [
    'render_column1',
    'render_column2',
    'render_column3'
])

print("Removed dead functions safely.")
