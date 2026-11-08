<#
.SYNOPSIS
    Skrypt budowy instalatora SPLogbook Windows System Setup (MSI / WiX Toolset 7.0.1).
    Bramka: STAGE-M3-E-WINDOWS-MSI-WIX-INTEGRATION-GATE
    Decyzja: OWNER-M3-E-TOOLCHAIN-01 (APPROVED 2026-11-08)

.DESCRIPTION
    Skrypt dziala w trybie fail-closed, weryfikuje czystosc srodowiska, kompiluje
    binaria release Rust (SPLogbook.exe, splogbook-updater.exe), przygotowuje
    izolowany staging allowlisty, generuje marker .splogbook-msi-marker i uruchamia
    zatwierdzony kompilator WiX Toolset 7.0.1 / MSBuild bez podpisu (unsigned test artifact).
#>

[CmdletBinding()]
param(
    [string]$OutputDir = "",
    [switch]$SkipCargoBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "=================================================================="
Write-Host "SPLogbook Windows System Setup MSI Build Engine (M3-E)"
Write-Host "Data: 2026-11-08"
Write-Host "WiX Toolset SDK: 7.0.1 (Decyzja OWNER-M3-E-TOOLCHAIN-01)"
Write-Host "=================================================================="

$repoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $repoRoot

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $repoRoot "dist"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $repoRoot $OutputDir
}

# 1. Weryfikacja zatwierdzonego silnika budowania MSBuild 2022
$approvedMsbuildPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe"
if (-not (Test-Path $approvedMsbuildPath)) {
    throw "BLOKER: Zatwierdzony kompilator MSBuild nie istnieje pod sciezka: $approvedMsbuildPath"
}
Write-Host "[OK] MSBuild 2022 zweryfikowany: $approvedMsbuildPath"

# 2. Weryfikacja EULA WiX 7
$wixEulaFile = Join-Path $env:USERPROFILE ".wix\wix7-osmf-eula.txt"
if (-not (Test-Path $wixEulaFile)) {
    Write-Host "Inicjalizacja akceptacji EULA WiX 7..."
    $localWix = Join-Path $env:USERPROFILE ".nuget\packages\wixtoolset.sdk\7.0.1\tools\net472\x86\wix.exe"
    if (Test-Path $localWix) {
        & $localWix eula accept wix7
    } else {
        throw "BLOKER: Brak zaakceptowanej licencji EULA WiX 7 ($wixEulaFile)"
    }
}
Write-Host "[OK] EULA WiX 7 zweryfikowana ($wixEulaFile)"

# 3. Weryfikacja wersji w Cargo.toml
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
$displayVersion = $pkg.version
$msiProductVersion = "1.3.0" # Format major.minor.build dla Windows Installer
Write-Host "[OK] Wersja SemVer: $displayVersion, Wersja MSI ProductVersion: $msiProductVersion"

# 4. Pobranie commita Git
$gitCommit = (git rev-parse HEAD).Trim()
if ($gitCommit.Length -ne 40) {
    throw "BLOKER: Niepoprawny hash commita Git: $gitCommit"
}
Write-Host "[OK] Commit wydania: $gitCommit"

# 5. Budowa binariow w profilu release (chyba ze pominieto)
$mainBin = Join-Path $repoRoot "target\release\SPLogbook.exe"
$updaterBin = Join-Path $repoRoot "target\release\splogbook-updater.exe"

if (-not $SkipCargoBuild) {
    Write-Host "Kompilacja binariow release (SPLogbook, splogbook-updater)..."
    cargo build --release --bin SPLogbook
    if ($LASTEXITCODE -ne 0) {
        throw "BLOKER: Blad kompilacji SPLogbook.exe"
    }

    cargo build --release --bin splogbook-updater
    if ($LASTEXITCODE -ne 0) {
        throw "BLOKER: Blad kompilacji splogbook-updater.exe"
    }
}

if (-not (Test-Path $mainBin)) {
    throw "BLOKER: Brak pliku binarnego $mainBin"
}
if (-not (Test-Path $updaterBin)) {
    throw "BLOKER: Brak pliku binarnego $updaterBin"
}
Write-Host "[OK] Binaria release gotowe."

# 6. Przygotowanie izolowanego katalogu stagingu allowlisty
$stagingDir = Join-Path ([System.IO.Path]::GetTempPath()) "SPLogbook_MSI_Staging_$([System.Guid]::NewGuid().ToString())"
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null
Write-Host "Przygotowanie stagingu: $stagingDir"

try {
    # 6.1 Kopiowanie binariow
    Copy-Item -Path $mainBin -Destination (Join-Path $stagingDir "SPLogbook.exe") -Force
    Copy-Item -Path $updaterBin -Destination (Join-Path $stagingDir "splogbook-updater.exe") -Force

    # 6.2 Generowanie kanonicznego markera MSI
    $markerContent = @{
        marker_version     = "1.0"
        installation_type  = "WindowsMsi"
        management_mode    = "SystemManaged"
        package_version    = $displayVersion
        upgrade_code       = "{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}"
        product_code       = "*"
        installation_scope = "PerMachine"
        installation_root  = "C:\Program Files\SPLogbook"
        build_commit       = $gitCommit
        installer_engine   = "WindowsInstallerWiX"
    } | ConvertTo-Json -Compress

    Set-Content -Path (Join-Path $stagingDir ".splogbook-msi-marker") -Value $markerContent -Encoding UTF8

    # 6.3 Kopiowanie assets
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

    # 6.4 Kopiowanie locales
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

    # 6.5 Kopiowanie baz referencyjnych (FirstInstallSeed)
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

    # 7. Weryfikacja braku zakazanych plikow w stagingu
    $forbiddenFiles = Get-ChildItem -Path $stagingDir -Recurse -File | Where-Object {
        $_.Name -match '\.(key|pem|p12|pfx|pdb|log|env)$' -or
        $_.Name -eq "default_log.db" -or
        $_.Name -eq "station_config.json" -or
        $_.FullName -match '\\\.git\\'
    }
    if ($forbiddenFiles) {
        throw "BLOKER: W stagingu wykryto zabronione pliki: $($forbiddenFiles.FullName -join ', ')"
    }
    Write-Host "[OK] Staging zweryfikowany pomyslnie (brak zakazanych plikow)."

    # 8. Uruchomienie kompilacji WiX Toolset przez MSBuild
    $wixProj = Join-Path $repoRoot "installer\windows\msi\SPLogbook.wixproj"
    if (-not (Test-Path $wixProj)) {
        throw "BLOKER: Brak pliku projektu WiX: $wixProj"
    }

    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    $outputMsiName = "SPLogbook-Windows-x64-MSI-$displayVersion.msi"
    $outputMsiPath = Join-Path $OutputDir $outputMsiName

    Write-Host "Kompilacja i walidacja pakietu MSI przy uzyciu WiX SDK 7.0.1..."
    $buildLog = Join-Path $OutputDir "wix_msi_build.log"

    $msbuildArgs = @(
        "-restore",
        $wixProj,
        "-p:Configuration=Release",
        "-p:Platform=x64",
        "-p:Platforms=x64",
        "-p:InstallerPlatform=x64",
        "-p:OutputName=$("SPLogbook-Windows-x64-MSI-" + $displayVersion)",
        "-p:StagingDir=$stagingDir",
        "-p:MsiProductVersion=$msiProductVersion",
        "-p:DisplayVersion=$displayVersion",
        "-p:BuildCommit=$gitCommit",
        "-p:AcceptEula=true",
        "-p:OutputPath=$OutputDir\",
        "-v:normal",
        "-fl",
        "-flp:logfile=$buildLog;verbosity=detailed"
    )

    $process = Start-Process -FilePath $approvedMsbuildPath -ArgumentList $msbuildArgs -NoNewWindow -PassThru -Wait
    if ($process.ExitCode -ne 0) {
        throw "BLOKER: Kompilacja MSI zakonczona kodem bledu $($process.ExitCode). Zobacz log: $buildLog"
    }

    if (-not (Test-Path $outputMsiPath)) {
        # Sprawdz czy plik trafil do bin/x64/Release
        $altMsi = Join-Path $repoRoot "installer\windows\msi\bin\x64\Release\$outputMsiName"
        if (Test-Path $altMsi) {
            Copy-Item -Path $altMsi -Destination $outputMsiPath -Force
        } else {
            throw "BLOKER: Brak wygenerowanego pliku MSI: $outputMsiPath"
        }
    }

    # 9. Weryfikacja rozmiaru i SHA-256
    $fileItem = Get-Item $outputMsiPath
    $fileHash = (Get-FileHash -Path $outputMsiPath -Algorithm SHA256).Hash
    Write-Host "=================================================================="
    Write-Host "SUKCES BUDOWY PAKIETU WINDOWS MSI"
    Write-Host "Plik: $($fileItem.FullName)"
    Write-Host "Rozmiar: $($fileItem.Length) bajtow"
    Write-Host "SHA-256: $fileHash"
    Write-Host "Podpis: UNSIGNED TEST ARTIFACT (zgodnie z polityka braku publikacji)"
    Write-Host "=================================================================="

    return @{
        MsiPath = $outputMsiPath
        Size    = $fileItem.Length
        Sha256  = $fileHash
        Version = $displayVersion
    }
}
finally {
    if (Test-Path $stagingDir) {
        Remove-Item -Path $stagingDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
