# NASTĘPNA BRAMKA PROJEKTU

**Dokument ID:** DSH-SPLOGBOOK-NEXT-001
**Data:** 2026-11-09
**Zasada:** ten plik wskazuje **dokładnie jedną** następną bramkę. Żadna inna bramka nie może być
w tym miejscu wymieniona jako „następna”.

---

## Status poprzednich bramek

| Bramka | Status |
|---|---|
| `STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE` | **`COMPLETED_DETERMINISTIC_HERMETIC_BASELINE_VERIFIED`** — 2 700 zarejestrowanych / 2 690 wykonanych / 0 failed / 10 HIL; 3 pełne przebiegi + 20 iteracji bez flaky; `dist/` nie wpływa na baseline; E2E tylko opt-in |
| `STAGE-DEPENDENCY-SECURITY-UPDATE-GATE` | **`COMPLETED_DEPENDENCY_SECURITY_UPDATE_VERIFIED`** — `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite 3.54.1), `bundled` zachowany, bez `--ignore`; `cargo audit` 0 podatności |
| `STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE` | **`COMPLETED_POLICY_ALIGNMENT_VERIFIED`** — minimum **Linux 7.0 / glibc 2.43**, 4 wspierane dystrybucje, Debian 13.7 i Leap 16.1 jawnie niewspierane, macierz CI wyrównana, strażnicy zieloni |
| `STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE` | **`COMPLETED_POST_CLOSURE_CONSISTENCY_VERIFIED`** — domknięta **jedną sesją** po zatrzymaniu drugiej sesji; klasyfikacja drzewa (tracked / untracked / ignored / strict), sprostowanie 7 profili (run `42518873662`), potwierdzony fałszywy PASS na 4 profilach (run `42638736785`), formalne zdefiniowanie bramki F-03 |

Raporty: `STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`,
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`,
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`.

## Polityka Linux (obowiązująca, zapisana w repozytorium)

| Element | Wartość |
|---|---|
| Decyzja o kernelu | `OWNER-M3-F-KERNEL-POLICY-OPTION-A` (dynamiczny floor) |
| Minimum kernela | **Linux 7.0** (Ubuntu 26.04.1 LTS) |
| Minimum glibc | **2.43** (`GLIBC_2.43`) |
| Wspierane dystrybucje | Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed |
| Niewspierane (potwierdzone) | Debian 13.7 (glibc 2.41), openSUSE Leap 16.1 (glibc 2.40), Ubuntu interim 26.10 |
| Snapshot | `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}` |

Zasada rewizji: minimum jest przeliczane przy każdym wydaniu i przy zmianie domyślnego kernela
wspieranej dystrybucji; nie wolno go podnosić bez dowodu technicznego. Kanały testing/Rawhide/
backports/RC nie wyznaczają minimum.

## Następna bramka (jedyna wykonywalna)

NEXT_GATE: **`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`**
NEXT_GATE_STATUS: `NOT_STARTED`
NEXT_GATE_OWNER_GO: `REQUIRED`
M3_G_STATUS: `BLOCKED_BY_F03`

# `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`

**Status:** `OPEN` / `NOT_STARTED` — **nie rozpoczęto**; wymaga osobnego promptu/GO od OWNER.
**Priorytet:** P1.
**Podstawa w repozytorium (nie nowa decyzja):** `scripts/verify_linux_container.sh:4-15`
(jawny marker „LEGACY NON-FAIL-CLOSED”), `security_guard.rs:5173-5179` (strażnik wymaga
markera, nie fail-closed), `docs/DEEPSEEK_HARNESS_HANDOFF.md:114` („OPEN — wymaga
CI-hardening przed M3-G”), `docs/PROJECT_CURRENT_STATE.md` §5 (dowód F-03) i §6 (co nie jest
zrobione), `AGENTS.md` §5 pkt 26 (zakaz deklarowania natywnego PASS),
`LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json:47-51`
(`native_evidence.status = LEGACY_NON_FAIL_CLOSED_NOT_REVERIFIED`).

**Dlaczego poprzedza M3-G:** M3-G nie może być zweryfikowane natywnie na macierzy 4 dystrybucji,
dopóki weryfikator przepuszcza niezaładowany ELF. Bez fail-closed każdy przyszły „PASS” M3-G
na Debian/Leap-class ABI byłby fałszywy. Kolejność F-03 → M3-G jest już zapisana w czterech
dokumentach; ta bramka nadaje jej identyfikator, status i kryteria — **nie zmienia kolejności
etapów M3 i nie otwiera M3-G**.

**Zakres:**
1. Instalacja wymaganych bibliotek runtime w **każdym** z 4 przypiętych obrazów OCI przed
   uruchomieniem weryfikatora (lista z `build-and-release.yml:304-321`: `libasound2`, `libX11`,
   `libxcursor`, `libxrandr`, `libxi`, `libxkbcommon`, `libxkbcommon-x11`, `libgl1`,
   `libfontconfig1`, `libwayland`, `libgtk-3`, `libusb-1.0`, `libudev`), instalacja fail-closed.
2. `scripts/verify_linux_container.sh`: usunięcie `|| true` z progów ELF/uruchomienia
   (linie 46, 49, 50) i zamiana ich na asercje.
3. Jawna asercja ABI: maksymalny wymagany symbol `GLIBC_2.43` weryfikowany z artefaktu
   (`readelf --version-info` / `objdump -T`), fail-closed.
4. Asercja tożsamości: `--version` = `1.3.0-alpha.1`, `--build-info` = commit budujący;
   SHA-256 artefaktu z joba `build-linux` **tego samego** runu.
5. Strażnik w `tests/security_guard.rs`: zakaz `|| true` w weryfikatorze i wymóg asercji glibc.
6. Dowód: jeden przebieg na 4 profilach, wynik per profil, logi jako artefakty, zapisane
   `run_id` i `head_sha`; brak publikacji release.

**Kryteria akceptacji:** wszystkie 4 profile `PASS` przy niezaładowanym/zepsutym artefakcie
→ `FAIL` (test negatywny), polityka 7.0/2.43 bez zmian, brak zmian `src/`, `Cargo.toml`,
`Cargo.lock`, brak tagu i release.

**Czego nie wolno:** mechanicznie usuwać `|| true` bez instalacji bibliotek (niekontrolowane
zaczerwienienie CI), zmieniać minimum 7.0/2.43, publikować release, rozpoczynać M3-G.

**Propozycja dla OWNER:** wykonać `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE` jako
następną sesję przed M3-G. Zmiana tej kolejności wymaga jawnej decyzji OWNER.

## Bramka zablokowana do zamknięcia F-03 — M3-G (AppImage)

# `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`

**Typ:** implementacja pakietu AppImage + weryfikacja natywna.
**Priorytet:** P1.
**Status:** `BLOCKED` / `NOT_STARTED` — **nie rozpoczęto**. Blokada: otwarta bramka F-03
(`STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`, `OPEN`) oraz brak osobnego promptu/GO
od OWNER.
**Prekondycja:** zamknięta bramka QA, bramka zależności, bramka wyrównania polityki Linux
**oraz** bramka F-03.

Kontrakt wejściowy dla M3-G: minimum **Linux 7.0 / glibc 2.43**, cztery wspierane dystrybucje
z sekcji powyżej, jeden kanoniczny artefakt, layout `SPLogbook/`, aktualizacje
`JOURNALED_TRANSACTIONAL_REPLACEMENT`, XDG Base Directory.

## M3-H/J

**`NOT_STARTED`** — DEB/RPM/Flatpak po M3-G.
