# AGENTS.md — SPLogbook: zasady operacyjne dla agentów AI

> Ten plik jest zwięzłym, obowiązkowym kontraktem dla każdej sesji agentowej
> (DeepSeek Harness i następców). Nie powtarza raportów — wskazuje, co wolno,
> czego nie wolno i gdzie jest źródło prawdy.

## 1. Data projektu

1. Aktualna data projektu: **2026-11-09** (data hosta zgodna; potwierdzona 2026-11-09).
   Daty projektu **nie wolno cofać**. Jeśli host wskazuje datę późniejszą — przyjmij ją
   i zapisz aktualizację osi czasu. Nie zmieniaj zegara systemowego ani dat commitów.

## 2. Git — zawsze najpierw fetch

2. Przed każdą pracą: `git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin`.
3. Jeśli `schannel` zawodzi (`SEC_E_NO_CREDENTIALS`), używaj **per-command** `-c http.sslBackend=openssl`;
   nie zmieniaj globalnej konfiguracji Git.
4. Stan drzewa ustalaj przez `git -c core.fsmonitor=false -c core.untrackedCache=false status --porcelain=v1`
   lub `git ls-files -o --exclude-standard` (zwykłe `git status` potrafi zgłosić fantomowy `scratch/`).
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

## 5. Polityka Linux (wiążąca intencja OWNER, jeszcze nie w repozytorium)

22. Minimum kernela właściciela: **Linux 7.2** (`OWNER_APPROVED_FIXED_MINIMUM`).
23. Minimum glibc właściciela: **2.43**; maksymalny wymagany symbol `GLIBC_2.43`.
24. Polityka Linux **nie jest jeszcze zapisana w repozytorium** (repo nadal koduje 6.12 / Debian /
    Leap). Wymaga osobnej bramki `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`
    **po** zamknięciu QA. Nie „poprawiaj” snapshotu przy okazji innych prac.
25. **M3-G (AppImage) pozostaje ZABLOKOWANE** do zamknięcia bramki QA i bramki wyrównania polityki.

## 6. Zasady testów i instalatorów

- Standardowy baseline **nie może** zależeć od `dist/` ani od żadnego nieśledzonego artefaktu.
- Natywny E2E instalatora jest **jawnym opt-in** (`SPLOG_RUN_INNO_E2E=1` /
  `SPLOG_RUN_MSI_E2E=1` + jawna ścieżka); brak opt-in to jawnie niewykonany fixture,
  nie natywny PASS.
- Przy opt-in: brak artefaktu, zła wersja, niezerowy exit code, brak logu lub brak dowodu
  instalacji/deinstalacji = **FAIL-CLOSED**. Nigdy nie ignoruj exit code, nigdy `|| true`,
  nigdy pusty `return` jako ukryty skip.
- Nie zmieniaj testu tylko po to, by uzyskać zielony wynik. Nie osłabiaj asercji bezpieczeństwa.
- Nie zmieniaj `src/` produkcyjnie, `Cargo.toml`, `Cargo.lock` bez osobnej, uzasadnionej bramki.

## 7. Zależności i bezpieczeństwo dostawcze

- Aktualizacje zależności wyłącznie w dedykowanej bramce bezpieczeństwa, nigdy „przy okazji”.
- `rusqlite` musi pozostać z feature `bundled` (systemowe SQLite jest zabronione).
- **Zakaz `cargo audit --ignore`** bez formalnej decyzji OWNER; nowa podatność = STOP i raport.
- Lockfile zmieniaj wyłącznie przez cargo (`cargo update -p <pkg> --precise <ver>`), nigdy ręcznie.
- Push dopiero po zielonym `cargo audit` i pełnym baseline.

## 8. Raportowanie

Każda bramka kończy się raportem: data/środowisko, Git przed/po, lista commitów, zmienione pliki,
dokładne komendy i liczniki, findings z `file:line`, ryzyka rezydualne i **jedna** następna bramka.
Faza analityczna nie tworzy commitów. Publikacja wyłącznie po zielonym baseline i osobnej bramce.
