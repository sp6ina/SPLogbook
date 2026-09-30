import urllib.request
import zipfile
import io
import os

url = "https://api.github.com/repos/sp6ina/SPLogbook/actions/runs/36606699518/logs"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    with urllib.request.urlopen(req) as response:
        with zipfile.ZipFile(io.BytesIO(response.read())) as z:
            # list files
            for info in z.infolist():
                if "Build Linux x86_64" in info.filename and "Compile release binary" in info.filename:
                    print(f"Log: {info.filename}")
                    content = z.read(info.filename).decode()
                    # print last 100 lines
                    lines = content.split('\n')
                    print('\n'.join(lines[-100:]))
except Exception as e:
    print(e)
