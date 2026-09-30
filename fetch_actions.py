import urllib.request
import json
import zipfile
import io

# Get latest workflow runs
url = "https://api.github.com/repos/sp6ina/SPLogbook/actions/runs"
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
try:
    with urllib.request.urlopen(req) as response:
        data = json.loads(response.read().decode())
        runs = data.get('workflow_runs', [])
        if not runs:
            print("No runs found")
            exit()
            
        run = runs[0]
        print(f"Latest run ID: {run['id']}, Status: {run['status']}, Conclusion: {run['conclusion']}")
        
        # Get jobs for the run
        jobs_url = run['jobs_url']
        req2 = urllib.request.Request(jobs_url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req2) as resp2:
            jobs_data = json.loads(resp2.read().decode())
            for job in jobs_data.get('jobs', []):
                print(f"Job: {job['name']} - {job['status']} - {job['conclusion']}")
                if job['conclusion'] == 'failure':
                    print(f"Failed step: {[s['name'] for s in job['steps'] if s['conclusion'] == 'failure']}")
except Exception as e:
    print(e)
