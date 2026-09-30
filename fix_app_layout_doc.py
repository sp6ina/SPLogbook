import re

with open('src/gui/app_layout.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = re.sub(r'\s*/// Tryb zakładek: każda kolumna pokazuje pasek zakładek, a pod nim tylko\s*/// jedną aktywną kartę \(nagłówek \+ ciało\)\. Zachowuje akcje odpinania,\s*/// zamykania i przenoszenia między kolumnami\.', '', content, flags=re.DOTALL)

with open('src/gui/app_layout.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Updated app_layout.rs")
