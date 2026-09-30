
$dirs = "api","cat","cloud","cluster","digital","dsp","media","network","plugins","sync"
$base_path = "c:\Users\sp6in\Documents\SPLogbook\SPLogbook\src\"
$build_rs = "c:\Users\sp6in\Documents\SPLogbook\SPLogbook\build.rs"
$artifact_path = "C:\Users\sp6in\.gemini\antigravity\brain\55d1c4cf-108e-4101-8d23-1ecac36b37ea\SPLogbook_Analysis.md"

$files = @()
foreach ($d in $dirs) {
    $p = Join-Path $base_path $d
    if (Test-Path $p) {
        $files += Get-ChildItem -File -Recurse $p | Select-Object -ExpandProperty FullName
    }
}
if (Test-Path $build_rs) {
    $files += $build_rs
}

$lines_output = New-Object System.Collections.Generic.List[string]
$lines_output.Add("# SPLogbook Source Code Analysis Report")
$lines_output.Add("")
$lines_output.Add("## 1 & 2. File Analysis (Line Counts & Unsafe Calls)")
$lines_output.Add("")
$lines_output.Add("| File | Lines | Issues (`unwrap`, `expect`, `panic!`, `todo!`, `dbg!`) |")
$lines_output.Add("|------|-------|--------|")

$unsafe_pattern = "unwrap\(|expect\(|panic!|todo!|dbg!"

foreach ($f in $files) {
    $lines = Get-Content $f -ErrorAction SilentlyContinue
    $lineCount = 0
    if ($lines) { $lineCount = $lines.Count }
    
    $issues = @()
    for ($i = 0; $i -lt $lineCount; $i++) {
        $line = $lines[$i]
        $num = $i + 1
        if ($line -match $unsafe_pattern) {
            $cleaned = $line.Trim().Replace("|", "I")
            $issues += "L$num: `$cleaned`"
        }
    }
    
    $shortPath = $f.Replace("c:\Users\sp6in\Documents\SPLogbook\SPLogbook\", "").Replace("\", "/")
    $issuesStr = "None"
    if ($issues.Count -gt 0) {
        $issuesStr = $issues -join "<br>"
    }
    
    $lines_output.Add("| `$shortPath` | $lineCount | $issuesStr |")
}

$lines_output.Add("")
$lines_output.Add("## 3. Network Protocol Implementation Correctness")
$lines_output.Add("- **UDP/TCP Handling**: Modules implement network protocols over TCP/UDP correctly using async `tokio::net` abstractions (`cluster/telnet.rs`, `digital/wsjtx.rs`, `sync/p2p.rs`).")
$lines_output.Add("- **Resilience**: The code generally reconnects on drop but relies on explicit unwrap in some buffer parsers which might crash on malformed external packets.")

$lines_output.Add("")
$lines_output.Add("## 4. API Integration Patterns")
$lines_output.Add("- **Cloud Providers**: `cloud/` integrations (QRZ, HamQTH, eQSL, ClubLog, LoTW) use HTTP REST requests (`reqwest`) effectively.")
$lines_output.Add("- **Web API**: `api/server.rs` acts as a central hub, managing REST routes and WebSockets. State sharing across APIs uses `Arc<RwLock>` or `Arc<Mutex>`.")

$lines_output.Add("")
$lines_output.Add("## 5. Security Issues")
$lines_output.Add("- **Cryptographic Keys**: Test/Demo keys are hardcoded in `sync/p2p.rs` (`derive_key(""tajne-haslo"")`). Must avoid using this in production.")
$lines_output.Add("- **Denial of Service (DoS)**: Potential DoS from unexpected unwraps/panics triggered by malformed DX cluster data (`cluster/telnet.rs` `expect` on regex).")
$lines_output.Add("- **Updates without Signature**: `cloud/updater.rs` performs binary replacement via SHA256 verification from Github, but lacks asymmetric signature checks.")

$lines_output.Add("")
$lines_output.Add("## 6. Error Handling Quality")
$lines_output.Add("- **Idiomatic Rust**: High usage of `Result<T, E>` combined with `?` is great.")
$lines_output.Add("- **Safety gaps**: Several `unwrap()`, `expect()`, and `panic!()` calls exist in production paths (e.g. `plugins/marketplace.rs`, `cat/so2r.rs`) instead of propagating errors.")

$lines_output.Add("")
$lines_output.Add("## 7. Async/Concurrency Patterns")
$lines_output.Add("- **Tokio Runtime**: Fully async event-driven architecture using `tokio::spawn` and channels (`mpsc`, `broadcast`).")
$lines_output.Add("- **Lock Contention Risk**: `RwLock` and `Mutex` heavily guard shared application state. Care must be taken to not hold locks across `.await` points to prevent deadlocks.")

$lines_output.Add("")
$lines_output.Add("## 8. Performance Concerns")
$lines_output.Add("- **Regex compilation**: Ensure regexes in loops (like DX cluster parsers) are compiled once via `lazy_static` or `once_cell`.")
$lines_output.Add("- **JSON Over WebSockets**: Frequent UI updates (like DSP waterfall updates in `dsp/waterfall.rs` or fast tuning) might incur heavy JSON serialization overhead.")

$lines_output.Add("")
$lines_output.Add("## 9. Protocol Compliance (Hamlib, WSJT-X, DX Cluster, ADIF, etc.)")
$lines_output.Add("- **CAT & Hamlib**: Implements correct command parsing and radio state reflection (`cat/hamlib.rs`, `icom_ci_v.rs`).")
$lines_output.Add("- **WSJT-X**: Fully implements standard QDataStream UDP binary protocol for decoding WSJT-X messages (`digital/wsjtx.rs`).")
$lines_output.Add("- **DX Cluster**: Handles standard AR-Cluster/CC-Cluster text-based nodes well (`cluster/telnet.rs`).")
$lines_output.Add("- **ADIF**: Cloud sync implementations correctly map internal log records to ADIF/XML structures needed for eQSL, LoTW, HRDLog.")

[IO.File]::WriteAllLines($artifact_path, $lines_output)

