# AGENTS.md — SPLogbook: zasady operacyjne dla agentów AI

> Ten plik jest zwięzłym, obowiązkowym kontraktem dla każdej sesji agentowej
> (DeepSeek Harness i następców). Nie powtarza raportów — wskazuje, co wolno,
> czego nie wolno i gdzie jest źródło prawdy.

## 1. Data projektu

1. Aktualna data projektu: **2026-11-13**; **oś czasu zaktualizowana 2026-11-14** — host wskazał
   późniejszą datę, którą przyjęto zgodnie z zasadą poniżej. Data 2026-11-09 pozostaje
   **historyczną** datą wcześniejszych bramek. Daty projektu **nie wolno cofać**.
   Jeśli host wskazuje datę późniejszą — przyjmij ją i zapisz aktualizację osi czasu.
   Nie zmieniaj zegara systemowego ani dat commitów.

## 2. Git — zawsze najpierw fetch

2. Przed każdą pracą: `git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin`.
3. Jeśli `schannel` zawodzi (`SEC_E_NO_CREDENTIALS`), używaj **per-command** `-c http.sslBackend=openssl`;
   nie zmieniaj globalnej konfiguracji Git.
4. Stan drzewa ustalaj przez `git -c core.fsmonitor=false -c core.untrackedCache=false status --porcelain=v1`
   **oraz** `git ls-files -o --exclude-standard` i rozróżniaj cztery stany: (a) zmiany śledzone,
   (b) pliki nieśledzone, (c) pliki ignorowane, (d) czystość ścisła. `scratch/` jest katalogiem
   lokalnym i jest ignorowany **tylko wzorcami plików** (`.gitignore:14` `*.log`, `.gitignore:52`
   `*.py`) — plik niepasujący do wzorca jest realnie nieśledzony i daje `?? scratch/` w każdym
   trybie, także z `-c core.fsmonitor=false -c core.untrackedCache=false` (stan zmierzony
   2026-11-09: `scratch/tree_dupes.txt` → `?? scratch/`). F-25 (nieświeży cache)
   wyjaśnia wyłącznie historyczny fantom i **nie zastępuje** weryfikacji `git ls-files -o`.
   Nie nazywaj stanu `CLEAN` bez podania definicji; `CLEAN` bez kwalifikatora oznacza czystość
   ścisłą (brak zmian śledzonych, nieśledzonych i ignorowanych).
5. Punktem odniesienia jest **`origin/main` po fetch**. Kod z `origin/main` jest źródłem stanu faktycznego.
6. Synchronizacja wyłącznie **fast-forward** (`git merge --ff-only`, `git pull --ff-only`).
7. **Zakaz rebase.**
8. **Zakaz zwykłego merge** (tworzącego merge commit).
9. **Zakaz force-push** i resetowania historii.
10. **Ochrona `v1.2.0`**: tag obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`,
    peeled commit `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592`. Nigdy nie twórz, nie przenoś,
    nie usuwaj tagów.
11. **Brak publikacji release** i brak pushowania assetów bez osobnej bramki.

## 3. Bezpieczeństwo fizyczne i dane

12. **Nie wykonuj HIL, PTT ani emisji RF.** Testy `#[ignore = "hardware-in-the-loop"]`
    (10 sztuk w `tests/cat_native_7.rs`) pozostają nieuruchamiane.
13. **Nie otwieraj produkcyjnych baz** ani konfiguracji użytkownika.
14. **Nie używaj produkcyjnego `HOME`/`APPDATA`**; testy instalatorów muszą izolować profil.

## 4. Źródła prawdy i kolejność pracy

15. Przed zmianą przeczytaj powiązane dokumenty: `docs/PROJECT_CURRENT_STATE.md`,
    `docs/NEXT_WORK_GATE.md`, właściwy DESIGN/OWNER, raport bramki i testy strażnicze.
16. **Kod jest źródłem stanu faktycznego.** Dokument nie nadpisuje kodu; komentarz nie nadpisuje wykonania.
17. **Decyzje OWNER są źródłem zatwierdzonej intencji** — jeśli dokument i kod są sprzeczne,
    opisz konflikt, nie zgaduj.
18. **Ukończonych etapów nie otwieraj bez potwierdzonej, odtwarzalnej regresji.**
19. **Baseline musi być zielony** przed i po każdej zmianie (lista komend w `docs/DEEPSEEK_HARNESS_HANDOFF.md` §5).
20. **Test modelowy nie jest natywnym E2E.** Testy kontenerowe/modelowe nie są dowodem zachowania
    dystrybucji ani jądra.
21. **Kontener nie jest dowodem kernela dystrybucji** (współdzieli jądro hosta); `uname -r` w OCI
    nie jest dowodem natywnym.

## 5. Polityka Linux (zapisana w repozytorium)

22. Minimum kernela: **Linux 7.0** — decyzja `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (model
    dynamiczny: najstarsza domyślna linia kernela wśród oficjalnie wspieranych dystrybucji;
    obecnie wyznacza ją Ubuntu 26.04 LTS). Decyzja zastępuje zarówno dynamiczny floor `6.12`,
    jak i sztywne `7.2` (`OWNER_APPROVED_FIXED_MINIMUM`).
23. Minimum glibc: **2.43**; maksymalny wymagany symbol `GLIBC_2.43`
    (`OWNER-M3-F-GLIBC-POLICY-01`).
24. Wspierane dystrybucje: **Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed**.
    Debian 13.7 (glibc 2.41) i openSUSE Leap 16.1 (glibc 2.40) są **niewspierane** — nie spełniają
    wymagania ABI; Ubuntu interim (26.10) jest poza zakresem. Snapshot:
    `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`. Politykę wolno zmienić wyłącznie nową,
    jawną decyzją OWNER i osobną bramką. Kontrakt DEB: Ubuntu 26.04 LTS (glibc 2.43), układ
    `/opt/SPLogbook` + launcher `/usr/bin/splogbook` (`OWNER-M3-H-03`).
25. **M3-G (AppImage) — ZAMKNIĘTE 2026-11-12**: **`COMPLETED_APPIMAGE_INTEGRATION_VERIFIED`**
    (run `43166825054`: 4/4 profile `success`, 12/12 mutacji odrzuconych, FUSE `VERIFIED`;
    `CI` `43166825036` = `success`; `Publish Release Assets` = `skipped`). Bloker
    `RUSTSEC-2026-0486` usunięty decyzją `OWNER-DEP-RUSTIX-01` (`rustix 1.1.6`).
    **M3-H (DEB) — ZAMKNIĘTE 2026-11-12**: **`COMPLETED_DEB_INTEGRATION_VERIFIED`** (run
    `43245181565`: `Build DEB`, `Verify DEB`, `Verify DEB (negative fixtures)` 27/27 = `success`;
    Windows/Linux/AppImage = `success`; `Publish Release Assets` = `skipped`; `CI`
    `43245181469` = `success`). **M3-I (RPM) — ZAMKNIĘTE 2026-11-14**: **`COMPLETED_RPM_INTEGRATION_VERIFIED`**.
    Historia: 2026-11-13 bramka była `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES` —
    implementacja i weryfikacja RPM = `success` (run `43453322341`), ale `CI`
    `43453322339` = `failure` na `RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`
    i `RUSTSEC-2026-0538`. Odblokowanie: `STAGE-M3-I-SEC` =
    `COMPLETED_RUSTSEC_REMEDIATION_VERIFIED` — `quinn-proto 0.11.19`,
    `quinn-udp 0.5.16`, `cargo audit` EXIT 0 bez nowych `--ignore`,
    `Cargo.toml`/`src/**` bez zmian; `CI` `43485657304` = `success`,
    `Build and Release` `43485657283` = `success`, `Publish Release Assets`
    = `skipped`. M3-J (Flatpak) pozostaje **`NOT_STARTED`** i wymaga
    `OWNER_GO_REQUIRED`.
26. **F-03 (P1) — ZAMKNIĘTE 2026-11-09** (`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`,
    `COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`). Weryfikator
    `scripts/verify_linux_container.sh` jest fail-closed: ELF, `ldd`, `--version`, `--build-info`
    i ABI `GLIBC_2.43` są asercjami, a `NATIVE VERIFICATION COMPLETED SUCCESSFULLY` jest
    wypisywany wyłącznie po wszystkich kontrolach. Biblioteki runtime są instalowane z
    oficjalnego menedżera pakietów w każdym z 4 przypiętych obrazów OCI **przed** weryfikacją
    (`scripts/install_linux_runtime_deps.sh`). Dowód: run `42685105375` na commicie `abdb767`,
    4/4 profile `success`, zero `not found` w logach, `--version` i `--build-info` exit 0,
    wymagany GLIBC ≤ 2.43, oraz job negatywny (brak biblioteki → exit 1, bez komunikatu
    sukcesu). **Nie deklaruj natywnego PASS bez tych dowodów.** M3-G jest `CLOSED`
    (2026-11-12); F-03 pozostaje `CLOSED` i nie może zostać otwarte ponownie.

## 6. Zasady testów i instalatorów

- Standardowy baseline **nie może** zależeć od `dist/` ani od żadnego nieśledzonego artefaktu.
- Natywny E2E instalatora jest **jawnym opt-in** (`SPLOG_RUN_INNO_E2E=1` /
  `SPLOG_RUN_MSI_E2E=1` + jawna ścieżka); brak opt-in to jawnie niewykonany fixture,
  nie natywny PASS.
- Przy opt-in: brak artefaktu, zła wersja, niezerowy exit code, brak logu lub brak dowodu
  instalacji/deinstalacji = **FAIL-CLOSED**. Nigdy nie ignoruj exit code, nigdy `|| true`,
  nigdy pusty `return` jako ukryty skip.
- Weryfikacja natywna Linux jest **fail-closed**. Maskowanie `ldd`, `--version` lub
  `--build-info` przez `|| true` jest zabronione; strażnik
  `m3_f_f03_fail_closed_verification_guard` musi pozostać zielony, a każdy profil musi
  instalować biblioteki runtime przed uruchomieniem weryfikatora.
- Nie zmieniaj testu tylko po to, by uzyskać zielony wynik. Nie osłabiaj asercji bezpieczeństwa.
- Nie zmieniaj `src/` produkcyjnie, `Cargo.toml`, `Cargo.lock` bez osobnej, uzasadnionej bramki.

## 7. Zależności i bezpieczeństwo dostawcze

- Aktualizacje zależności wyłącznie w dedykowanej bramce bezpieczeństwa, nigdy „przy okazji”.
- `rusqlite` musi pozostać z feature `bundled` (systemowe SQLite jest zabronione).
- **Zakaz `cargo audit --ignore`** bez formalnej decyzji OWNER; nowa podatność = STOP i raport.
- Lockfile zmieniaj wyłącznie przez cargo (`cargo update -p <pkg> --precise <ver>`), nigdy ręcznie.
- Push dopiero po zielonym `cargo audit` i pełnym baseline.

## 8. Raportowanie

- Pracę **zapisującą** nad repozytorium prowadzi **dokładnie jedna sesja** (jedno worktree, jedno
  `target/`, jeden zestaw raportów). Równoległa druga sesja musi zostać zatrzymana **przed**
  testami, commitem i pushem; brak potwierdzenia zatrzymania = `BLOCKED`, bez wyjątków.
  Nie uruchamiaj dwóch poleceń Cargo korzystających z tego samego `target/` (na tym hoście
  obowiązuje `-j 1` — limit pliku stronicowania, `os error 1455`).

Każda bramka kończy się raportem: data/środowisko, Git przed/po, lista commitów, zmienione pliki,
dokładne komendy i liczniki, findings z `file:line`, ryzyka rezydualne i **jedna** następna bramka.
Faza analityczna nie tworzy commitów. Publikacja wyłącznie po zielonym baseline i osobnej bramce.
