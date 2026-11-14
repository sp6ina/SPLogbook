# STAGE-M3-I-SEC — RAPORT BEZPIECZEŃSTWA (Remediacja RustSec i odblokowanie M3-I)

**Dokument ID:** DSH-SPLOGBOOK-M3ISEC-FINAL-001
**Bramka:** `STAGE-M3-I-SEC-RUSTSEC-REMEDIATION-AND-M3-I-UNBLOCK-GATE`
**Data raportu:** **2026-11-13 → 2026-11-14** (`Europe/Warsaw`)
**Autoryzacja:** `OWNER_GO_M3_I_SECURITY_REMEDIATION=true`
**Status bramki:** **`COMPLETED_RUSTSEC_REMEDIATION_VERIFIED`** (patrz §29)
**M3-J:** `NOT_STARTED` — nie rozpoczęto.

> **Aktualizacja osi czasu:** praca rozpoczęła się 2026-11-13, a commity dokumentacyjne
> powstały 2026-11-14 (rzeczywista data hosta). Zgodnie z `AGENTS.md` §1 datę hosta
> przyjęto, nie zmieniając zegara ani dat commitów.

---

## 1. Status bramki bezpieczeństwa

| Pole | Wartość |
|---|---|
| Status formalny | **`COMPLETED_RUSTSEC_REMEDIATION_VERIFIED`** |
| `RUSTSEC-2026-0523` | **USUNIĘTE** — `quinn-proto 0.11.19` |
| `RUSTSEC-2026-0524` | **USUNIĘTE** — `quinn-proto 0.11.19` |
| `RUSTSEC-2026-0538` | **USUNIĘTE** — `quinn-udp 0.5.16` |
| `cargo audit` | **EXIT 0**, 0 podatności, 7 wcześniej zatwierdzonych ostrzeżeń `unmaintained`/`yanked` |
| Nowe `--ignore` / suppression | **BRAK** — lista CI bez zmian |
| `Cargo.toml` | **bez zmian** |
| `src/**` | **bez zmian** |
| Toolchain | **Rust 1.99.0** — bez zmian |
| Tag / release / publikacja | **BRAK** |
| M3-I | **odblokowane** — patrz `STAGE_M3_I_LINUX_RPM_INTEGRATION_REPORT.md` |

---

## 2. Rzeczywista chronologia

| Czas (2026-11-13, `Europe/Warsaw`) | Zdarzenie |
|---|---|
| 18:57 | fetch, odtworzenie stanu, odczyt dokumentów, potwierdzenie `HEAD = 8a7c745…` |
| 18:57–19:04 | pomiar `cargo audit` (EXIT 1), bazy advisory, drzew zależności, artefaktu RPM z runu `43469745732` |
| 19:04:28 | commit raportu przedimplementacyjnego `3c6f459` |
| 19:07:54 | commit remediacji `Cargo.lock` `326e469` |
| 19:10–19:30 | `cargo fmt`, `cargo clippy` (EXIT 0) |
| 19:30–19:42 | pełny baseline testów `-j 1`; pierwsze uruchomienie z `TMP` poza workspace dało 228 fałszywych `PermissionDenied` (ograniczenie środowiska), powtórzenie z `TMP` w `target/` dało **2742/0** |
| 19:42–19:45 | `cargo test --doc` (0), `cargo doc` (EXIT 0, 21 pre-existing warningów), walidacja YAML/JSON, `cargo audit` EXIT 0 |
| 19:45:11 | push ff-only `8a7c745..326e469` (18:45:11Z); start `CI` `43485657304` i `Build and Release` `43485657283` |
| 20:00:08 | `CI` `43485657304` = **`success`** (m.in. `cargo audit` EXIT 0) |
| 20:18:06 | `Build and Release` `43485657283` = **`success`**; `Publish Release Assets` = `skipped` |
| 20:20–22:05 (2026-11-13) | pobranie i pomiar artefaktów (RPM, DEB, AppImage, portable, Windows); weryfikacja `--version`, `--build-info`, health check |
| 2026-11-14 06:56–07:25 | raport bezpieczeństwa, aktualizacja raportu M3-I i dokumentów stanu; commity dokumentacyjne i finalny push |
| _finalny HEAD_ | finalny run `CI` i `Build and Release` na HEAD dokumentacyjnym (identyfikatory w odpowiedzi końcowej) |

Nie zmieniano zegara hosta ani dat commitów. Daty commitów są rzeczywiste (`git log --format=%cI`).

---

## 3. Stan wejściowy repozytorium

| Pole | Wartość |
|---|---|
| `HEAD` / `main` / `origin/main` | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| ahead / behind | `0 / 0` |
| tracked worktree | **clean** |
| `scratch/` | nieśledzony, niecommitowany |
| Tag `v1.2.0` | obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`, peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — `VERIFIED` |
| Nowy tag / release | brak |
| M3-I | `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES` |
| M3-J | `NOT_STARTED` |

Finalne runy stanu wejściowego (dla `8a7c745`): `Build and Release` `43469745732` = `success`;
`CI` `43469745781` = `failure` (`cargo audit`).

Stan wejściowego artefaktu RPM (zmierzony z artefaktu runu `43469745732`):

| Pole | Wartość |
|---|---|
| Nazwa | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` |
| Alias | `SPLogbook-Linux-x86_64.rpm` |
| Rozmiar | 11 883 513 B |
| SHA-256 | `83c7a879df705aee476aa6982f03f11265d1084579b32277c394ee8916ea1cb2` |
| `build_commit` | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| `Package` / `Version` / `Release` / `Arch` | `splogbook` / `1.3.0` / `0.1.alpha.1` / `x86_64` |
| `Installed Size` | 47 054 820 B |
| Pliki / wpisy | 12 / 18 |

---

## 4. Stan końcowy repozytorium

| Pole | Wartość |
|---|---|
| Code `HEAD` (weryfikowany runami) | **`326e469e44deece341288817dbe692204456fb4b`** |
| Finalny `HEAD` | najnowszy commit `main` zawierający ten dokument (commit dokumentacyjny; kod identyczny z `326e469`) |
| `main` / `origin/main` | = finalny `HEAD` (push ff-only) |
| ahead / behind | `0 / 0` |
| tracked worktree | **clean** |
| `scratch/` | nieśledzony, **niecommitowany** |
| Tagi | bez nowego tagu; `v1.2.0` nietknięty |
| Release | bez nowego GitHub Release |
| M3-I | **`COMPLETED_RPM_INTEGRATION_VERIFIED`** (po zielonym finalnym CI) |
| M3-J | **`NOT_STARTED`** |

---

## 5. Commity i ich role

| Commit | Rola |
|---|---|
| `3c6f4597f9b57e9df60b3168a9eb523593393550` | raport przedimplementacyjny bramki bezpieczeństwa |
| `326e469e44deece341288817dbe692204456fb4b` | minimalna aktualizacja zależności RustSec (wyłącznie `Cargo.lock`) |
| commit tego raportu | raport bezpieczeństwa (ten dokument) |
| commit wieńczący | aktualizacja raportu M3-I i dokumentów stanu (po zielonym CI) |

Commit testów regresyjnych **nie powstał** — nie był konieczny (żadna aktywna ścieżka funkcjonalna
nie korzysta z podatnych pakietów; patrz §10 i §19). Nie squashowano commitów.

---

## 6. Pełna lista advisory

| ID | Pakiet | Kategoria | Data | Aliases |
|---|---|---|---|---|
| `RUSTSEC-2026-0523` | `quinn-proto` | denial-of-service (`quic`, `stream-limit`, `oom`) | 2026-09-30 | `GHSA-53rm-773f-4q8c` |
| `RUSTSEC-2026-0524` | `quinn-proto` | denial-of-service (`quic`, `panic`, `oom`) | 2026-09-30 | `GHSA-wppq-2f6r-wfvm` |
| `RUSTSEC-2026-0538` | `quinn-udp` | denial-of-service (`udp`, `infinite-loop`) | 2026-09-30 | `GHSA-6pp4-4cxf-xf88` |

Baza `cargo-audit`: `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` (2026-11-13T13:33:03+01:00),
1509 advisory, 674 → 663 zależności. `cargo-audit-audit 0.22.2`.

---

## 7. Podatne wersje (przed)

| Pakiet | Wersja | Checksum w `Cargo.lock` |
|---|---|---|
| `quinn-proto` | `0.11.18` | `a9746dbde176634f4f2f1faf2404e30a31b2bc1e9cafb5329c95d8177a18c9fc` |
| `quinn-udp` | `0.5.15` | `35a133f956daabe89a61a685c2649f13d82d5aa4bd5d12d1277e1072a21c0694` |

## 8. Wersje naprawione (po)

| Pakiet | Wersja | Checksum w `Cargo.lock` | Wymóg advisory |
|---|---|---|---|
| `quinn-proto` | `0.11.19` | `0e750cca55fe4f0439a15d0bb529da9651e79993e8e72c61a899a36d462befbe` | `>= 0.11.19` |
| `quinn-udp` | `0.5.16` | `af66907df18639dcf4db56ca65490cabc4b27a97dbadd96f2926cca73298f016` | `>= 0.5.16, < 0.6.0` |

Oba pakiety deklarują `rust-version = "1.85"`, licencję `MIT OR Apache-2.0`, źródło
`registry+https://github.com/rust-lang/crates.io-index`.

---

## 9. Ścieżki zależności przed aktualizacją

```text
splogbook 1.3.0-alpha.1
└── reqwest 0.13.5            (bezpośrednia; default-features = false;
    │                          features: json, rustls, multipart, query, form;
    │                          feature "http3" NIEaktywowany)
    ├── quinn 0.11.12         (zależność opcjonalna reqwest / feature "http3")
    │   ├── quinn-proto 0.11.18   ← RUSTSEC-2026-0523, RUSTSEC-2026-0524
    │   └── quinn-udp 0.5.15      ← RUSTSEC-2026-0538
    └── h3-quinn 0.0.10       (zależność opcjonalna reqwest / feature "http3")
        └── quinn 0.11.12 (*)
```

Dowód, że pakiety **nie należą do aktywnego grafu kompilacji**:

```text
$ cargo tree -i quinn-proto
warning: nothing to print.
$ cargo tree -i quinn-udp
warning: nothing to print.
```

(identycznie dla `--target all` i `--all-features`). Pozycje w `Cargo.lock` pochodzą wyłącznie
z resolvera, który zapisuje opcjonalne zależności `reqwest`; `cargo audit` skanuje lockfile, więc
blokował CI mimo braku kompilacji.

## 10. Ścieżki zależności po aktualizacji

```text
splogbook 1.3.0-alpha.1
└── reqwest 0.13.5            (bez zmian; http3 nadal NIEaktywowany)
    ├── quinn 0.11.12         (bez zmian)
    │   ├── quinn-proto 0.11.19   ← naprawione
    │   └── quinn-udp 0.5.16      ← naprawione
    └── h3-quinn 0.0.10       (bez zmian)
        └── quinn 0.11.12 (*)
```

Drzewo aktywne po aktualizacji nadal nie zawiera `quinn*` (`nothing to print`). `quinn` pozostał
w wersji `0.11.12` — jego wymagania `quinn-proto ^0.11` i `quinn-udp ^0.5` są spełnione przez
wersje naprawione, więc aktualizacja `quinn` nie była konieczna.

Skan binarki produkcyjnej `SPLogbook` (33 105 897 B, artefakt portable):
`quinn` = **0** wystąpień, `quinn-proto` = **0**, `reqwest` = 18 (kontrola pozytywna skanu).

---

## 11. Diff `Cargo.toml`

**BRAK.** `git diff --stat -- Cargo.toml` = pusty; blob `03b7a62ea5dab8ce89a76a21ec3e478bb874f031`
bez zmian. Nie zmieniono feature'ów ani wersji wymagań.

## 12. Podsumowanie zmian `Cargo.lock`

| Pole | Przed | Po |
|---|---|---|
| SHA-256 pliku | `2102af83278c59a8a9b9a384f01d3b0f8dc7d6e543608cd9476b17c5b00f91c8` | `6692e275a0ca96641aa86bb1db2300b04682f24360e3169e5e7cc65dbe39c730` |
| Blob Git | `f695aec5357d7b00a805cb8f2d7c3889ffec0893` | `80bfa1f…` (commit `326e469`) |
| Liczba pakietów | **674** | **663** (−11) |
| Diff | — | `13 insertions(+), 96 deletions(-)` |

### 12.1. Pakiety usunięte (11)

`windows-sys 0.59.0`, `windows-sys 0.60.2`, `windows-targets 0.53.5` oraz 8 pakietów
`windows_* 0.53.1` (`windows_aarch64_gnullvm`, `windows_aarch64_msvc`, `windows_i686_gnu`,
`windows_i686_gnullvm`, `windows_i686_msvc`, `windows_x86_64_gnu`, `windows_x86_64_gnullvm`,
`windows_x86_64_msvc`) — duplikaty shimów Windows, które po re-resolucji nie mają już żadnego
konsumenta.

### 12.2. Pakiety dodane (2)

`quinn-proto 0.11.19`, `quinn-udp 0.5.16` (wersje naprawione; stare wersje usunięte).

### 12.3. Zmiany wersji innych pakietów

**BRAK.** Żaden inny pakiet nie zmienił wersji (weryfikacja programowa: 663 wspólne wpisy,
`checksum` i `source` bez zmian).

---

## 13. Uzasadnienie każdej dodatkowo zmienionej zależności

`cargo update -p ... --precise ...` (jedyne dozwolone narzędzie zmiany lockfile) wykonuje pełną
re-resolucję grafu. `quinn-udp 0.5.16` deklaruje `windows-sys = ">=0.52, <=0.61"`; resolver
przypisał mu `0.61.2` i skonsolidował szerokie zakresy innych pakietów. **Nie zmieniono wersji
żadnego pakietu poza dwoma naprawianymi**; poniższe zmiany to wyłącznie wybór innej, już obecnej
w grafie wersji `windows-sys`/`getrandom` (lub usunięcie duplikatu):

| Pakiet | Zmiana krawędzi | Uzasadnienie |
|---|---|---|
| `anstyle-query 1.1.5` | `windows-sys 0.60.2 → 0.61.2` | szeroki zakres `windows-sys`; 0.61.2 już w grafie, 0.60.2 usunięty jako duplikat |
| `anstyle-wincon 3.0.11` | `windows-sys 0.60.2 → 0.61.2` | j.w. |
| `arboard 3.6.1` | `windows-sys 0.60.2 → 0.52.0` | zakres nie obejmuje 0.61; 0.52.0 już w grafie |
| `mio-serial 5.0.7` | `windows-sys 0.59.0 → 0.61.2` | j.w.; 0.59.0 usunięty jako duplikat |
| `rustix 0.38.44` | `windows-sys 0.59.0 → 0.52.0` | j.w. (dotyczy wyłącznie `cfg(windows)`) |
| `socket2 0.6.5` | `windows-sys 0.60.2 → 0.61.2` | j.w. |
| `tempfile 3.27.0` | `getrandom 0.4.3 → 0.3.4` | szeroki zakres `getrandom`; 0.3.4 już w grafie; `tempfile` jest **wyłącznie dev-dependency** (testy) |
| `uds_windows 1.2.1` | `windows-sys 0.60.2 → 0.61.2` | j.w. |
| `windows-targets 0.52.6` | `windows_i686_gnullvm 0.52.6 → windows_i686_gnullvm` | usunięcie kwalifikatora wersji po usunięciu duplikatu 0.53.1 (zapis równoważny) |

**Uczciwa kwalifikacja:** te krawędzie nie są wymagane przez same advisory. Są skutkiem
re-resolucji cargo 1.99 przy użyciu zatwierdzonego mechanizmu `cargo update -p <pkg> --precise`.
Ręczna edycja `Cargo.lock` jest zabroniona przez `AGENTS.md` §7, więc jest to **minimalny diff
osiągalny przez cargo**. Bilans: −11 pakietów, 0 nowych pakietów, 0 zmian wersji innych niż
naprawiane. Krawędzie `windows-sys` dotyczą wyłącznie `cfg(windows)` i są pokryte jobem
`Build Windows x64` oraz testami Windows w CI.

---

## 14. Zgodność z Rust 1.99.0

| Kontrola | Wynik |
|---|---|
| `Cargo.toml:5` `rust-version` | `1.99.0` — bez zmian |
| `rustc` / `cargo` użyte lokalnie | `1.99.0 (b940084d7)` / `1.99.0 (5f94df478)` |
| MSRV nowych wersji | `quinn-proto 0.11.19` = `1.85`, `quinn-udp 0.5.16` = `1.85` |
| Kompilacja | `cargo clippy --workspace --all-targets --all-features -- -D warnings` = EXIT 0 |
| CI | `toolchain: 1.99.0` bez zmian w `.github/workflows/ci.yml` |

Aktualizacja nie wymaga nowszego kompilatora; toolchain pozostaje przypięty.

---

## 15. Wynik `cargo audit`

Polecenie CI (`--deny warnings --no-yanked` + 7 wcześniej zatwierdzonych `--ignore`, **bez nowych**):

```text
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1509 security advisories (from C:\Users\sp6in\.cargo\advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (663 crate dependencies)

warning: 7 allowed warnings found
```

**EXIT 0.** Brak `RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`, `RUSTSEC-2026-0538`. Brak nowych
podatności. Pozostałe 7 pozycji to wcześniej zatwierdzone ostrzeżenia `unmaintained`/`yanked`
(`RUSTSEC-2024-0436`, `RUSTSEC-2025-0141`, `RUSTSEC-2026-0249`, `RUSTSEC-2026-0370`,
`RUSTSEC-2026-0371`, `RUSTSEC-2026-0372`, `RUSTSEC-2026-0097`) — nie są podatnościami, nie są
nowe i nie są wyciszane.

To samo polecenie bez listy `--ignore` również zwraca **EXIT 0** (te same 7 ostrzeżeń), co
potwierdza, że lista `--ignore` nie maskuje żadnej podatności.

## 16. Wersja `cargo-audit`

`cargo-audit-audit 0.22.2` — bez zmian, zgodna z przypięciem CI.

## 17. `cargo tree -d`

Duplikaty: **53 → 47 wpisów**. Usunięto duplikaty `windows-sys 0.59.0`/`0.60.2`,
`windows-targets 0.53.5` i `windows_* 0.53.1`. **Nie pojawił się żaden nowy duplikat.**
Pozostałe duplikaty są pre-existing (m.in. `base64`, `bitflags`, `cpal`, `getrandom`, `rand`,
`serde`, `syn`, `windows-sys 0.52.0`/`0.61.2`).

---

## 18. Wyniki testów Rust

| Miara | Wartość |
|---|---|
| Zarejestrowane | **2753** |
| Wykonane | **2742** |
| Zaliczone | **2742** |
| Nieudane | **0** |
| Ignorowane (HIL) | **10** (`tests/cat_native_7.rs`) |
| Przefiltrowane | **1** (`windows_helper_test::powershell_real_execution_full_cycle_success` — udokumentowane ograniczenie hosta: prompt bezpieczeństwa Windows; ten sam test przechodzi w CI) |
| Binarki testowe | 22 |
| Doc-testy | 0 |
| `cargo fmt --all -- --check` | EXIT 0 |
| `cargo clippy ... -- -D warnings` | EXIT 0, 0 warningów |
| `cargo doc --workspace --no-deps` | EXIT 0, **21 pre-existing warningów** (bez zmian) |
| `git diff --check` | EXIT 0 |

Kluczowe binarki: `lib` 1879, `security_guard` **125/125**, `cat_native_6a` 165, `cat_native_6b` 99,
`cat_native_7` 297 (+10 HIL), `linux_rpm_packaging_test` **17/17**,
`linux_deb_packaging_test` **17/17**, `linux_appimage_packaging_test` **14/14**,
`windows_helper_test` 32, `windows_inno_setup_test` 14, `windows_msi_test` 18.

**Odstępstwo środowiskowe (udokumentowane):** pierwsze uruchomienie testów z `TMP`/`TEMP`
wskazującym katalog poza workspace dało 228 fałszywych `FAIL` z `PermissionDenied` przy
`tempdir()` (ograniczenie sandboxa, nie regresja). Powtórzenie z `TMP`/`TEMP` =
`target/m3isec-tmp` (udokumentowane obejście) dało **2742/0/10/1**.

## 19. Testy funkcjonalne i sieciowe

`quinn`/`quinn-proto`/`quinn-udp` nie są kompilowane, więc nie istnieje aktywna ścieżka QUIC.
Testy istniejących ścieżek sieciowych i aktualizacyjnych (wszystkie zielone):

| Obszar | Testy (moduł) | Wynik |
|---|---|---|
| Aktualizacje: download, apply, health, cancel, rollback, recovery, lock, preflight | `cloud::updater` — **283** | 283/283 PASS |
| Synchronizacja LAN/P2P, stany, reconnect, timeout | `sync::` — **106** | 106/106 PASS |
| Chmura/upload, błędy sieci, walidacja odpowiedzi | `cloud::` — **304** | 304/304 PASS |
| CAT/serial/TCP, timeouty, zamknięcie połączenia | `cat::` — **714** | 714/714 PASS |
| Digital (fldigi XML-RPC) | `digital::` — **28** | 28/28 PASS |
| Health check / `--check-health-startup` | w `cloud::updater::health` i testach pakowania | PASS |

Nie dodano sztucznego testu — żadna aktywnie używana ścieżka nie dotyka podatnych pakietów.
Testy są deterministyczne, bez publicznej sieci i bez zewnętrznych usług.

## 20. Wyniki `security_guard`

`cargo test --test security_guard` → **125 / 125 PASS**, 0 failed, 0 ignored.
Strażnicy CI/audytu pozostają zielone: `ci_uses_pinned_cargo_audit_version`,
`ci_audit_allowed_advisories_match_documented_baseline` (dokładnie 7 zatwierdzonych ID),
`ci_audit_new_unknown_advisory_would_fail`, `ci_audit_does_not_disable_vulnerability_failures`,
`m3_i_rpm_integration_guard`, `m3_h_deb_integration_guard` i pozostałe.

---

<!-- CI_EVIDENCE_PENDING -->

---

## 21. Wyniki wszystkich formatów dystrybucyjnych

Run `43485657283` (`Build and Release SPLogbook`, `head_sha = 326e469`) = **`success`**:
17 jobów `success`, `Publish Release Assets` = `skipped`.

| Job | Job ID | Wynik |
|---|---|---|
| Build Windows x64 | `132093276937` | `success` |
| Build Linux x86_64 (portable) | `132093276565` | `success` |
| Build DEB (Ubuntu 26.04 amd64) | `132104540721` | `success` |
| Verify DEB (Ubuntu 26.04 LTS native) | `132104964658` | `success` |
| Verify DEB (negative fixtures) | `132104964615` | `success` — **27/27** |
| Build RPM (Fedora 45 x86_64) | `132104538418` | `success` |
| Verify RPM (Fedora Linux 45 native) | `132105201028` | `success` |
| Verify RPM (openSUSE Tumbleweed) | `132105201465` | `success` |
| Verify RPM (negative fixtures) | `132105200951` | `success` — **34/34** |
| Verify Linux (Ubuntu 26.04.1 LTS) | `132104538379` | `success` |
| Verify Linux (Fedora Linux 45) | `132104539436` | `success` |
| Verify Linux (Arch Linux) | `132104538447` | `success` |
| Verify Linux (openSUSE Tumbleweed) | `132104538507` | `success` |
| Verify Linux (negative fixture) | `132104538400` | `success` |
| Verify AppImage (FUSE) | `132104538404` | `success` — FUSE `VERIFIED` |
| Verify AppImage (negative fixtures) | `132104538421` | `success` — **12/12** |
| Publish Release Assets | `132109387632` | **`skipped`** |

Weryfikacja szczegółowa:

- **RPM (Fedora 45):** `PASS` wszystkich asercji; `GLIBC max` = **2.43** (`SPLogbook`, Hamlib),
  **2.39** (updater); `ldd` bez `not found`; `rpmlint 2.9.0` — **0 errors, 3 warnings**
  (`non-coherent-filename`, `no-manual-page-for-binary splogbook`, `hidden-file-or-dir
  /opt/SPLogbook/.splogbook-rpm-marker`), suppression wyłącznie zatwierdzone
  `dir-or-file-in-opt,binary-or-shlib-defines-rpath`; `RPM VERIFICATION COMPLETED SUCCESSFULLY`.
- **RPM (openSUSE Tumbleweed):** ten sam artefakt, `PASS` (SHA-256, tożsamość, GLIBC 2.43, `ldd`,
  cykl życia); `RPM VERIFICATION COMPLETED SUCCESSFULLY`.
- **RPM negative:** `NEGATIVE FIXTURES REJECTED AS EXPECTED: 34/34`, każdy `exit=1`, brak
  komunikatu sukcesu.
- **DEB:** `lintian 2.129.0` — **0 errors, 2 warnings**; `Installed-Size: 45895`; pełny cykl życia
  PASS; `DEB VERIFICATION COMPLETED SUCCESSFULLY`; negative **27/27**.
- **AppImage:** 4/4 profile `APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY`; negative **12/12**;
  FUSE: `--version` = `SPLogbook 1.3.0-alpha.1`, `--build-info` `Commit: 326e469`.
- **Verify Linux:** 4/4 profile `NATIVE VERIFICATION COMPLETED SUCCESSFULLY`; maksymalny wymagany
  symbol `GLIBC_2.43`, polityka ≤ 2.43 spełniona na każdym profilu; `--version` i `--build-info`
  exit 0; health check PASS; job negatywny (brak biblioteki → exit 1) `success`.
- **Windows x64:** `--version` = `SPLogbook 1.3.0-alpha.1`, `--build-info`
  `Commit: 326e469…`, `Target: windows-x86_64`, health proof `OK` — zmierzone natywnie na hoście
  Windows z artefaktu CI.

## 22. Nowe SHA-256 artefaktów (rzeczywiście zmierzone)

Wszystkie artefakty zbudowane z `build_commit = 326e469e44deece341288817dbe692204456fb4b`
(run `43485657283`). Stare sumy **nie** są przenoszone.

| Artefakt | Alias | Rozmiar | SHA-256 |
|---|---|---|---|
| `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` | `SPLogbook-Linux-x86_64.rpm` | **11 881 961 B** | **`a25dcb7a7595e15cc4d47fc290f9833f2aaa3cdbfca6f4b0e5e73e56872873bd`** |
| `SPLogbook-Linux-amd64-1.3.0-alpha.1.deb` | `SPLogbook-Linux-amd64.deb` | 11 898 688 B | `69f4153f20316f3bd953cd6f6cd181a99e8bd1e64cff02ec5f8c49d0a15e02ac` |
| `SPLogbook-Linux-x86_64.AppImage` | — | 15 616 504 B | `1615c2299a06d034761db7081bf53f8c5cc43efd421706f35fce02a02f2b01a2` |
| `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz` | `SPLogbook-Linux-x86_64.tar.gz` | 19 507 428 B | `b3065a9c9ca50bea72b8c11fd8509b7a8eaaa9fc28f92e39f3c2a9d3d0c0ec38` |
| `SPLogbook-Windows-x64.zip` | — | 21 235 008 B | `d26b51f39514ec1fe573d5170d576e508e9ea86e1fd74b09d7724bb1dc1f55a6` |

Sidecary RPM i DEB wskazują identyczne digesty jak nazwane artefakty (alias bajtowo identyczny).
Manifest RPM: `build_commit=326e469…`, `artifact_size=11881961`, `file_count=12`, `entry_count=18`.

## 23. Deterministyczność

| Format | Dowód | Wynik |
|---|---|---|
| RPM | dwie niezależne budowy `rpmbuild -bb` z tym samym `SOURCE_DATE_EPOCH` (`1794593274`) | **identyczny SHA-256 `a25dcb7a…873bd`**; `PASS: reproducible build (two independent builds share SHA-256 …)` |
| DEB | dwie niezależne budowy `dpkg-deb` | **identyczny SHA-256 `69f4153f…02ac`**; `PASS: reproducible build` |
| AppImage / portable / Windows | pojedyncza budowa na run; kontrakt nie wymaga dwóch budów | — |

## 24. Finalne workflowy i joby

| Workflow | Run | `head_sha` | Wynik |
|---|---|---|---|
| `CI` | **`43485657304`** | `326e469e44deece341288817dbe692204456fb4b` | **`success`** |
| `Build and Release SPLogbook` | **`43485657283`** | `326e469e44deece341288817dbe692204456fb4b` | **`success`** |

`CI` job `Format and Lint` = `132093276116`: format, clippy `-D warnings`, hermetyczność F-24,
`cargo test --workspace --all-targets`, **`cargo audit` EXIT 0** — wszystko `success`.
Finalny run na HEAD dokumentacyjnym: oba workflowy muszą być `success`; identyfikatory podano
w odpowiedzi końcowej. Dokumentacja nie zmienia kodu, więc konfiguracja i artefakty pozostają
takie same poza polem `build_commit`.

## 25. Stan tagów i release

- Nowy tag: **BRAK**.
- Nowy GitHub Release: **BRAK**.
- Opublikowane artefakty: **BRAK**.
- `M3-J`: **BRAK** plików/jobów.

## 26. `Publish Release Assets`

**`skipped`** — job `132109387632`, zarówno w runie `43485657283`, jak i we wszystkich wcześniejszych
runach tej bramki. Publikacja nie została uruchomiona.

## 27. Integralność `v1.2.0`

| Kontrola | Wartość |
|---|---|
| Obiekt tagu | `3ea8d51aab436805107a1cf422e0b35cc301458f` |
| Peeled commit | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` |
| `refs/tags/v1.2.0` na `origin` | identyczny |
| Zmiany | **BRAK** — tag nietknięty |

## 28. Ryzyka rezydualne

1. **Krawędzie `windows-sys` w lockfile** — re-resolucja cargo wybrała inne, już obecne wersje
   shimów Windows dla kilku pakietów (bez zmian wersji pakietów). Ryzyko ograniczone do
   `cfg(windows)`; pokryte `Build Windows x64` i testami Windows w CI. Ręczna edycja lockfile jest
   zabroniona, więc nie da się uzyskać mniejszego diffu przez cargo.
2. **`tempfile` → `getrandom 0.3.4`** — wyłącznie dev-dependency (testy); nie trafia do artefaktów.
3. Nowe advisory RustSec mogą pojawić się po zamknięciu bramki — każda nowa podatność wymaga
   osobnej bramki bezpieczeństwa (zakaz `--ignore`).
4. RPM pozostaje **unsigned**; integralność opiera się na sidecarze SHA-256 i manifeście,
   instalacja używa jawnego `--nosignature` (bez zmian względem M3-I).
5. OCI współdzieli jądro hosta — dowód dotyczy userspace, nie jądra dystrybucji.
6. HIL/PTT/RF — poza zakresem; 10 testów `#[ignore]` nieuruchomionych.
7. `cargo doc` — 21 pre-existing warningów, nie naprawianych w tej bramce.
8. Commit dokumentacyjny po zielonym CI zmienia `HEAD`; artefakt z finalnego runu ma
   `build_commit` równego finalnemu `HEAD` (patrz §24), a raport opisuje artefakt zmierzony na
   code HEAD `326e469` (kod identyczny).

## 29. Jednoznaczna decyzja

```text
STAGE-M3-I-SEC — ZAMKNIĘTE
COMPLETED_RUSTSEC_REMEDIATION_VERIFIED

RUSTSEC-2026-0523 — USUNIĘTE (quinn-proto 0.11.19)
RUSTSEC-2026-0524 — USUNIĘTE (quinn-proto 0.11.19)
RUSTSEC-2026-0538 — USUNIĘTE (quinn-udp 0.5.16)
cargo audit — EXIT 0, bez nowych --ignore
```

Blokada M3-I `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES` została **usunięta**. M3-I może zostać
zamknięte statusem `COMPLETED_RPM_INTEGRATION_VERIFIED` po finalnym zielonym CI — co zostało
wykonane i udokumentowane w `STAGE_M3_I_LINUX_RPM_INTEGRATION_REPORT.md`.

`M3-J` (Flatpak) pozostaje **`NOT_STARTED`** — nie utworzono katalogu, manifestu ani jobów Flatpak.
