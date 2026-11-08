<#
.SYNOPSIS
    Skrypt budowy instalatora SPLogbook Windows User Setup (Inno Setup 7.1.1).
    Bramka: STAGE-M3-D-WINDOWS-USER-SETUP-INTEGRATION-GATE
    Decyzja: OWNER-M3-D-TOOLCHAIN-01 (APPROVED 2026-11-08)

.DESCRIPTION
    Skrypt dziala w trybie fail-closed, weryfikuje czystosc srodowiska, kompiluje
    binaria release Rust (SPLogbook.exe, splogbook-updater.exe), przygotowuje
    izolowany staging allowlisty i uruchamia zatwierdzony kompilator Inno Setup 7.1.1
    bez podpisu (unsigned test artifact).
#>

[CmdletBinding()]
param(
    [string]$OutputDir = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "=================================================================="
Write-Host "SPLogbook Windows User Setup Build Engine (M3-D)"
Write-Host "Data: 2026-11-08"
Write-Host "=================================================================="

$repoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $repoRoot

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "dist"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $repoRoot $OutputDir
}

# 1. Weryfikacja zatwierdzonego kompilatora Inno Setup 7.1.1 (OWNER-M3-D-TOOLCHAIN-01)
$approvedIsccPath = "C:\Program Files\Inno Setup 7\ISCC.exe"
if (-not (Test-Path $approvedIsccPath)) {
    throw "BLOKER: Zatwierdzony kompilator Inno Setup nie istnieje pod sciezka: $approvedIsccPath"
}

$isccVersionOutput = (& $approvedIsccPath --version 2>&1) | Out-String
if ($isccVersionOutput.Trim() -ne "7.1.1") {
    throw "BLOKER: Niezgodna wersja kompilatora Inno Setup. Oczekiwano '7.1.1', wykryto: '$($isccVersionOutput.Trim())'"
}
Write-Host "[OK] Zatwierdzony kompilator Inno Setup 7.1.1 zweryfikowany: $approvedIsccPath"

# 2. Weryfikacja wersji w Cargo.toml
$metadataRaw = cargo metadata --no-deps --format-version 1
$metadata = $metadataRaw | ConvertFrom-Json
$pkg = $metadata.packages | Where-Object { $_.name -eq "splogbook" }
if (-not $pkg) {
    throw "BLOKER: Nie odnaleziono pakietu splogbook w cargo metadata"
}

$expectedVersion = "1.3.0-alpha.1"
if ($pkg.version -ne $expectedVersion) {
    throw "BLOKER: Niezgodna wersja pakietu w Cargo.toml. Oczekiwano '$expectedVersion', wykryto: '$($pkg.version)'"
}
$appVersion = $pkg.version
Write-Host "[OK] Wersja pakietu zweryfikowana: $appVersion"

# 3. Pobranie commita Git
$gitCommit = (git rev-parse HEAD).Trim()
if ($gitCommit.Length -ne 40) {
    throw "BLOKER: Niepoprawny hash commita Git: $gitCommit"
}
Write-Host "[OK] Commit wydania: $gitCommit"

# 4. Budowa binariow w profilu release
Write-Host "Kompilacja binariow release (SPLogbook, splogbook-updater)..."
cargo build --release --bin SPLogbook
if ($LASTEXITCODE -ne 0) {
    throw "BLOKER: Blad kompilacji SPLogbook.exe"
}

cargo build --release --bin splogbook-updater
if ($LASTEXITCODE -ne 0) {
    throw "BLOKER: Blad kompilacji splogbook-updater.exe"
}

$mainBin = Join-Path $repoRoot "target\release\SPLogbook.exe"
$updaterBin = Join-Path $repoRoot "target\release\splogbook-updater.exe"

if (-not (Test-Path $mainBin)) {
    throw "BLOKER: Brak pliku binarnego $mainBin"
}
if (-not (Test-Path $updaterBin)) {
    throw "BLOKER: Brak pliku binarnego $updaterBin"
}
Write-Host "[OK] Binaria release skompilowane pomyslnie."

# 5. Przygotowanie izolowanego katalogu stagingu allowlisty
$stagingDir = Join-Path ([System.IO.Path]::GetTempPath()) "SPLogbook_UserSetup_Staging_$([System.Guid]::NewGuid().ToString())"
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null
Write-Host "Przygotowanie stagingu: $stagingDir"

try {
    # 5.1 Kopiowanie binariow
    Copy-Item -Path $mainBin -Destination (Join-Path $stagingDir "SPLogbook.exe") -Force
    Copy-Item -Path $updaterBin -Destination (Join-Path $stagingDir "splogbook-updater.exe") -Force

    # 5.2 Kopiowanie assets
    $destAssets = Join-Path $stagingDir "assets"
    New-Item -ItemType Directory -Path $destAssets -Force | Out-Null
    foreach ($assetName in @("icon.ico", "icon.png", "icon.rgba")) {
        $srcAsset = Join-Path $repoRoot "assets\$assetName"
        if (Test-Path $srcAsset) {
            Copy-Item -Path $srcAsset -Destination (Join-Path $destAssets $assetName) -Force
        } else {
            throw "BLOKER: Brak wymaganego pliku zasobow: $srcAsset"
        }
    }

    # 5.3 Kopiowanie locales
    $destLocales = Join-Path $stagingDir "locales"
    New-Item -ItemType Directory -Path $destLocales -Force | Out-Null
    $srcLocales = Join-Path $repoRoot "locales"
    Get-ChildItem -Path $srcLocales -Filter "*.json" -File | ForEach-Object {
        Copy-Item -Path $_.FullName -Destination (Join-Path $destLocales $_.Name) -Force
    }

    $srcManual = Join-Path $srcLocales "manual"
    if (Test-Path $srcManual) {
        $destManual = Join-Path $destLocales "manual"
        New-Item -ItemType Directory -Path $destManual -Force | Out-Null
        Get-ChildItem -Path $srcManual -Filter "*.json" -File | ForEach-Object {
            Copy-Item -Path $_.FullName -Destination (Join-Path $destManual $_.Name) -Force
        }
    }

    # 5.4 Kopiowanie baz referencyjnych (FirstInstallSeed)
    $destDatabases = Join-Path $stagingDir "databases"
    New-Item -ItemType Directory -Path $destDatabases -Force | Out-Null
    foreach ($dbName in @("callbook.db", "serviceLOG.db", "clubs.csv")) {
        $srcDb = Join-Path $repoRoot "databases\$dbName"
        if (Test-Path $srcDb) {
            Copy-Item -Path $srcDb -Destination (Join-Path $destDatabases $dbName) -Force
        } else {
            throw "BLOKER: Brak wymaganej bazy referencyjnej: $srcDb"
        }
    }

    # 6. Weryfikacja braku zakazanych plikow w stagingu
    $forbiddenFiles = Get-ChildItem -Path $stagingDir -Recurse -File | Where-Object {
        $_.Name -match '\.(key|pem|p12|pfx|pdb|log|env)$' -or
        $_.Name -eq "default_log.db" -or
        $_.Name -eq "station_config.json" -or
        $_.FullName -match '\\\.git\\'
    }
    if ($forbiddenFiles) {
        throw "BLOKER: Wykryto zakazane pliki w stagingu: $($forbiddenFiles | Select-Object -ExpandProperty FullName)"
    }
    Write-Host "[OK] Staging zweryfikowany pod katem allowlisty i braku sekretow."

    # 7. Przygotowanie katalogu wyjsciowego
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    }
    $resolvedOutputDir = (Resolve-Path $OutputDir).Path
    $outputBaseFilename = "SPLogbook-Windows-x64-Setup-$appVersion"
    $expectedOutputExe = Join-Path $resolvedOutputDir "$outputBaseFilename.exe"

    if (Test-Path $expectedOutputExe) {
        Remove-Item -Path $expectedOutputExe -Force
    }

    # 8. Uruchomienie kompilatora ISCC z flaga --no-signing
    $issScript = Join-Path $repoRoot "installer\windows\inno\SPLogbook.iss"
    if (-not (Test-Path $issScript)) {
        throw "BLOKER: Brak skryptu Inno Setup: $issScript"
    }

    Write-Host "Uruchamianie kompilatora Inno Setup 7.1.1..."
    $isccArgs = @(
        "--no-signing",
        "/DAppVersion=$appVersion",
        "/DGitCommit=$gitCommit",
        "/DBuildChannel=alpha",
        "/DDistributionStatus=development",
        "/DStagingDir=$stagingDir",
        "/DOutputDir=$resolvedOutputDir",
        "/DOutputBaseFilename=$outputBaseFilename",
        $issScript
    )

    $processInfo = New-Object System.Diagnostics.ProcessStartInfo
    $processInfo.FileName = $approvedIsccPath
    $processInfo.Arguments = ($isccArgs | ForEach-Object {
        if ($_ -match ' ') { "`"$_`"" } else { $_ }
    }) -join ' '
    $processInfo.RedirectStandardOutput = $true
    $processInfo.RedirectStandardError = $true
    $processInfo.UseShellExecute = $false
    $processInfo.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::Start($processInfo)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()

    $isccExitCode = $process.ExitCode
    $combinedOutput = "$stdout`n$stderr"

    Write-Host "--- ISCC Output ---"
    Write-Host $combinedOutput
    Write-Host "-------------------"

    if ($isccExitCode -ne 0) {
        throw "BLOKER: Kompilator Inno Setup zakonczyl sie bledem (kod: $isccExitCode)"
    }

    # 9. Sprawdzenie ostrzezen kompilatora (kazdy warning = bloker)
    if ($combinedOutput -match '(?i)warning') {
        throw "BLOKER: Kompilator ISCC zglosil ostrzezenia! Kazdy warning jest blokujacy."
    }

    # 10. Weryfikacja wynikowego artefaktu
    if (-not (Test-Path $expectedOutputExe)) {
        throw "BLOKER: Oczekiwany plik instalatora nie zostal utworzony: $expectedOutputExe"
    }

    $hash = (Get-FileHash -Algorithm SHA256 -Path $expectedOutputExe).Hash
    $size = (Get-Item -Path $expectedOutputExe).Length

    Write-Host "=================================================================="
    Write-Host "SUKCES: Instalator Windows User Setup zostal pomyslnie zbudowany!"
    Write-Host "Plik: $expectedOutputExe"
    Write-Host "Rozmiar: $size bajtow"
    Write-Host "SHA-256: $hash"
    Write-Host "Podpis: UNSIGNED TEST ARTIFACT"
    Write-Host "=================================================================="

    return [PSCustomObject]@{
        Status = "SUCCESS"
        InstallerPath = $expectedOutputExe
        Version = $appVersion
        Commit = $gitCommit
        Sha256 = $hash
        Size = $size
        Signature = "UNSIGNED TEST ARTIFACT"
    }
}
finally {
    if (Test-Path $stagingDir) {
        Remove-Item -Path $stagingDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
