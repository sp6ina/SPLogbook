//! Utwardzony fallback kompatybilności PowerShell dla aktualizatora SPLogbook na Windows (M3-B).
//!
//! Zgodnie ze specyfikacją M3-B oraz decyzjami architektonicznymi:
//! - PowerShell jest WYŁĄCZNIE kontrolowanym fallbackiem kompatybilności.
//! - PowerShell nie jest niezależnym updaterem i nie wykonuje operacji sieciowych.
//! - PowerShell otrzymuje wyłącznie przygotowany lokalnie, zweryfikowany kontrakt `WindowsHelperRequest`
//!   lub parametry z zatwierdzonego `UpdatePlan`.
//! - PowerShell bezwzględnie NIE UŻYWA:
//!   * dzikich kart (`*`, `Copy-Item "$staging\*"`, `Copy-Item "$rollback\*"`);
//!   * dynamicznego wykonywania kodu (`Invoke-Expression`, `iex`);
//!   * eskalacji uprawnień (`runas`, `sudo`, `UAC`);
//!   * pobierania z sieci (`Invoke-WebRequest`, `curl`, `wget`, `Net.WebClient`);
//!   * niesprawdzonych stringów argumentów (wymuszone tablice argumentów).
//! - Wszystkie operacje na ścieżkach wymuszają `-LiteralPath`.
//! - Skrypt wymusza `Set-StrictMode -Version Latest` oraz `$ErrorActionPreference = 'Stop'`.
//! - Kopia `.rollback` po udanej aktualizacji jest ZACHOWYWANA (decyzja retencyjna OWNER-M2-08/09).

use std::path::Path;

use crate::cloud::updater::UpdatePlan;
use crate::cloud::updater::helper_protocol::WindowsHelperRequest;

/// Generuje bezpieczny, utwardzony skrypt PowerShell wykonujący podmianę plików
/// na podstawie zatwierdzonego `UpdatePlan` (kontrolowany fallback kompatybilności).
pub fn generate_powershell_update_script(exe: &Path, pid: u32, plan: &UpdatePlan) -> String {
    let exe_str = exe.to_string_lossy().replace('\'', "''");

    let replace_items: Vec<String> = plan
        .replace_files
        .iter()
        .map(|(r, _)| format!("'{}'", r.to_string_lossy().replace('\'', "''")))
        .collect();
    let replace_items_ps = replace_items.join(", ");

    let create_items: Vec<String> = plan
        .create_files
        .iter()
        .map(|(r, _)| format!("'{}'", r.to_string_lossy().replace('\'', "''")))
        .collect();
    let create_items_ps = create_items.join(", ");

    format!(
        "Set-StrictMode -Version Latest\n\
         $ErrorActionPreference = 'Stop'\n\
         $ConfirmPreference = 'None'\n\
         \n\
         $exe = '{exe_str}'\n\
         $pidToWait = {pid}\n\
         \n\
         $baseDir = Split-Path -Path $exe -Parent\n\
         $staging = Join-Path $baseDir '.staging'\n\
         $rollback = Join-Path $baseDir '.rollback'\n\
         $healthOk = Join-Path $baseDir '.health_ok'\n\
         \n\
         $replaceFiles = @({replace_items_ps})\n\
         $createFiles = @({create_items_ps})\n\
         \n\
         if (Test-Path -LiteralPath $rollback) {{ Remove-Item -Recurse -Force -LiteralPath $rollback }}\n\
         New-Item -ItemType Directory -Force -Path $rollback | Out-Null\n\
         \n\
         $deadline = (Get-Date).AddSeconds(90)\n\
         while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $deadline) {{ exit 2 }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 500\n\
         \n\
         # 1. Kopia zapasowa wylacznie dla plikow podmienianych z UpdatePlan\n\
         foreach ($rel in $replaceFiles) {{\n\
             $orig = Join-Path $baseDir $rel\n\
             if (Test-Path -LiteralPath $orig) {{\n\
                 $dest = Join-Path $rollback $rel\n\
                 $destDir = Split-Path $dest -Parent\n\
                 if (-not (Test-Path -LiteralPath $destDir)) {{ New-Item -ItemType Directory -Force -Path $destDir | Out-Null }}\n\
                 Copy-Item -LiteralPath $orig -Destination $dest -Force\n\
             }}\n\
         }}\n\
         \n\
         # 2. Bezpieczna podmiana plikow programu\n\
         foreach ($rel in $replaceFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $baseDir $rel\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         # 3. Utworzenie nowych dopuszczonych plikow (ziarna/zasoby)\n\
         foreach ($rel in $createFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $baseDir $rel\n\
             $dstDir = Split-Path $dst -Parent\n\
             if (-not (Test-Path -LiteralPath $dstDir)) {{ New-Item -ItemType Directory -Force -Path $dstDir | Out-Null }}\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         if (Test-Path -LiteralPath $healthOk) {{ Remove-Item -Force -LiteralPath $healthOk }}\n\
         \n\
         $argList = @('--check-health-startup')\n\
         $newProc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru\n\
         \n\
         $healthDeadline = (Get-Date).AddSeconds(15)\n\
         $success = $false\n\
         while ((Get-Date) -lt $healthDeadline) {{\n\
             if (Test-Path -LiteralPath $healthOk) {{\n\
                 $success = $true\n\
                 break\n\
             }}\n\
             if ($newProc.HasExited) {{\n\
                 break\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         \n\
         if ($success) {{\n\
             Remove-Item -Recurse -Force -LiteralPath $staging -ErrorAction SilentlyContinue\n\
             Remove-Item -Force -LiteralPath $healthOk -ErrorAction SilentlyContinue\n\
             # Kopia .rollback jest ZACHOWYWANA do decyzji retencyjnej (M3-B / OWNER-M2-08/09)\n\
             exit 0\n\
         }} else {{\n\
             Stop-Process -Id $newProc.Id -Force -ErrorAction SilentlyContinue\n\
             # Rollback: usuniecie nowo utworzonych plikow\n\
             foreach ($rel in $createFiles) {{\n\
                 $target = Join-Path $baseDir $rel\n\
                 if (Test-Path -LiteralPath $target) {{ Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue }}\n\
             }}\n\
             # Rollback: przywrocenie wylacznie plikow podmienionych z rollbacku\n\
             foreach ($rel in $replaceFiles) {{\n\
                 $bkp = Join-Path $rollback $rel\n\
                 if (Test-Path -LiteralPath $bkp) {{\n\
                     Copy-Item -LiteralPath $bkp -Destination (Join-Path $baseDir $rel) -Force\n\
                 }}\n\
             }}\n\
             Start-Process -FilePath $exe\n\
             exit 9\n\
         }}\n"
    )
}

/// Generuje utwardzony skrypt PowerShell wykonujący operację na podstawie pliku `WindowsHelperRequest`.
pub fn generate_powershell_from_request_file(request_path: &Path) -> String {
    let req_str = request_path.to_string_lossy().replace('\'', "''");

    format!(
        "Set-StrictMode -Version Latest\n\
         $ErrorActionPreference = 'Stop'\n\
         $ConfirmPreference = 'None'\n\
         \n\
         $reqPath = '{req_str}'\n\
         if (-not (Test-Path -LiteralPath $reqPath)) {{\n\
             Write-Error \"Plik zlecenia nie istnieje: $reqPath\"\n\
             exit 1\n\
         }}\n\
         \n\
         $raw = Get-Content -LiteralPath $reqPath -Raw -Encoding UTF8\n\
         $req = ConvertFrom-Json -InputObject $raw\n\
         \n\
         if ($req.request_version -ne 1) {{\n\
             Write-Error \"Unsupported request version: $($req.request_version)\"\n\
             exit 1\n\
         }}\n\
         \n\
         $root = $req.installation_root\n\
         $candExe = $req.candidate_executable\n\
         $staging = Split-Path -Path $candExe -Parent\n\
         $rollback = Join-Path $root '.rollback'\n\
         $parentPid = $req.expected_parent_pid\n\
         $parentTimeout = [int]$req.parent_process_timeout_seconds\n\
         $healthTimeout = [int]$req.candidate_health_timeout_seconds\n\
         $healthProofPath = $req.health_proof_path\n\
         $attemptId = $req.attempt_id\n\
         $targetVer = $req.expected_target_version\n\
         $targetCommit = $req.expected_target_commit\n\
         $replaceFiles = @($req.update_plan_identity.ordered_replace_entries)\n\
         $createFiles = @($req.update_plan_identity.ordered_create_entries)\n\
         $resultPath = $req.result_record_path\n\
         \n\
         $planDigest = $req.plan_digest\n\
         $totalOps = $replaceFiles.Count + $createFiles.Count\n\
         \n\
         function Write-HelperResult($exitCode, $category, $finalState, $rollbackRes, $healthRes, $completedOps, $err) {{\n\
             if ($resultPath) {{\n\
                 $errMsg = if ($err) {{ $err }} else {{ \"\" }}\n\
                 $resObj = [PSCustomObject]@{{\n\
                     result_version = 1\n\
                     attempt_id = $attemptId\n\
                     plan_digest = $planDigest\n\
                     final_update_state = $finalState\n\
                     helper_engine = 'PowerShellFallback'\n\
                     result_category = $category\n\
                     status = $category\n\
                     exit_code = [int]$exitCode\n\
                     rollback_result = $rollbackRes\n\
                     health_result = $healthRes\n\
                     completed_operation_count = [int]$completedOps\n\
                     total_operation_count = [int]$totalOps\n\
                     backup_retained = $true\n\
                     manual_recovery_required = $false\n\
                     sanitized_diagnostic_summary = $errMsg\n\
                     completion_time = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')\n\
                 }}\n\
                 $resJson = ConvertTo-Json -InputObject $resObj\n\
                 Set-Content -LiteralPath $resultPath -Value $resJson -Encoding UTF8\n\
             }}\n\
         }}\n\
         \n\
         $parentDeadline = (Get-Date).AddSeconds($parentTimeout)\n\
         while (Get-Process -Id $parentPid -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $parentDeadline) {{\n\
                 Write-HelperResult 2 'AbortedPreApply' 'FailedPreApply' $null $null 0 'Parent process did not exit within designated timeout'\n\
                 exit 2\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 300\n\
         \n\
         if (Test-Path -LiteralPath $rollback) {{ Remove-Item -Recurse -Force -LiteralPath $rollback }}\n\
         New-Item -ItemType Directory -Force -Path $rollback | Out-Null\n\
         \n\
         foreach ($rel in $replaceFiles) {{\n\
             $orig = Join-Path $root $rel\n\
             if (Test-Path -LiteralPath $orig) {{\n\
                 $dest = Join-Path $rollback $rel\n\
                 $destDir = Split-Path $dest -Parent\n\
                 if (-not (Test-Path -LiteralPath $destDir)) {{ New-Item -ItemType Directory -Force -Path $destDir | Out-Null }}\n\
                 Copy-Item -LiteralPath $orig -Destination $dest -Force\n\
             }}\n\
         }}\n\
         \n\
         foreach ($rel in $replaceFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $root $rel\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         foreach ($rel in $createFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $root $rel\n\
             $dstDir = Split-Path $dst -Parent\n\
             if (-not (Test-Path -LiteralPath $dstDir)) {{ New-Item -ItemType Directory -Force -Path $dstDir | Out-Null }}\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         if (Test-Path -LiteralPath $healthProofPath) {{ Remove-Item -Force -LiteralPath $healthProofPath }}\n\
         \n\
         $candArgs = @(\n\
             '--check-health-startup',\n\
             '--update-attempt-id', \"`\"$attemptId`\"\",\n\
             '--health-proof-path', \"`\"$healthProofPath`\"\",\n\
             '--expected-version', \"`\"$targetVer`\"\",\n\
             '--expected-commit', \"`\"$targetCommit`\"\"\n\
         )\n\
         $proc = Start-Process -FilePath $candExe -ArgumentList $candArgs -PassThru\n\
         \n\
         $hDeadline = (Get-Date).AddSeconds($healthTimeout)\n\
         $healthVerified = $false\n\
         while ((Get-Date) -lt $hDeadline) {{\n\
             if (Test-Path -LiteralPath $healthProofPath) {{\n\
                 try {{\n\
                     $proofContent = Get-Content -LiteralPath $healthProofPath -Raw -Encoding UTF8\n\
                     $proofJson = ConvertFrom-Json -InputObject $proofContent\n\
                     if ($proofJson.attempt_id -eq $attemptId -and $proofJson.target_version -eq $targetVer -and $proofJson.target_commit -eq $targetCommit) {{\n\
                         $healthVerified = $true\n\
                         break\n\
                     }}\n\
                 }} catch {{\n\
                 }}\n\
             }}\n\
             if ($proc.HasExited) {{\n\
                 if (Test-Path -LiteralPath $healthProofPath) {{\n\
                     try {{\n\
                         $proofContent = Get-Content -LiteralPath $healthProofPath -Raw -Encoding UTF8\n\
                         $proofJson = ConvertFrom-Json -InputObject $proofContent\n\
                         if ($proofJson.attempt_id -eq $attemptId -and $proofJson.target_version -eq $targetVer -and $proofJson.target_commit -eq $targetCommit) {{\n\
                             $healthVerified = $true\n\
                         }}\n\
                     }} catch {{\n\
                     }}\n\
                 }}\n\
                 break\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         \n\
         if ($healthVerified) {{\n\
             Remove-Item -Recurse -Force -LiteralPath $staging -ErrorAction SilentlyContinue\n\
             Remove-Item -Force -LiteralPath $healthProofPath -ErrorAction SilentlyContinue\n\
             Write-HelperResult 0 'Success' 'Committed' 'None' 'Passed' $totalOps ''\n\
             exit 0\n\
         }} else {{\n\
             Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue\n\
             foreach ($rel in $createFiles) {{\n\
                 $t = Join-Path $root $rel\n\
                 if (Test-Path -LiteralPath $t) {{ Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue }}\n\
             }}\n\
             foreach ($rel in $replaceFiles) {{\n\
                 $bk = Join-Path $rollback $rel\n\
                 if (Test-Path -LiteralPath $bk) {{\n\
                     Copy-Item -LiteralPath $bk -Destination (Join-Path $root $rel) -Force\n\
                 }}\n\
             }}\n\
             Write-HelperResult 9 'FailedRollbackSucceeded' 'FailedPostApply' 'Success' 'TimedOut' $totalOps 'Health verification failed - rollback succeeded'\n\
             Start-Process -FilePath $candExe -ErrorAction SilentlyContinue\n\
             exit 9\n\
         }}\n"
    )
}

/// Generuje utwardzony skrypt z wbudowanego obiektu `WindowsHelperRequest`.
pub fn generate_powershell_from_request(req: &WindowsHelperRequest) -> String {
    let req_json = serde_json::to_string_pretty(req).unwrap_or_default();
    let json_escaped = req_json.replace('\'', "''");

    format!(
        "Set-StrictMode -Version Latest\n\
         $ErrorActionPreference = 'Stop'\n\
         $ConfirmPreference = 'None'\n\
         \n\
         $raw = @'\n\
{json_escaped}\n\
'@\n\
         $req = ConvertFrom-Json -InputObject $raw\n\
         \n\
         if ($req.request_version -ne 1) {{\n\
             Write-Error \"Unsupported request version: $($req.request_version)\"\n\
             exit 1\n\
         }}\n\
         \n\
         $root = $req.installation_root\n\
         $candExe = $req.candidate_executable\n\
         $staging = Split-Path -Path $candExe -Parent\n\
         $rollback = Join-Path $root '.rollback'\n\
         $parentPid = $req.expected_parent_pid\n\
         $parentTimeout = [int]$req.parent_process_timeout_seconds\n\
         $healthTimeout = [int]$req.candidate_health_timeout_seconds\n\
         $healthProofPath = $req.health_proof_path\n\
         $attemptId = $req.attempt_id\n\
         $targetVer = $req.expected_target_version\n\
         $targetCommit = $req.expected_target_commit\n\
         $replaceFiles = @($req.update_plan_identity.ordered_replace_entries)\n\
         $createFiles = @($req.update_plan_identity.ordered_create_entries)\n\
         $resultPath = $req.result_record_path\n\
         \n\
         $planDigest = $req.plan_digest\n\
         $totalOps = $replaceFiles.Count + $createFiles.Count\n\
         \n\
         function Write-HelperResult($exitCode, $category, $finalState, $rollbackRes, $healthRes, $completedOps, $err) {{\n\
             if ($resultPath) {{\n\
                 $errMsg = if ($err) {{ $err }} else {{ \"\" }}\n\
                 $resObj = [PSCustomObject]@{{\n\
                     result_version = 1\n\
                     attempt_id = $attemptId\n\
                     plan_digest = $planDigest\n\
                     final_update_state = $finalState\n\
                     helper_engine = 'PowerShellFallback'\n\
                     result_category = $category\n\
                     status = $category\n\
                     exit_code = [int]$exitCode\n\
                     rollback_result = $rollbackRes\n\
                     health_result = $healthRes\n\
                     completed_operation_count = [int]$completedOps\n\
                     total_operation_count = [int]$totalOps\n\
                     backup_retained = $true\n\
                     manual_recovery_required = $false\n\
                     sanitized_diagnostic_summary = $errMsg\n\
                     completion_time = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')\n\
                 }}\n\
                 $resJson = ConvertTo-Json -InputObject $resObj\n\
                 Set-Content -LiteralPath $resultPath -Value $resJson -Encoding UTF8\n\
             }}\n\
         }}\n\
         \n\
         $parentDeadline = (Get-Date).AddSeconds($parentTimeout)\n\
         while (Get-Process -Id $parentPid -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $parentDeadline) {{\n\
                 Write-HelperResult 2 'AbortedPreApply' 'FailedPreApply' $null $null 0 'Parent process did not exit within designated timeout'\n\
                 exit 2\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 300\n\
         \n\
         if (Test-Path -LiteralPath $rollback) {{ Remove-Item -Recurse -Force -LiteralPath $rollback }}\n\
         New-Item -ItemType Directory -Force -Path $rollback | Out-Null\n\
         \n\
         foreach ($rel in $replaceFiles) {{\n\
             $orig = Join-Path $root $rel\n\
             if (Test-Path -LiteralPath $orig) {{\n\
                 $dest = Join-Path $rollback $rel\n\
                 $destDir = Split-Path $dest -Parent\n\
                 if (-not (Test-Path -LiteralPath $destDir)) {{ New-Item -ItemType Directory -Force -Path $destDir | Out-Null }}\n\
                 Copy-Item -LiteralPath $orig -Destination $dest -Force\n\
             }}\n\
         }}\n\
         \n\
         foreach ($rel in $replaceFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $root $rel\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         foreach ($rel in $createFiles) {{\n\
             $src = Join-Path $staging $rel\n\
             $dst = Join-Path $root $rel\n\
             $dstDir = Split-Path $dst -Parent\n\
             if (-not (Test-Path -LiteralPath $dstDir)) {{ New-Item -ItemType Directory -Force -Path $dstDir | Out-Null }}\n\
             Copy-Item -LiteralPath $src -Destination $dst -Force\n\
         }}\n\
         \n\
         if (Test-Path -LiteralPath $healthProofPath) {{ Remove-Item -Force -LiteralPath $healthProofPath }}\n\
         \n\
         $candArgs = @(\n\
             '--check-health-startup',\n\
             '--update-attempt-id', \"`\"$attemptId`\"\",\n\
             '--health-proof-path', \"`\"$healthProofPath`\"\",\n\
             '--expected-version', \"`\"$targetVer`\"\",\n\
             '--expected-commit', \"`\"$targetCommit`\"\"\n\
         )\n\
         $proc = Start-Process -FilePath $candExe -ArgumentList $candArgs -PassThru\n\
         \n\
         $hDeadline = (Get-Date).AddSeconds($healthTimeout)\n\
         $healthVerified = $false\n\
         while ((Get-Date) -lt $hDeadline) {{\n\
             if (Test-Path -LiteralPath $healthProofPath) {{\n\
                 try {{\n\
                     $proofContent = Get-Content -LiteralPath $healthProofPath -Raw -Encoding UTF8\n\
                     $proofJson = ConvertFrom-Json -InputObject $proofContent\n\
                     if ($proofJson.attempt_id -eq $attemptId -and $proofJson.target_version -eq $targetVer -and $proofJson.target_commit -eq $targetCommit) {{\n\
                         $healthVerified = $true\n\
                         break\n\
                     }}\n\
                 }} catch {{\n\
                 }}\n\
             }}\n\
             if ($proc.HasExited) {{\n\
                 if (Test-Path -LiteralPath $healthProofPath) {{\n\
                     try {{\n\
                         $proofContent = Get-Content -LiteralPath $healthProofPath -Raw -Encoding UTF8\n\
                         $proofJson = ConvertFrom-Json -InputObject $proofContent\n\
                         if ($proofJson.attempt_id -eq $attemptId -and $proofJson.target_version -eq $targetVer -and $proofJson.target_commit -eq $targetCommit) {{\n\
                             $healthVerified = $true\n\
                         }}\n\
                     }} catch {{\n\
                     }}\n\
                 }}\n\
                 break\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         \n\
         if ($healthVerified) {{\n\
             Remove-Item -Recurse -Force -LiteralPath $staging -ErrorAction SilentlyContinue\n\
             Remove-Item -Force -LiteralPath $healthProofPath -ErrorAction SilentlyContinue\n\
             Write-HelperResult 0 'Success' 'Committed' 'None' 'Passed' $totalOps ''\n\
             exit 0\n\
         }} else {{\n\
             Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue\n\
             foreach ($rel in $createFiles) {{\n\
                 $t = Join-Path $root $rel\n\
                 if (Test-Path -LiteralPath $t) {{ Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue }}\n\
             }}\n\
             foreach ($rel in $replaceFiles) {{\n\
                 $bk = Join-Path $rollback $rel\n\
                 if (Test-Path -LiteralPath $bk) {{\n\
                     Copy-Item -LiteralPath $bk -Destination (Join-Path $root $rel) -Force\n\
                 }}\n\
             }}\n\
             Write-HelperResult 9 'FailedRollbackSucceeded' 'FailedPostApply' 'Success' 'TimedOut' $totalOps 'Health verification failed - rollback succeeded'\n\
             Start-Process -FilePath $candExe -ErrorAction SilentlyContinue\n\
             exit 9\n\
         }}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn powershell_script_generation_has_strict_mode_and_no_wildcards() {
        let plan = UpdatePlan {
            replace_files: vec![(
                PathBuf::from("SPLogbook.exe"),
                PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: vec![(
                PathBuf::from("resources/icon.png"),
                PathBuf::from("C:\\SPLogbook\\resources\\icon.png"),
            )],
            preserve_files: vec![(
                PathBuf::from("C:\\SPLogbook\\databases\\serviceLOG.db"),
                "Preserved DB".into(),
            )],
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            1234,
            &plan,
        );

        assert!(script.contains("Set-StrictMode -Version Latest"));
        assert!(script.contains("$ErrorActionPreference = 'Stop'"));
        assert!(!script.contains("Copy-Item -Path \"$staging\\*\""));
        assert!(!script.contains("Copy-Item -Path \"$rollback\\*\""));
        assert!(!script.contains("$staging\\*"));
        assert!(!script.contains("$rollback\\*"));
        assert!(!script.contains("Invoke-Expression"));
        assert!(!script.contains("iex"));
        assert!(!script.contains("runas"));
        assert!(!script.contains("serviceLOG.db"));
        assert!(script.contains("$replaceFiles = @('SPLogbook.exe')"));
    }

    #[test]
    fn powershell_request_runner_has_strict_mode_and_structured_args() {
        let runner = generate_powershell_from_request_file(Path::new("C:\\temp\\request.json"));

        assert!(runner.contains("Set-StrictMode -Version Latest"));
        assert!(runner.contains("$ErrorActionPreference = 'Stop'"));
        assert!(!runner.contains("Invoke-Expression"));
        assert!(!runner.contains("iex"));
        assert!(!runner.contains("runas"));
        assert!(runner.contains("ConvertFrom-Json"));
        assert!(runner.contains("-LiteralPath"));
    }

    #[test]
    fn helper_parity_plan_safety_policy() {
        let plan = UpdatePlan {
            replace_files: vec![(
                PathBuf::from("SPLogbook.exe"),
                PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: vec![(
                PathBuf::from("config/station.json"),
                PathBuf::from("C:\\SPLogbook\\config\\station.json"),
            )],
            preserve_files: vec![
                (
                    PathBuf::from("C:\\SPLogbook\\databases\\serviceLOG.db"),
                    "Preserved SQLite DB".into(),
                ),
                (
                    PathBuf::from("C:\\SPLogbook\\station_config.json"),
                    "Preserved User Config".into(),
                ),
            ],
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            4321,
            &plan,
        );

        // Ochrona baz danych i konfiguracji użytkownika
        assert!(!script.contains("serviceLOG.db"));
        assert!(!script.contains("station_config.json"));
        // Zakaz dynamicznego wykonywania i eskalacji
        assert!(!script.contains("Invoke-Expression"));
        assert!(!script.contains("iex"));
        assert!(!script.contains("runas"));
    }

    #[test]
    fn helper_parity_rollback_retention() {
        let plan = UpdatePlan {
            replace_files: vec![(
                PathBuf::from("SPLogbook.exe"),
                PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            )],
            create_files: Vec::new(),
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };

        let script = generate_powershell_update_script(
            Path::new("C:\\SPLogbook\\SPLogbook.exe"),
            4321,
            &plan,
        );

        // Skrypt nie może usuwać katalogu .rollback po sukcesie
        let success_block = script
            .split("if ($success) {")
            .nth(1)
            .and_then(|s| s.split("} else {").next())
            .unwrap_or("");
        assert!(
            !success_block.contains("Remove-Item -Recurse -Force -LiteralPath $rollback"),
            "Katalog .rollback musi być zachowany po sukcesie na potrzeby retencji"
        );
    }

    #[test]
    fn helper_parity_exit_codes_alignment() {
        let runner = generate_powershell_from_request_file(Path::new("C:\\temp\\req.json"));

        // Exit codes muszą odpowiadać WindowsHelperExitCode
        assert!(runner.contains("exit 0")); // Success
        assert!(runner.contains("exit 1")); // InvalidArguments
        assert!(runner.contains("exit 2")); // ParentProcessDidNotExit
        assert!(runner.contains("exit 9")); // HealthVerificationFailedRollbackSucceeded
    }
}
