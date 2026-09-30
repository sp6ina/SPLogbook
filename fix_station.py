import re

with open('src/core/station.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = re.sub(r'\s*pub tabbed_columns: bool,', '', content)
content = re.sub(r'\s*pub left_column_width: f32,', '', content)
content = re.sub(r'\s*pub right_column_width: f32,', '', content)
content = re.sub(r'\s*tabbed_columns: false,', '', content)
content = re.sub(r'\s*left_column_width: 350\.0,', '', content)
content = re.sub(r'\s*right_column_width: 360\.0,', '', content)

with open('src/core/station.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated station.rs")
