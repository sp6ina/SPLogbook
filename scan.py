import os, re
dir_path = r'c:\Users\sp6in\Documents\SPLogbook\SPLogbook\src\gui'
res = []
error_handling_regex = re.compile(r'(unwrap\(\)|expect\(|panic!|todo!|dbg!)')
for root, _, files in os.walk(dir_path):
  for f in files:
    if f.endswith('.rs'):
      path = os.path.join(root, f)
      with open(path, 'r', encoding='utf-8') as file:
        lines = file.readlines()
        calls = []
        for i, line in enumerate(lines):
          if error_handling_regex.search(line):
            calls.append(f'Line {i+1}: {line.strip()}')
        res.append({'path': path.replace(dir_path, '').strip('\\\\/'), 'lines': len(lines), 'calls': calls})
with open('gui_report.txt', 'w', encoding='utf-8') as out:
  for item in res:
    out.write(f'--- {item["path"]} ({item["lines"]} lines) ---\n')
    if item['calls']:
      out.write('Error handling calls:\n')
      for c in item['calls']:
        out.write(f'  {c}\n')
    out.write('\n')
