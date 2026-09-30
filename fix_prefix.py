# -*- coding: utf-8 -*-
with open('src/core/prefix.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('for r in rows.flatten() {', 'for r in rows {\n            let r = match r {\n                Ok(r) => r,\n                Err(e) => {\n                    eprintln!("Ignorowanie bledu w prefix.rs: {}", e);\n                    continue;\n                }\n            };')
content = content.replace('for (mut info, raw_regex) in rows.flatten() {', 'for row_res in rows {\n            let (mut info, raw_regex) = match row_res {\n                Ok(val) => val,\n                Err(e) => {\n                    eprintln!("Ignorowanie bledu w prefix.rs: {}", e);\n                    continue;\n                }\n            };')

with open('src/core/prefix.rs', 'w', encoding='utf-8') as f:
    f.write(content)
