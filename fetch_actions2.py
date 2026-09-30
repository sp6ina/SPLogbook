import urllib.request
import json
import zipfile
import io

url = "https://api.github.com/repos/sp6ina/SPLogbook/actions/runs"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    with urllib.request.urlopen(req) as response:
        data = json.loads(response.read().decode())
        runs = data.get('workflow_runs', [])
        for run in runs[:5]:
            print(f"Run ID: {run['id']}, Name: {run['name']}, Status: {run['status']}, Conclusion: {run['conclusion']}")
            if run['conclusion'] == 'failure':
                jobs_url = run['jobs_url']
                req2 = urllib.request.Request(jobs_url, headers={'User-Agent': 'Mozilla/5.0'})
                with urllib.request.urlopen(req2) as resp2:
                    jobs_data = json.loads(resp2.read().decode())
                    for job in jobs_data.get('jobs', []):
                        if job['conclusion'] == 'failure':
                            print(f"  Failed Job: {job['name']}")
                            for step in job['steps']:
                                if step['conclusion'] == 'failure':
                                    print(f"    Failed Step: {step['name']}")
except Exception as e:
    print(e)
