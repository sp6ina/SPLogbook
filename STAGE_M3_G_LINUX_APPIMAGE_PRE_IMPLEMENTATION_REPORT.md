# STAGE-M3-G — RAPORT PRZEDIMPLEMENTACYJNY (AppImage, Linux x86_64)

**Dokument ID:** DSH-SPLOGBOOK-M3G-PRE-001
**Bramka:** `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`
**Aktualna data projektu:** **2026-11-11**
**Strefa czasowa pomiaru:** `+01:00` (Europe/Warsaw)
**Status raportu:** `ACCEPTED_BY_OWNER` (decyzje `OWNER-M3-G-01`, `OWNER-M3-G-02`,
`OWNER-M3-G-05`) + sekcja korygująca `M3-G APPIMAGETOOL CONTINUOUS RESOLUTION UPDATE`
(decyzje `OWNER-M3-G-03-R1`, `OWNER-M3-G-04-R1`, `OWNER-M3-G-08`, `OWNER-M3-G-09`)
**Status bramki w chwili zapisu:** `M3_G_STATUS=IN_PROGRESS` (GO OWNER udzielone)

> Ten raport **nie tworzy** konkurencyjnego źródła prawdy. Hierarchia źródeł:
> aktualny `origin/main` → kod/skrypty/workflowy → `AGENTS.md` → decyzje OWNER →
> DESIGN → `PROJECT_CURRENT_STATE.md` → `NEXT_WORK_GATE.md` → plan → macierz →
> raporty. Kod rozstrzyga stan faktyczny, DESIGN rozstrzyga architekturę.

---

## 1. Wejście bramki (rekonstrukcja z 2026-11-11)

| Pole | Wartość |
|---|---|
| Repozytorium | `sp6ina/SPLogbook` (`origin = https://github.com/sp6ina/SPLogbook.git`) |
| Gałąź | `main` |
| `HEAD` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `main` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `origin/main` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `ahead/behind` | `0 / 0` |
| Oczekiwany commit wejściowy | **POTWIERDZONY** (`git cat-file -t` = `commit`, `merge-base --is-ancestor` = 0) |
| `main...origin/main` po `fetch --all --tags --prune` | bez zmian (`0 / 0`) |
| Tag `v1.2.0` | obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — nietknięty |
| Tagi `v1.3.0*` | **0** |
| Stash | 2 wpisy (`On repair/full-system-remediation`) — **nietknięte**, nie zastosowane, nie usunięte |
| Submoduły | brak (`.gitmodules` nie istnieje) |
| `TRACKED_WORKTREE_CLEAN` | **TRUE** (`git diff --check` = 0) |
| `UNTRACKED_FILES_PRESENT` | **TRUE** — `?? scratch/` (`scratch/audit/…`, `scratch/ci/…`, `scratch/tree_dupes.txt`, dowody wcześniejszych bramek) |
| `IGNORED_FILES_PRESENT` | **TRUE** (`target/`, `dist/`, `SPLogbook-backup/`, logi i skrypty `*.log`/`*.py` w `scratch/`) |
| `STRICT_WORKTREE_CLEAN` | **FALSE** |
| Jedna aktywna sesja | **POTWIERDZONA** — brak procesów `git`/`cargo`/`rustc`, brak `.git/index.lock`, brak obcej blokady `target/` |
| `scratch/` | katalog **lokalny**, nieproduktowy; nie usuwany, nie commitowany, nie dodawany przez `git add .` |

`scratch/` nie jest ignorowany jako katalog — ignorowane są wyłącznie wzorce plików
(`.gitignore:14` `*.log`, `.gitignore:52` `*.py`). Plik niepasujący do wzorca daje realnie
`?? scratch/`. To nie fantom F-25.

## 2. Potwierdzenie tożsamości bramki

| Warunek | Wartość | Wynik |
|---|---|---|
| `F03_STATUS` | `CLOSED` | **POTWIERDZONE** |
| `NEXT_GATE` | `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE` | **POTWIERDZONE** (`docs/NEXT_WORK_GATE.md:62`) |
| `NEXT_GATE_STATUS` | `NOT_STARTED` | **POTWIERDZONE** |
| `M3_G_STATUS` wejściowo | `READY_FOR_OWNER_GO` | **POTWIERDZONE** |
| M3-G jest następną bramką | tak — M3-H/J oznaczone `NOT_STARTED` „po M3-G” | **POTWIERDZONE** |

Dokumenty odczytane w całości lub w zakresie wymaganym: `AGENTS.md`,
`docs/PROJECT_CURRENT_STATE.md`, `docs/NEXT_WORK_GATE.md`, `MASTER_IMPLEMENTATION_PLAN.md`,
`docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/DOCUMENTATION_INDEX.md`,
`docs/DEEPSEEK_HARNESS_HANDOFF.md`, `DESIGN_3_0_B_LINUX_UPDATE.md`,
`DESIGN_3_0_C_PLATFORM_INSTALLERS.md`, `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{md,json}`,
raporty F-03 (wstępny i zamknięcia), raporty portable tar.gz oraz native evidence,
raport QA baseline, raport zależności, `scripts/build_linux_portable_tar_gz.sh`,
`scripts/verify_linux_container.sh`, `scripts/install_linux_runtime_deps.sh`,
`.github/workflows/build-and-release.yml`, `.github/workflows/ci.yml`, `tests/security_guard.rs`,
testy portable tar.gz / multi-distro / archive security / XDG, `assets/icon.png`,
`assets/splogbook.desktop`, `assets/run.sh`, `installer/linux/portable/templates/splogbook.desktop`,
`src/core/xdg.rs`, `src/main.rs`, `Cargo.toml`, `Cargo.lock`, `.gitignore`.

## 3. Zatwierdzony DESIGN i granice zakresu

- `DESIGN_3_0_B_LINUX_UPDATE.md` §4.2/§6/§8: AppImage Type 2, `package_type`
  `linux-appimage`, asset `SPLogbook-Linux-x86_64.AppImage`, transakcyjna podmiana obrazu
  (`0o755`, `.rollback`), dane wyłącznie w XDG, „AppImage nie może być traktowane jak tar.gz”.
- `DESIGN_3_0_C_PLATFORM_INSTALLERS.md`: `appimagetool` jako narzędzie budowy; adapter
  `SelfManaged`; model `InstallationType::LinuxAppImage`; `linux-appimage` w tabeli pakietów.
- Polityka Linux (niezmieniona): kernel **7.0**, glibc **2.43**, cztery wspierane dystrybucje.
- DESIGN **nie definiuje**: dokładnego schematu markera AppImage, layoutu AppDir ani pinu
  konkretnego release type-2 runtime. Dlatego stosowany jest kontrakt minimalny z promptu
  bramki (`§13–§14`), a nie nowy, konkurencyjny schemat.
- Poza zakresem: publiczny release, tag, `v1.3.0`, manifest release, `ReleasePackageType`,
  API updatera, `src/**`, `Cargo.toml`, `Cargo.lock`, schemat SQLite, dane użytkownika.

## 4. Polityka „najnowsze zweryfikowane narzędzia”

Trwały wymóg: **użyj najnowszego oficjalnego Continuous build `appimagetool`, zweryfikuj
pochodzenie, zapisz commit i SHA-256, przypnij na czas bramki, zbuduj i przetestuj tym
dokładnie plikiem.** Konkretny commit i SHA-256 są dowodem **bieżącej** bramki, a nie wiecznym
pinem projektu.

---

# M3-G APPIMAGETOOL CONTINUOUS RESOLUTION UPDATE

Sekcja wykonana **przed pierwszą edycją kodu i workflow** (2026-11-11), zgodnie z §8 promptu.

## 5. Wynik ponownego fetch i stan Git (2026-11-11)

| Pole | Wartość |
|---|---|
| `git fetch --all --tags --prune` | EXIT 0 |
| `HEAD` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `main` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `origin/main` | `81db60c99d73472f02e94b30f1f908ae180f39e0` |
| `git rev-list --left-right --count main...origin/main` | `0	0` |
| Jedna sesja | potwierdzona (brak równoległych procesów) |
| `F03_STATUS` | `CLOSED` |
| `M3_G_STATUS` | `READY_FOR_OWNER_GO` → `IN_PROGRESS` po GO OWNER |

## 6. Rozwiązanie oficjalnego Continuous build `appimagetool`

| Pole | Wartość |
|---|---|
| `APPIMAGETOOL_SOURCE` | `https://github.com/AppImage/appimagetool` (oficjalne repozytorium; **nie** `AppImageKit`) |
| `APPIMAGETOOL_CHANNEL` | `continuous` |
| `APPIMAGETOOL_RELEASE_ID` | `435944148` |
| `APPIMAGETOOL_RELEASE_NAME` | `Continuous build` |
| `APPIMAGETOOL_SOURCE_COMMIT` | `11d54c9acc7f246df1398050a4b880314ac76862` (`target_commitish`) |
| `APPIMAGETOOL_PUBLISHED_AT` | `2026-11-06T00:48:33Z` |
| `APPIMAGETOOL_ASSET` | `appimagetool-x86_64.AppImage` (dokładnie jeden pasujący asset x86_64) |
| `APPIMAGETOOL_SIZE` | `15133176` B |
| `APPIMAGETOOL_PUBLISHED_SHA256` | `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` |
| `APPIMAGETOOL_LOCAL_SHA256` | `e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03` |
| `APPIMAGETOOL_DIGEST_MATCH` | **TRUE** |
| Algorytm digestu | `sha256` (obsługiwany) |
| Build log (metadane release) | `https://github.com/AppImage/appimagetool/actions/runs/42139246819` |

Weryfikacja: metadane odczytane z oficjalnego API wydania `continuous`, asset pobrany
bezpośrednio z `browser_download_url` tego wydania, lokalny SHA-256 obliczony niezależnie —
**identyczny** z opublikowanym. Snapshot kontrolny z promptu
(`11d54c9` / `e8c1169c…`) jest **nadal aktualny**; nowszy build nie istnieje.

**Decyzja:** build `11d54c9acc7f246df1398050a4b880314ac76862`
(`sha256:e8c1169c4af66c3184d7ee110713d4be04befe671fa358e2c1591ee840553b03`, `15133176` B)
zostaje **przypięty na czas całej bramki**. Każdy job budowy używa tego samego pliku;
nie ma ponownego rozwiązywania tagu `continuous` w kolejnych jobach.

`appimagetool` **nie jest** commitowany do repozytorium (artefakt narzędziowy, nie produkt).
Plik roboczy znajduje się wyłącznie w katalogu nieśledzonym `scratch/`.

## 7. Polityka type-2 runtime

| Pole | Wartość |
|---|---|
| `RUNTIME_SOURCE` | `https://github.com/AppImage/type2-runtime` (oficjalne) |
| `RUNTIME_RELEASE_OR_COMMIT` | `continuous` → commit `fedfad51acc2860b2916a1eef32ba4a500ff31ef` |
| `RUNTIME_RELEASE_ID` | `417724209` |
| `RUNTIME_PUBLISHED_AT` | `2026-10-19T20:37:20Z` |
| `RUNTIME_ASSET` | `runtime-x86_64` |
| `RUNTIME_SIZE` | `944632` B |
| `RUNTIME_PUBLISHED_SHA256` | `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46` |
| `RUNTIME_LOCAL_SHA256` | `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46` |
| `RUNTIME_DIGEST_MATCH` | **TRUE** |
| Wcześniej zatwierdzony release | `20251108` — `runtime-x86_64`, `944632` B, `sha256:2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d` |
| `RUNTIME_DESIGN_COMPATIBLE` | **TRUE** — DESIGN nie przypina konkretnego release type-2 runtime |

**Rozstrzygnięcie polityki (wariant A):** `DESIGN_3_0_B/C` **nie** narzuca release
`20251108` (brak takiego zapisu w dokumentach zatwierdzonych). `OWNER-M3-G-04-R1` oraz
`OWNER-M3-G-09` pozwalają rozwiązać **najnowszy zweryfikowany** oficjalny runtime, dlatego
użyty jest release `continuous` @ `fedfad51`. Release `20251108` pozostaje zapisany jako
punkt odniesienia; różnica digestu przy identycznym rozmiarze jest odnotowana jawnie.
Nie występuje konflikt dokumentów, więc `BLOCKED_RUNTIME_POLICY_CONFLICT` nie ma zastosowania.

Kontrola ABI: runtime type 2 jest **statycznie linkowany** (brak `PT_INTERP`, brak
`DT_NEEDED`, brak symboli `GLIBC_*`) — nie wnosi wymagania nowszego niż `GLIBC_2.43`
i nie zmienia polityki ABI artefaktu. Runtime jest przekazywany jawnie przez `--runtime-file`;
`appimagetool` **nie** pobiera runtime w tle.

## 8. Potwierdzenia negatywne (czego NIE użyto)

- **NIE** użyto wymuszonego fallbacku do `appimagetool 1.9.1`.
- **NIE** użyto „latest” bez weryfikacji (każdy plik ma SHA-256 + commit + rozmiar).
- **NIE** użyto `curl | sh`, nieoficjalnych mirrorów ani rekonstruowanych adresów.
- **NIE** użyto `AppImageKit` ani jego przestarzałych assetów.
- **NIE** rozwiązano ponownie tagu `continuous` po przypięciu.
- **NIE** uruchomiono niezweryfikowanej binarki.

## 9. Zaktualizowany plan implementacji

1. `scripts/install_appimage_toolchain.sh` — resolve-once + weryfikacja SHA-256 + pin
   (`appimagetool` + type-2 runtime), metadane audytowe JSON, brak commitowania narzędzia.
2. `scripts/build_linux_appimage.sh` — wejście: zweryfikowany portable tar.gz z **tego samego
   commita**; weryfikacja SHA-256; bezpieczne rozpakowanie; AppDir wg zatwierdzonego layoutu;
   marker JSON; `patchelf` RUNPATH `$ORIGIN/../lib`; `appimagetool` z przypiętym
   `--runtime-file`; `.sha256`; brak publikacji, brak `-u`, brak podpisu.
3. `installer/linux/appimage/AppRun` — POSIX `sh`, `set -eu`, `exec "$APPDIR/usr/bin/SPLogbook" "$@"`.
4. `scripts/verify_appimage.sh` — fail-closed weryfikator (obraz + tryb AppDir), komunikat
   sukcesu wyłącznie na końcu.
5. `scripts/verify_appimage_negative_fixture.sh` — 12 mutacji musi zostać odrzuconych.
6. Workflow: build (AppImage + metadane), matrix (ten sam AppImage w 4 profilach),
   job negatywny, job FUSE (rzeczywisty mount albo jawna klasyfikacja braku).
7. `tests/linux_appimage_packaging_test.rs` + strażnik `m3_g_appimage_integration_guard`.
8. `scripts/install_linux_runtime_deps.sh` — `desktop-file-utils` w każdym profilu.
9. Pełny baseline lokalny, commit implementacyjny, push ff-only, analiza CI,
   commit dokumentacyjny, finalny push ff-only.

## 10. Warunki natychmiastowego zatrzymania (bramka)

Zatrzymanie i `BLOCKED` następuje, gdy: `origin/main` niedostępny lub zmieniony; nierozpoznane
zmiany śledzone; druga sesja lub obcy proces na `target/`; M3-G nie jest aktualną bramką;
F-03 nie jest `CLOSED`; metadane `continuous` niedostępne; asset niejednoznaczny; brak
SHA-256; niezgodność SHA-256; zmiana assetu między resolution a pobraniem; wymóg
nieoficjalnego źródła lub `curl | sh`; regresja najnowszego Continuous; sprzeczna polityka
runtime; ABI > 2.43; awaria portable tar.gz; utrata fail-closed F-03; dane użytkownika
w AppDir; zapis do AppDir; nieodrzucona mutacja negatywna; brak 4/4 profili; CI na innym SHA;
niepełne logi; publikacja release nie jest `skipped`; niezamierzony tag/release; konieczność
force push / `reset --hard` / `clean -fd` / `rebase`; wykryty sekret; potrzeba nowej decyzji OWNER.
