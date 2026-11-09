# DEEPSEEK HARNESS — WYMAGANE ZDOLNOŚCI ŚRODOWISKA

**Dokument ID:** DSH-SPLOGBOOK-CAP-001
**Data:** 2026-11-09
**Status:** OPERACYJNY — wymagania dla bieżącego i przyszłych harnessów
**Powiązane:** `AGENTS.md`, `docs/DEEPSEEK_HARNESS_HANDOFF.md`

Legenda: **M** = wymagane (mandatory), **O** = opcjonalne, **X** = niedozwolone w tym środowisku.

| ID | Zdolność | Cel | Wymagane? | Substytut lokalny | Uprawnienia | Ryzyko |
|---|---|---|---|---|---|---|
| C-01 | Ekspozycja wersji harnessu i modelu w nagłówku sesji | powtarzalne nagłówki audytu | O | odczyt `DSH_*` / deklaracja zadania | odczyt env | brak |
| C-02 | Wyliczenie pluginów/MCP w sesji | inwentarz zdolności | O | `plugin_manager list_plugins` | odczyt | brak |
| C-03 | Backend TLS działający w sandboxie **lub** gotowy `-c http.sslBackend=openssl` | `git fetch`, `ls-remote` | **M** | `-c http.sslBackend=openssl` per-command (zweryfikowane) | brak | brak |
| C-04 | Odczyt GitHub API / logów CI (token lub `gh`) | weryfikacja runów CI (np. `42518873662`) | O | poleganie na raportach w repo (słabsze) | repo read | obsługa poświadczeń |
| C-05 | Natywny target Linux (WSL2 / agent) | ponowna weryfikacja kernela/glibc i artefaktu ELF | **M dla M3-G+** | zdalne CI (kontener ≠ dowód jądra) | wykonanie kodu | kontener nie jest dowodem kernela |
| C-06 | Fizyczny bench HIL | NATIVE-7B | **X** — poza zakresem; wymaga zgody OWNER | pozostaje `BLOCKED` | emisja RF | **niedopuszczalne bez zgody** |
| C-07 | Watcher długich testów (streaming) | uniknięcie „ślepych” kompilacji | O | background job + odczyt metadanych | brak | brak |
| C-08 | Publikacja deliverable (`present`) | karty plików dla użytkownika | O | linki w odpowiedzi końcowej | filesystem | brak |
| C-09 | `cargo audit` z zapisywalną bazą advisory | bramka podatności | **M** | `cargo audit --db <katalog tymczasowy>` (zweryfikowane) | zapis do temp | brak |
| C-10 | **Nowe (też udokumentowane w raporcie QA):** możliwość uruchomienia instalatora Inno w Medium Integrity Level | natywny E2E instalatora | **M dla natywnego E2E**; **niedostępne w sandboxie DSH (Low IL)** | opt-in E2E na runnerze Windows CI; w sandboxie test jest jawnie niewykonany | zapis HKCU + Menu Start użytkownika testowego | brak, jeśli izolowane `USERPROFILE` |

## Zasady użycia

- **C-06 (HIL) nie może być symulowane.** Brak sprzętu = `BLOCKED`, nie „test zastępczy”.
- **C-10:** sandbox DSH działa w Low Mandatory Integrity Level (`S-1-16-4096`); zapisy do
  `HKCU` i Medium-IL Menu Start są odrzucane (`0x80070005`), mimo `FullControl` w ACL.
  Natywny E2E instalatora w tym środowisku **musi** być opt-in i **musi** fail-closed.
- **C-05:** dowolny test w kontenerze OCI raportuje jądro **hosta**, nie dystrybucji.
  Nie wolno na tej podstawie twierdzić „zweryfikowano kernel dystrybucji”.

## Luki bieżącej sesji (2026-11-09)

| Luka | Wpływ | Obejście |
|---|---|---|
| C-06 (HIL) niedostępne | NATIVE-7B `BLOCKED` | brak — wymagany sprzęt i zgoda OWNER |
| C-10 (Medium IL) niedostępne | natywny E2E Inno nie przejdzie w sandboxie | opt-in + runner Windows CI; w sandboxie jawny skip fixture |
| C-05 (Linux) niedostępne | brak ponownej weryfikacji natywnej M3-F | zdalne CI; bramka polityki Linux i tak wymaga decyzji OWNER |
| C-04 (logi CI) niedostępne | brak weryfikacji runu `42518873662` | dowody z raportów w repo |
