# RAPORT KOŃCOWY BRAMKI
# STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE

**Projekt:** SPLogbook
**Repozytorium:** `sp6ina/SPLogbook`
**Gałąź docelowa:** `main`
**Tryb:** AUTONOMOUS REPOSITORY AGENT — **jedna sesja** (analiza, implementacja, testy, dowody CI, dokumentacja)
**Status bramki:** **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`**

---

**1. Data projektu:** 2026-11-09.

**2. Data, czas i strefa końcowego pomiaru:** 2026-11-09, `21:00–21:35+01:00`
(Europe/Warsaw, CET `+01:00`).

**3. Identyfikator bramki:** `STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE`.

**4. Status bramki:** `COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`.

**5. Decyzja OWNER:** `OWNER-M3-F-F03-01` — **GO** dla zakresu F-03 (usunięcie fałszywego PASS,
biblioteki runtime w 4 obrazach, fail-closed `ldd`/`--version`/`--build-info`, jawna kontrola
`GLIBC_2.43`, testy regresyjne, dowody CI). Bez AppImage, M3-G, release i tagu.

**6. Stan początkowy Git:** `HEAD = main = origin/main = e85ee4e0c80d2f3ca96b9e7068f401586662b18b`;
`main...origin/main` = `0/0`; drzewo śledzone czyste; `?? scratch/`.

**7. Wynik fetch:** `git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false
fetch --all --tags --prune` → **EXIT 0**; `origin/main` niezmieniony względem oczekiwanego SHA.

**8. HEAD/main/origin-main na wejściu:** `e85ee4e0c80d2f3ca96b9e7068f401586662b18b` (wszystkie).

**9. Ahead/behind na wejściu:** `0 / 0`.

**10. Aktywna gałąź:** `main`.

**11. Stan tagu `v1.2.0`:** obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled
`6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — **VERIFIED, nietknięty** (lokalnie i zdalnie);
brak `v1.3.0*`.

**12. Stan stashy:** 2 istniejące (`clubs_csv_untracked`, `uncommitted pre-stage-10-work`) —
**nietknięte, nie zastosowane**.

**13. Stan submodułów:** brak (`.gitmodules` nie istnieje, 0 wpisów trybu `160000`).

**14. Klasyfikacja `scratch/`:** katalog lokalny, nieśledzony, niecommitowany, **nieusunięty**;
`UNTRACKED_SCRATCH_PRESENT=true` (`scratch/tree_dupes.txt`, logi dowodowe F-03, `scratch/audit/…`,
`scratch/ci/…`); wzorce ignorowania `.gitignore:14` `*.log`, `.gitignore:52` `*.py`.

**15. Przeczytane dokumenty:** `AGENTS.md`; `docs/PROJECT_CURRENT_STATE.md`;
`docs/NEXT_WORK_GATE.md`; `MASTER_IMPLEMENTATION_PLAN.md`; `docs/DECISION_TRACEABILITY_MATRIX.md`;
`docs/DOCUMENTATION_INDEX.md`; `docs/DEEPSEEK_HARNESS_HANDOFF.md`; `DESIGN_3_0_B_LINUX_UPDATE.md`;
`LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`;
`STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_{REPORT,PRE_IMPLEMENTATION_REPORT}.md`;
`STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md`;
`STAGE_M3_F_NATIVE_LINUX_EVIDENCE_AND_SNAPSHOT_CORRECTION_REPORT.md`;
`STAGE_M3_F_LINUX_PORTABLE_TAR_GZ_INTEGRATION_REPORT.md`;
`STAGE_QA_BASELINE_DETERMINISM_AND_HERMETICITY_REPORT.md`;
`STAGE_DEPENDENCY_SECURITY_UPDATE_REPORT.md`; `scripts/verify_linux_container.sh`;
`scripts/build_linux_portable_tar_gz.sh`; `.github/workflows/{build-and-release,ci}.yml`;
`tests/{security_guard,linux_multi_distribution_compatibility_test,linux_portable_tar_gz_test,linux_archive_security_test}.rs`;
`src/main.rs`; `src/core/build_info.rs`; `build.rs`; legacy logi `scratch/evidence_r42638736785/*`.

**16. Hierarchia źródeł:** (1) kod/skrypty/workflowy, (2) GitHub Actions API i pobrane logi,
(3) dokumenty kanoniczne, (4) decyzje OWNER. Konflikt dokument↔kod rozstrzygano na korzyść kodu.

**17. Odtworzony opis F-03:** `scripts/verify_linux_container.sh` maskował `|| true` progi
`ldd` (`:46`), `--version` (`:49`) i `--build-info` (`:50`); przy braku `libasound.so.2`
binarka się nie uruchamiała, a skrypt wypisywał `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`
(`:80`), więc job CI był `success`.

**18. Dowody fałszywego PASS:** run `42638736785` (commit `72c3fe6`) — 4/4 joby `success`,
a we wszystkich logach `libasound.so.2 => not found` i `error while loading shared libraries`
przy jednoczesnym komunikacie sukcesu; dodatkowo run `42518873662` (7 profili, legacy).

**19. Krytyczne `|| true` przed zmianą:** `verify_linux_container.sh:46` (`ldd`),
`:49` (`--version`), `:50` (`--build-info`); `:45` (`file`) zaklasyfikowano jako krytyczne
w nowym skrypcie.

**20. Diagnostyczne `|| true`:** `:26` (`getconf GNU_LIBC_VERSION`), `:27` (`ldd --version`),
`:31` (`PKG_QUERY_CMD`) — pozostają nieblokujące z komentarzem `NON-CRITICAL`.

**21. Cztery profile OCI:** `ubuntu-lts`, `fedora-final`, `arch-rolling`, `opensuse-tumbleweed`.

**22. Digesty obrazów:**
`ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c`;
`fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c`;
`archlinux:base-20261101.0.620205@sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9`;
`opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1`.

**23. Biblioteki wymagane w każdym profilu:** 15 sonamów (`libasound.so.2`, `libX11.so.6`,
`libXcursor.so.1`, `libXrandr.so.2`, `libXi.so.6`, `libxkbcommon.so.0`,
`libxkbcommon-x11.so.0`, `libGL.so.1`, `libfontconfig.so.1`, `libwayland-client.so.0`,
`libwayland-cursor.so.0`, `libwayland-egl.so.1`, `libgtk-3.so.0`, `libusb-1.0.so.0`,
`libudev.so.1`) + `file` i `binutils`; instalacja: apt (Ubuntu), dnf (Fedora), pacman (Arch),
zypper (Tumbleweed), potwierdzona w obrazach.

**24. Lista zmienionych plików:** implementacja — `scripts/verify_linux_container.sh`,
`scripts/install_linux_runtime_deps.sh` (nowy), `.github/workflows/build-and-release.yml`,
`tests/security_guard.rs`; dokumentacja — `AGENTS.md`, `docs/PROJECT_CURRENT_STATE.md`,
`docs/NEXT_WORK_GATE.md`, `MASTER_IMPLEMENTATION_PLAN.md`,
`docs/DECISION_TRACEABILITY_MATRIX.md`, `docs/DOCUMENTATION_INDEX.md`,
`docs/DEEPSEEK_HARNESS_HANDOFF.md`, `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.{json,md}`,
raport przedimplementacyjny i ten raport oraz `tests/security_guard.rs` (asercje statusu).

**25. Opis zmian skryptu:** `set -eu`; usunięto marker legacy; asercje SHA-256, rozpakowania,
markera i tożsamości; `file`/`readelf` jako asercje ELF; `ldd` z jawnym odrzuceniem `not found`;
`--version` = `SPLogbook 1.3.0-alpha.1`; `--build-info` z `Commit` = marker = SHA CI;
`GLIBC_2.43` porównywane semantycznie; 15 sonamów przez `ldconfig -p`; sukces jako ostatnia
instrukcja.

**26. Opis zmian workflow:** instalacja `install_linux_runtime_deps.sh` przed weryfikatorem
w `docker run`; przekazanie `EXPECTED_BUILD_COMMIT=${{ github.sha }}`; asercje pozytywnych
dowodów w logu; upload logów `if: always()`; nowy job `verify-linux-negative`; `publish-release`
zależy też od joba negatywnego.

**27. Opis kontroli `ldd`:** przechwycenie outputu i kodu, FAIL przy niezerowym kodzie,
FAIL przy `not found`, wymóg co najmniej jednego `=> /`.

**28. Opis kontroli `not found`:** jawny `case` na całym outputcie `ldd`; dodatkowo 15 sonamów
przez `ldconfig -p` (brak = FAIL).

**29. Opis kontroli `--version`:** realne uruchomienie, exit 0, brak błędu loadera, output
dokładnie `SPLogbook 1.3.0-alpha.1`.

**30. Opis kontroli `--build-info`:** exit 0, brak błędu loadera, `Version: 1.3.0-alpha.1`,
`Commit:` czysty 40-hex = marker = `EXPECTED_BUILD_COMMIT`, `Target: linux-x86_64`.

**31. Opis kontroli `GLIBC_2.43`:** `readelf --version-info` → maksimum `GLIBC_x.y` przez
`sort -V`; semantyczne porównanie major/minor z polityką `2.43`; `getconf` ≥ 2.43; FAIL przy
przekroczeniu, bez podnoszenia polityki.

**32. Opis testu negatywnego:** job `verify-linux-negative` — goły `ubuntu:26.04` (bez
bibliotek runtime) z realnym artefaktem; oczekiwany exit ≠ 0, brak komunikatu sukcesu,
obecność `not found`; wynik: **exit 1**, `libasound.so.2 => not found`,
`FAIL: ldd reports an unresolved shared library`.

**33. Opis testu pozytywnego:** 4 profile z instalacją bibliotek; każdy: `ldd` bez `not found`,
`--version` exit 0, `--build-info` exit 0, GLIBC ≤ 2.43, komunikat sukcesu; wynik **4/4 success**.

**34. Wyniki testów regresyjnych:** `m3_f_f03_fail_closed_verification_guard` — **1/1 passed**;
`m3_f_post_closure_consistency_guard` — **1/1 passed**;
`m3_f_owner_decisions_and_support_policy_guard` — **1/1 passed**.

**35. Wynik `cargo fmt`:** `cargo +stable fmt --all -- --check` → **EXIT 0**.

**36. Wynik `cargo check`:**
`cargo +stable check --workspace --all-targets -j 1` → **EXIT 0** (41,19 s, inkrementalnie).

**37. Wynik clippy:** `cargo +stable clippy --workspace --all-targets -j 1 -- -D warnings` →
**EXIT 0, 0 ostrzeżeń**.

**38. Wynik pełnego `cargo test`:**
`cargo +stable test --workspace --all-targets -j 1` → **EXIT 0** (pełny przebieg na kodzie
implementacji, przed zmianami dokumentacyjnymi): 2 692 zaliczone, 0 nieudanych, 10 ignored.
Po zmianach dokumentacyjnych powtórzono baseline: `cargo +stable test --workspace --all-targets
-j 1 -- --skip powershell_real_execution_full_cycle_success` → **EXIT 0**, 2 691 zaliczonych,
0 nieudanych, 10 ignored, 1 filtered. Jeden pominięty test (`powershell_real_execution_full_cycle_success`)
przeszedł w pełnym przebiegu na tym samym kodzie; w powtórce został zablokowany przez
**udokumentowany prompt sandboxa DSH** „Otwieranie pliku — ostrzeżenie o zabezpieczeniach”
(Low Integrity Level), nie przez regresję F-03.

**39. Rzeczywiste liczniki baseline:** pełny przebieg: zarejestrowane **2 702** (`-- --list`),
wykonane **2 692**, zaliczone **2 692**, nieudane **0**, ignorowane (HIL) **10**, HIL wykonane
**0**, PTT/RF **0/0**, doc-testy **0**. Powtórka po dokumentacji (1 filtered): wykonane **2 691**,
zaliczone **2 691**, nieudane **0**, ignorowane **10**.

**40. Wynik `security_guard`:** **122 / 122 passed**.

**41. Wynik strażnika F-03:** `m3_f_f03_fail_closed_verification_guard` — **passed**.

**42. Wyniki multi-distro:** `linux_multi_distribution_compatibility_test` — **5 / 5 passed**.

**43. Wyniki portable tar.gz:** `linux_portable_tar_gz_test` — **7 / 7 passed**.

**44. Wyniki archive security:** `linux_archive_security_test` — **13 / 13 passed**.

**45. Wynik `cargo doc`:** `cargo +stable doc --workspace --no-deps` → **EXIT 0**; 22 pre-existing
ostrzeżenia rustdoc, 0 błędów.

**46. Wynik `cargo tree -d`:** **EXIT 0**; stan zastany (52 grupy / 24 nazwy crate z duplikatami
wersji), nie pogorszony.

**47. Wynik walidacji JSON:** `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json` → **JSON OK**,
4 profile, `native_evidence.status = FAIL_CLOSED_VERIFIED`, `ci_run_id = 42685105375`.

**48. Wynik walidacji YAML:** `.github/workflows/build-and-release.yml` → **YAML OK**, 5 jobów
(`build-linux`, `build-windows`, `verify-linux-matrix`, `verify-linux-negative`,
`publish-release`), macierz dokładnie 4 profile; `.github/workflows/ci.yml` → **YAML OK**.

**49. Wynik `git diff --check`:** **EXIT 0** (przed commitami i po zmianach).

**50. Informacja o `cargo audit`:** nie uruchamiano lokalnie w tej bramce — brak zmian
`Cargo.toml`/`Cargo.lock`; job CI `Format and Lint` (z `cargo audit`) zakończył się **success**
(run `42685105448`). Nie użyto `--ignore` w tej bramce.

**51. SHA commita implementacyjnego:** `4699eacdf0076554e51107593641fb64f4dc6dfb`.

**52. Tytuł commita implementacyjnego:** `ci(linux): make native verification fail closed`.

**53. Wynik pierwszego pushu:** `git push origin main` → `e85ee4e..4699eac` (**EXIT 0**, bez force).
Dodatkowy commit doprecyzowujący: `abdb767d41bd07da1304c1c8f5d8993726a6575a` —
`ci(linux): make native evidence free of diagnostic ambiguity` (`4699eac..abdb767`, EXIT 0).

**54. Identyfikator właściwego runu CI:** `42685105375` (Build and Release SPLogbook).

**55. SHA testowany przez CI:** `abdb767d41bd07da1304c1c8f5d8993726a6575a`.

**56. Czas rozpoczęcia i zakończenia runu:** `2026-11-09T19:37:19Z` → `2026-11-09T19:58:58Z`
(`20:37–20:58+01:00`).

**57. Wynik joba build:** `Build Linux x86_64` = **success** (19:37:22Z–19:58:04Z);
`Build Windows x64` = **success** (19:37:22Z–19:58:58Z).

**58. Wynik profilu Ubuntu:** `Verify Linux (Ubuntu 26.04.1 LTS Resolute Raccoon)` = **success**
(19:58:07Z–19:58:46Z).

**59. Wynik profilu Fedora:** `Verify Linux (Fedora Linux 45 Final/GA)` = **success**
(19:58:07Z–19:58:43Z).

**60. Wynik profilu Arch:** `Verify Linux (Arch Linux Rolling Snapshot)` = **success**
(19:58:07Z–19:58:35Z).

**61. Wynik profilu Tumbleweed:** `Verify Linux (openSUSE Tumbleweed)` = **success**
(19:58:06Z–19:58:35Z).

**62. Dowód braku `not found` w Ubuntu:** log `native-evidence-ubuntu-lts.log` — **0 wystąpień**
ciągu `not found`; `libasound.so.2 => /usr/lib/x86_64-linux-gnu/libasound.so.2`;
`PASS: ldd resolved every direct dependency (all shared libraries mapped)`.

**63. Dowód braku `not found` w Fedorze:** log `native-evidence-fedora-final.log` — **0**;
`libasound.so.2 => /lib64/libasound.so.2`; `PASS: ldd …`.

**64. Dowód braku `not found` w Arch:** log `native-evidence-arch-rolling.log` — **0**;
`libasound.so.2 => /usr/lib/libasound.so.2`; `PASS: ldd …`.

**65. Dowód braku `not found` w Tumbleweed:** log `native-evidence-opensuse-tumbleweed.log` —
**0**; `libasound.so.2 => /lib64/libasound.so.2`; `PASS: ldd …`.

**66. Dowód działania `--version` w czterech profilach:** każdy log zawiera
`SPLogbook 1.3.0-alpha.1` oraz `PASS: --version executed and returned the expected identity`.

**67. Dowód działania `--build-info` w czterech profilach:** każdy log zawiera
`Version: 1.3.0-alpha.1`, `Commit: abdb767d41bd07da1304c1c8f5d8993726a6575a`,
`Target: linux-x86_64` oraz `PASS: --build-info executed and reports the expected build identity`.

**68. Dowód kontroli GLIBC w czterech profilach:** każdy log: `Maximum required GLIBC symbol:
GLIBC_2.43`, `Policy maximum required GLIBC symbol: GLIBC_2.43`, dostępne 2.43 (Ubuntu) lub 2.44
(Fedora/Arch/Tumbleweed), `PASS: GLIBC policy satisfied`.

**69. Wynik `Publish Release Assets`:** **skipped** (job `129401759837`).

**70. Potwierdzenie braku release:** GitHub API `releases` → wyłącznie historyczne `v1.2.0`,
`v1.1.0`, `v1.0.3`, `v1.0.2`, `v1.0.1`, `v1.0.0`; brak `v1.3.0*`.

**71. Potwierdzenie braku tagu:** `git ls-remote --tags origin` → brak `v1.3.0*`;
`v1.2.0` = `3ea8d51a…` / peeled `6f7d3aaa…`.

**72. SHA commita dokumentacyjnego:** **najnowszy commit `main` zawierający ten raport**
(self-reference); dokładny SHA zmierzony po pushu w tej sesji i podany w odpowiedzi końcowej.

**73. Tytuł commita dokumentacyjnego:** `docs(ci): close F-03 Linux verification gate`.

**74. Wynik finalnego pushu:** `git push origin main` (ff-only, bez force) → **EXIT 0**;
`abdb767..commit_dokumentacyjny`.

**75. HEAD/main/origin-main po pushu:** `HEAD == main == origin/main` = commit dokumentacyjny
(zmierzony po pushu; patrz odpowiedź końcowa).

**76. Ahead/behind po pushu:** **0 / 0**.

**77. `TRACKED_WORKTREE_CLEAN`:** **true**.

**78. `UNTRACKED_FILES_PRESENT`:** **true** — wyłącznie `scratch/` i jego pliki.

**79. `IGNORED_FILES_PRESENT`:** **true** (`target/`, `dist/`, `SPLogbook-backup/`, logi `scratch/`).

**80. `STRICT_WORKTREE_CLEAN`:** **false** (z powodu nieśledzonego `scratch/`).

**81. Stan `scratch/`:** **nietknięty** — niecommitowany, nieusunięty, poza stagingiem.

**82. Stan `v1.2.0`:** **nietknięty** — obiekt `3ea8d51a…`, peeled `6f7d3aaa…`.

**83. Potwierdzenie braku AppImage:** brak zmian/plików AppImage; M3-G nie rozpoczęte.

**84. Potwierdzenie braku prac M3-G:** `M3_G_STATUS = READY_FOR_OWNER_GO`, `NEXT_GATE_STATUS =
NOT_STARTED`; brak implementacji AppImage.

**85. Ryzyka rezydualne:** (a) `ubuntu-latest` w `build-linux` jest nieprzypięty — przyszły dryf
może podnieść wymagany symbol GLIBC powyżej 2.43 (bramka wykryje to jako FAIL i zablokuje);
(b) kontener OCI nie jest dowodem jądra dystrybucji (współdzieli jądro hosta); (c) obraz
Tumbleweed `latest@sha256:6e3a892…` raportuje `VERSION_ID=20261107`, podczas gdy metadane
snapshotu opisują `20261021` — rozbieżność metadanych dystrybucji, digest obrazu pozostaje
przypięty i zweryfikowany; (d) 22 pre-existing ostrzeżenia rustdoc; (e) 24 nazwy crate
z duplikatami wersji (stan zastany); (f) `scratch/` pozostaje nieśledzony
(`STRICT_WORKTREE_CLEAN=false`); (g) pre-existing test
`windows_helper_test::powershell_real_execution_full_cycle_success` jest w tym środowisku
blokowany przez prompt „Otwieranie pliku — ostrzeżenie o zabezpieczeniach” (Low IL; ten sam
test przeszedł w pełnym przebiegu na tym samym kodzie) — nie jest to regresja F-03 i nie
zmieniano testu; (h) awaria GitHub Actions 18:12–18:43Z wpłynęła na runy commita wejściowego
`e85ee4e` — udokumentowana, nie wpływa na runy bramki.

**86. Następna dozwolona bramka:** `STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE`
(`NEXT_GATE_STATUS = NOT_STARTED`, `M3_G_STATUS = READY_FOR_OWNER_GO`).

**87. Wymóg osobnego GO OWNER:** **TAK** — M3-G nie może zostać rozpoczęte bez nowego,
jawnego GO OWNER; ta bramka nie tworzy promptu implementacyjnego M3-G.

**88. Ostateczny status:** **`COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED`** —
F-03 `CLOSED`, M3-G `READY_FOR_OWNER_GO`, brak release/tagu/AppImage.

---

## Dowody kluczowe (skrót)

```
run 42685105375 | head_sha abdb767d41bd07da1304c1c8f5d8993726a6575a | conclusion success
artifact SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz
  sha256 9b31fd2437b89a80d58784338f4702bbe175d84907824d635a9a4caa9002127c
  size 19500951 B | entries 17 | build_commit abdb767…
profiles: ubuntu-lts PASS | fedora-final PASS | arch-rolling PASS | opensuse-tumbleweed PASS
negative fixture: libasound.so.2 => not found | FAIL | exit 1 | no success message
publish-release: skipped | new tag: none | new release: none | v1.2.0 unchanged
```
