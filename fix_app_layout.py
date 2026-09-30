import re

with open('src/gui/app_layout.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = re.sub(r'\s*self\.left_column_width = 350\.0;', '', content)
content = re.sub(r'\s*self\.right_column_width = 360\.0;', '', content)

with open('src/gui/app_layout.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated app_layout.rs")
