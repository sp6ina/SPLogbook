# -*- coding: utf-8 -*-
with open('src/core/service_db.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('for r in rows.flatten() {', 'for r in rows {\n            let r = match r {\n                Ok(r) => r,\n                Err(e) => {\n                    eprintln!("Ignorowanie bledu w service_db.rs: {}", e);\n                    continue;\n                }\n            };')

content = content.replace('return rows.flatten().collect();', 'let mut res = Vec::new();\n        for r in rows {\n            match r {\n                Ok(r) => res.push(r),\n                Err(e) => eprintln!("Ignorowanie bledu w service_db.rs: {}", e),\n            }\n        }\n        return res;')

with open('src/core/service_db.rs', 'w', encoding='utf-8') as f:
    f.write(content)
