# STAGE-M3-I — RAPORT KOŃCOWY (Linux RPM, x86_64)

**Dokument ID:** DSH-SPLOGBOOK-M3I-FINAL-001
**Bramka:** `STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE`
**Data raportu:** **2026-11-13** (`Europe/Warsaw`)
**Status bramki:** **`BLOCKED`**
**Bloker:** **`RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`, `RUSTSEC-2026-0538`**
**Nie użyto:** `COMPLETED_RPM_INTEGRATION_VERIFIED`

> Implementacja RPM, build, determinizm, walidator, testy negatywne i pełny
> cykl życia są zielone. Bramka **nie może** zostać zamknięta, ponieważ
> finalny job `CI` (`Format and Lint`) zakończył się `failure` na trzech nowych
> podatnościach RustSec w lockfile. Zgodnie z `AGENTS.md` §7 nowa podatność
> wymaga STOP i osobnej bramki bezpieczeństwa; nie wolno jej wyciszyć ani
> naprawiać „przy okazji” w M3-I.

---

## 1. Jednoznaczny status

| Pole | Wartość |
|---|---|
| Status bramki | **`BLOCKED`** |
| Kod statusu | `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES` |
| Implementacja RPM | **kompletna** |
| Build i weryfikacja RPM | **PASS** |
| Testy negatywne RPM | **34/34 PASS** |
| Cykl życia RPM | **PASS** (Fedora 45 i openSUSE Tumbleweed) |
| Determinizm RPM | **PASS** (dwie budowy, identyczny SHA-256) |
| Regresja pozostałych formatów | **PASS** w `Build and Release` |
| Finalny `CI` | **FAIL** — `cargo audit` |
| Tag / release / publikacja | **BRAK** |
| M3-J | **`NOT_STARTED`** |

**Bramka nie jest zamknięta.** Nie użyto statusu
`COMPLETED_RPM_INTEGRATION_VERIFIED`, ponieważ kryterium „finalne CI na
finalnym HEAD jest zielone” nie jest spełnione.

---

## 2. Chronologia (rzeczywiste daty)

| Czas (2026-11-13) | Zdarzenie |
|---|---|
| 07:54–08:00 | fetch, baseline, analiza przedimplementacyjna |
| 08:00 | commit raportu przedimplementacyjnego `956ebe0` |
| 08:0x–15:00 | implementacja RPM i iteracje CI (szczegóły commitów §4) |
| 15:30:42Z | start finalnego `Build and Release` na `3afd3ee` |
| 16:04:56Z | koniec finalnego `Build and Release` = `success`; `Publish Release Assets` = `skipped` |
| 15:30:42Z | start finalnego `CI` na `3afd3ee` |
| 15:42:32Z | `cargo audit` wykrywa 3 nowe podatności; `CI` = `failure` |
| 17:0x | raport końcowy i dokumentacja |

Nie zmieniano zegara hosta ani dat commitów.

---

## 3. Stan repozytorium na wejściu i wyjściu

### 3.1. Wejście

| Pole | Wartość |
|---|---|
| HEAD / main / origin/main | `9427e5767f101d42347a5a004d7c309e1ec29783` |
| ahead / behind | `0 / 0` |
| tracked worktree | clean |
| `scratch/` | nieśledzony, nietknięty |
| M3-H | `COMPLETED_DEB_INTEGRATION_VERIFIED` |
| M3-J | `NOT_STARTED` |

### 3.2. Wyjście (przed commitami dokumentacyjnymi)

| Pole | Wartość |
|---|---|
| Implementacyjny HEAD | `3afd3ee0d097281e4cb14e6bcb2ec901ce9d702f` |
| main / origin/main | `3afd3ee0d097281e4cb14e6bcb2ec901ce9d702f` |
| ahead / behind | `0 / 0` |
| tracked worktree | clean |
| `scratch/` | nieśledzony, niecommitowany |
| Tagi | bez nowego tagu |
| Release | bez nowego GitHub Release |

Po commitach dokumentacyjnych finalny HEAD zmienia się; raport opisuje
implementacyjny HEAD, na którym wykonano pełny `Build and Release` oraz
`CI`. Commit dokumentacyjny jest kontynuacją tej samej historii (ff-only).

---

## 4. Commity M3-I

| Commit | Rola |
|---|---|
| `956ebe0` | raport przedimplementacyjny |
| `b016977` | implementacja pipeline RPM (spec, build, verify, manifest, fixture, testy, workflow) |
| `91cfbc9` | manifest z rzeczywistej listy plików RPM |
| `a8f5e0f` | normalizacja RUNPATH i twardsze kontrole metadanych |
| `486e380` | poprawne indeksy pól manifestu |
| `b9d0715` | poprawne kody `rpmdev-vercmp` |
| `5f18f52` | guardy workflow uwzględniają joby RPM |
| `4b5f2ff` | szablon README odporny na case-insensitive FS; guard Windows |
| `17deb1d` | poprawne sekcje `%prep`/`%build` |
| `f179e96` | lista plików zgodna z rzeczywistym payloadem |
| `4ed31f2` | weryfikacja unsigned RPM przez sidecar/manifest |
| `b247e09` | `/bin/sh` w kontrakcie zależności |
| `c1d26ac` | rpmlint: `addFilter` zamiast nieobsługiwanych flag |
| `4ff50bf` | `glibc >= 2.43` zamiast symbolu |
| `2c5068d` | indeksy ścieżek manifestu |
| `ca196db` | numeryczne UID/GID |
| `b0fa3be` | rpmlintrc 2.9 |
| `3179529` | poprawny fixture wrong-architecture |
| `f015d63` | `libhamlib.so.4` 0755 zgodnie z RPM |
| `3afd3ee` | jawny `--nosignature` dla unsigned RPM |
| `docs(linux): close/block M3-I` | raport końcowy i dokumentacja (ten commit) |

Nie squashowano etapów diagnostycznych.

---

## 5. Finalny `Build and Release`

**Run:** `43453322341`
**Workflow:** `Build and Release SPLogbook`
**head_sha:** `3afd3ee0d097281e4cb14e6bcb2ec901ce9d702f`
**conclusion:** **`success`**
**URL:** https://github.com/sp6ina/SPLogbook/actions/runs/43453322341

| Job | Wynik | Job ID |
|---|---|---|
| Build Windows x64 | `success` | `131980910412` |
| Build Linux x86_64 | `success` | `131980910682` |
| Build DEB (Ubuntu 26.04 amd64) | `success` | `131993722724` |
| Verify DEB (Ubuntu 26.04 LTS native) | `success` | `131994399381` |
| Verify DEB (negative fixtures) | `success` | `131994399543` |
| Build RPM (Fedora 45 x86_64) | **`success`** | `131993722515` |
| Verify RPM (Fedora Linux 45 native) | **`success`** | `131994677500` |
| Verify RPM (openSUSE Tumbleweed compatibility) | **`success`** | `131994677240` |
| Verify RPM (negative fixtures) | **`success`** (34/34) | `131994677110` |
| Verify Linux (Ubuntu 26.04.1 LTS) | `success` | `131993722613` |
| Verify Linux (Fedora 45) | `success` | `131993722668` |
| Verify Linux (Arch Linux) | `success` | `131993722574` |
| Verify Linux (openSUSE Tumbleweed) | `success` | `131993722932` |
| Verify Linux (negative fixture) | `success` | `131993722693` |
| Verify AppImage (FUSE) | `success` | `131993722532` |
| Verify AppImage (negative fixtures) | `success` | `131993722683` |
| Publish Release Assets | **`skipped`** | `132000816363` |

Wszystkie joby istniejące przed M3-I pozostały `success`.

---

## 6. Artefakt RPM

| Pole | Wartość |
|---|---|
| Nazwa wersjonowana | `SPLogbook-Linux-x86_64-1.3.0-alpha.1.rpm` |
| Alias | `SPLogbook-Linux-x86_64.rpm` |
| Wewnętrzna nazwa RPM | `splogbook-1.3.0-0.1.alpha.1.x86_64.rpm` |
| Rozmiar | **11 879 109 B** |
| SHA-256 | **`8915f7bfc0a829d5e3a10d5c961dc68f285b0ade901cb8475ab13705c6c4a57a`** |
| `Package` | `splogbook` |
| `Version` | `1.3.0` |
| `Release` | `0.1.alpha.1` |
| `Architecture` | `x86_64` |
| `Vendor` | `SPLogbook` |
| `License` | `GPL-3.0-or-later AND LGPL-2.1-or-later` |
| `URL` | `https://github.com/sp6ina/SPLogbook` |
| `Group` | `Applications/Communications` |
| `Installed Size` | **47 054 820 B** |
| `BuildHost` | `build.splogbook.invalid` |
| `BuildTime` | `1794583835` (`2026-11-13`) |
| `Payload` | `cpio` / `xz` |
| Pliki | **12 regularnych** |
| Wpisy | **18** (12 plików + 6 katalogów) |
| `build_commit` | `3afd3ee0d097281e4cb14e6bcb2ec901ce9d702f` |
| Sygnatura | brak (jawnie unsigned; integralność sidecar + manifest) |

Alias jest bajtowo identyczny; oba sidecary SHA-256 wskazują ten sam digest.

---

## 7. Zawartość RPM

```text
/opt/SPLogbook
/opt/SPLogbook/.splogbook-rpm-marker
/opt/SPLogbook/LICENSE
/opt/SPLogbook/SPLogbook
/opt/SPLogbook/THIRD_PARTY_LICENSES
/opt/SPLogbook/THIRD_PARTY_LICENSES/HAMLIB_LGPL-2.1.txt
/opt/SPLogbook/lib
/opt/SPLogbook/lib/libhamlib.so.4
/opt/SPLogbook/locales
/opt/SPLogbook/splogbook-updater
/usr/bin/splogbook
/usr/share/applications/splogbook.desktop
/usr/share/doc/splogbook
/usr/share/doc/splogbook/README.RPM
/usr/share/icons/hicolor/256x256/apps/splogbook.png
/usr/share/licenses/splogbook
/usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt
/usr/share/licenses/splogbook/LICENSE
```

- `SPLogbook`, `splogbook-updater`, `/usr/bin/splogbook`, `libhamlib.so.4`:
  `0755`.
- `LICENSE`, licencje, marker, desktop, ikona, dokumentacja: `0644`.
- Wszystkie wpisy: `root:root` (0:0).
- `locales/` jest puste w zweryfikowanym payloadzie portable; pakiet
  zachowuje katalog, nie wymyśla plików.
- Brak `assets/`, `README.txt`, portable markera, `rigctld`, `rotctld`,
  baz, konfiguracji, eksportów, backupów i sekretów.

---

## 8. Zależności

`AutoReqProv: no`; jawne capability soname + floor pakietu glibc:

```text
/bin/sh                (×2, dla %post i %postun)
glibc >= 2.43
libasound.so.2()(64bit)
libX11.so.6()(64bit)
libXcursor.so.1()(64bit)
libXrandr.so.2()(64bit)
libXi.so.6()(64bit)
libxkbcommon.so.0()(64bit)
libxkbcommon-x11.so.0()(64bit)
libGL.so.1()(64bit)
libfontconfig.so.1()(64bit)
libwayland-client.so.0()(64bit)
libwayland-cursor.so.0()(64bit)
libwayland-egl.so.1()(64bit)
libgtk-3.so.0()(64bit)
libusb-1.0.so.0()(64bit)
libudev.so.1()(64bit)
libgcc_s.so.1()(64bit)
libm.so.6()(64bit)
libc.so.6()(64bit)
```

`rpmlib(...)` są techniczne. Brak `hamlib`, `Recommends`, `Suggests`,
`Supplements`, `Enhances`, `Conflicts`, `Obsoletes`.

---

## 9. RUNPATH, ELF, GLIBC

- `SPLogbook` i `splogbook-updater`: dokładnie `RUNPATH=$ORIGIN/lib`.
- `libhamlib.so.4`: dokładnie `RUNPATH=$ORIGIN`.
- Brak `RPATH`.
- Payload portable miał szerszy RUNPATH `$ORIGIN/lib:$ORIGIN/../lib`;
  builder RPM normalizował wyłącznie RUNPATH przez `patchelf`, bez zmiany
  kodu programu i tożsamości build.
- `ldd` na trzech ELF: exit 0, zero `not found`.
- Maksymalny wymagany symbol: `GLIBC_2.43` (`SPLogbook`, Hamlib);
  `GLIBC_2.39` (updater). Polityka ≤ 2.43 spełniona.

---

## 10. Determinizm

Dwie niezależne budowy `rpmbuild -bb` z tego samego package-root i tego
samego `SOURCE_DATE_EPOCH` dały identyczny SHA-256:

```text
8915f7bfc0a829d5e3a10d5c961dc68f285b0ade901cb8475ab13705c6c4a57a
```

Porównano: SHA-256, rozmiar, listę plików, metadane RPM, tryby, właścicieli,
scriptlety, payload, zależności i `rpm -qp --dump`. Brak różnic.
`BuildHost` = `build.splogbook.invalid`, `BuildTime` = `SOURCE_DATE_EPOCH`,
payload `xz`, mtime clamped.

---

## 11. rpmlint

- Wersja: **`rpmlint 2.9.0`** (Fedora 45).
- Wynik: **0 errors, 3 warnings, 32 filtered, 0 badness**.
- Wyjątki (tylko dwa, dziedziczone z `OWNER-M3-H-03`):
  `dir-or-file-in-opt`, `binary-or-shlib-defines-rpath`.
- Ostrzeżenia:
  1. `W: non-coherent-filename` — artefakt ma zatwierdzoną przyjazną nazwę
     `SPLogbook-Linux-x86_64-...rpm`, a RPM oczekuje nazwy wewnętrznej.
  2. `W: no-manual-page-for-binary splogbook` — brak man page w M3-I.
  3. `W: hidden-file-or-dir /opt/SPLogbook/.splogbook-rpm-marker` — marker
     musi być ukryty, bo taki plik wykrywa updater.
- Żadne z tych ostrzeżeń nie jest ukryte; wszystkie są raportowane.

---

## 12. Testy negatywne

**34/34 odrzuconych**, każdy exit ≠ 0, bez komunikatu sukcesu.

Zakres: brak/zły sidecar, uszkodzony RPM, brak/zły manifest, zła nazwa,
wersja, release, architektura, `build_commit`, `target`, brak binarki,
updatera, launchera, ikony, markera, Hamlib, zły SONAME, Hamlib nie-ELF,
dodatkowy plik, dodatkowa biblioteka, złe prawa, zły właściciel, symlink,
RUNPATH, RPATH, nierozwiązana biblioteka, launcher `sudo`/`LD_LIBRARY_PATH`,
zły desktop `Exec`, glibc > 2.43, `%pre`, scriptlet sieciowy, scriptlet
usuwający dane, niezgodność manifestu i sum kontrolnych.

Log: `rpm-negative-fixtures.log`, job `131994677110`.

---

## 13. Cykl życia

Wykonano na dwóch natywnych userspace’ach:

- **Fedora Linux 45** — referencyjny; `RPM VERIFICATION COMPLETED SUCCESSFULLY`.
- **openSUSE Tumbleweed** — kompatybilność; ten sam artefakt;
  `RPM VERIFICATION COMPLETED SUCCESSFULLY`.

Sekwencja (obie dystrybucje):

```text
fresh install
→ remove + full erase
→ previous fixture 1.3.0-0.1.alpha.0
→ upgrade 1.3.0-0.1.alpha.1
→ downgrade 1.3.0-0.1.alpha.0
→ re-upgrade 1.3.0-0.1.alpha.1
→ reinstall (--replacepkgs)
→ remove + full erase
→ fresh install
→ remove + full erase
```

Dane użytkownika (XDG config/data/state/cache, QSO DB, WAL, SHM,
`station_config.json`, ADIF, Cabrillo, backupy, token) były porównywane
SHA-256 przed/po każdej operacji; **wszystkie PASS**.

`rpm -e` usuwa wyłącznie pliki pakietu; nie ma odpowiednika `purge`.

---

## 14. Testy funkcjonalne

- `--version` = `SPLogbook 1.3.0-alpha.1`.
- `--build-info`: `Version: 1.3.0-alpha.1`, `Commit: 3afd3ee…`,
  `Target: linux-x86_64`.
- `--check-health-startup --health-proof-path`: proof `OK`.
- Launcher z `/`, `/tmp` i innego CWD: PASS.
- Uruchomienie nie zmieniło `/opt/SPLogbook`.
- `desktop-file-validate`: PASS (hint kategorii, nie błąd).
- Ikona 256×256 PNG bajtowo zgodna.
- `ldd`: zero `not found`.

---

## 15. Bezpieczeństwo i dane

- Tylko `%post` i `%postun`; brak `%pre`, `%preun`, `%pretrans`,
  `%posttrans`, triggerów, `%config`, `%ghost`.
- Brak sieci, `sudo`, `pkexec`, `doas`, `systemctl`, `crontab`, `HOME`,
  XDG w scriptletach i launcherze.
- Brak danych użytkownika i sekretów w pakiecie.
- Brak setuid/setgid/world-writable/symlinków.
- Pakiet jest **unsigned**; integralność zapewnia SHA-256 sidecar i manifest,
  a instalacja używa jawnego `rpm --nosignature`.

---

## 16. Brak regresji

W finalnym `Build and Release` (`43453322341`) wszystkie istniejące joby
były `success`:

- Build Windows x64,
- Build Linux x86_64 (portable tar.gz),
- 4× Verify Linux (Ubuntu 26.04, Fedora 45, Arch, openSUSE Tumbleweed),
- Verify Linux negative,
- 2× Verify AppImage (FUSE + negative),
- Build DEB, Verify DEB, Verify DEB negative,
- Build RPM, Verify RPM (Fedora + openSUSE), Verify RPM negative.

F-03 i `security_guard` pozostały nietknięte i zielone. `Publish Release
Assets` = `skipped`.

---

## 17. Baseline i testy kontraktowe

| Miara | Wynik |
|---|---|
| Lokalny pełny baseline (`cargo test --workspace --all-targets -j 1`, skip znanego testu Windows) | **2742 passed, 0 failed, 10 ignored, 1 filtered** |
| `security_guard` | **125/125** (w tym `m3_i_rpm_integration_guard`) |
| `linux_rpm_packaging_test` | **17/17** |
| `linux_deb_packaging_test` | **17/17** |
| `linux_appimage_packaging_test` | **14/14** |
| HIL wykonane | **0** |
| PTT / RF | **0 / 0** |

Lokalny baseline wykonano na Windows; natywne RPM jest dowodem CI.

---

## 18. `cargo audit` — BLOKER

Finalny `CI` (`43453322339`, `Format and Lint`, job `131980910338`) zakończył
się `failure`:

```text
Crate:    quinn-proto
Version:  0.11.18
ID:       RUSTSEC-2026-0524
ID:       RUSTSEC-2026-0523
Solution: Upgrade to >=0.11.19

Crate:    quinn-udp
Version:  0.5.15
ID:       RUSTSEC-2026-0538
Solution: Upgrade to >=0.5.16, <0.6.0 OR >=0.6.3
```

- 3 podatności, 1509 advisories, 674 zależności.
- Brak `--ignore` dla tych ID; nie zmieniano `Cargo.toml`/`Cargo.lock`.
- Zgodnie z `AGENTS.md` §7: **STOP**, osobna bramka bezpieczeństwa.
- To nie jest regresja wprowadzona przez M3-I; to nowa aktualizacja bazy
  RustSec wykryta w finalnym CI.

**Skutek:** bramka M3-I = `BLOCKED`; nie `COMPLETED_RPM_INTEGRATION_VERIFIED`.

---

## 19. `cargo doc` i `cargo tree -d`

- `cargo doc --workspace --no-deps`: **EXIT 0**; 21 pre-existing warningów
  rustdoc (broken intra-doc links, HTML tags, bare URLs). Nie są nowe i nie
  są wyciszane.
- `cargo tree -d`: duplikaty pre-existing (`base64`, `bitflags`, `cpal`,
  `digest`, `getrandom`, `log`, `memchr`, `miniz_oxide`, `once_cell`, `rand`,
  `rand_core`, `serde`, `serde_core`, `serde_json`, `syn`, `windows-sys`).
  Brak nowych duplikatów z M3-I.

---

## 20. Tag, release i publikacja

- Nowy tag: **BRAK**.
- Nowy GitHub Release: **BRAK**.
- Publiczny RPM: **BRAK**.
- `Publish Release Assets` = **`skipped`**.
- `publish-release` ma trwałą asercję, że `.rpm` nie trafia do
  `release-staging`.
- `v1.2.0`: obiekt `3ea8d51aab436805107a1cf422e0b35cc301458f`,
  peeled `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — **VERIFIED**,
  nietknięty.

---

## 21. Obrazy OCI

| Profil | Obraz / digest | Rola |
|---|---|---|
| Fedora 45 | `fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c` | build + verify referencyjny |
| openSUSE Tumbleweed | `opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1` | verify kompatybilnościowy |
| Ubuntu 26.04 | `ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` | DEB / F-03 |

Kontenery współdzielą jądro hosta; dowód dotyczy userspace, nie jądra
dystrybucji.

---

## 22. Ryzyka rezydualne

1. **Nowe RustSec** — bloker; wymaga osobnej bramki bezpieczeństwa i
   aktualizacji `quinn-proto`/`quinn-udp` przez `cargo`.
2. Pakiet RPM jest unsigned; integralność opiera się na sidecarze i
   manifeście, a instalacja używa `--nosignature`.
3. `rpmlint` zgłasza 3 zatwierdzone ostrzeżenia (nazwa artefaktu, brak man
   page, ukryty marker); nie są ukrywane.
4. `locales/` jest puste w payloadzie portable; RPM zachowuje katalog.
5. Brak testu na jądrze dystrybucji — OCI dzieli jądro hosta.
6. HIL/PTT/RF nie wykonano.
7. `cargo doc` ma 21 pre-existing warningów; nie naprawiano ich w M3-I.
8. Test Windows `test_xdg_relative_paths_are_rejected_and_fallback_to_home`
   jest wrażliwy na równoległe zmiany zmiennych środowiskowych; w finalnym
   runie `Build and Release` przeszedł, ale w jednym z wcześniejszych runów
   wystąpił flake. Nie zmieniano testu.

---

## 23. M3-J

**`M3-J` (Flatpak) = `NOT_STARTED`.**
Nie utworzono katalogu `installer/linux/flatpak`, nie dodano manifestu
Flatpak, nie uruchomiono żadnego joba M3-J.

---

## 24. Decyzja końcowa

```text
STAGE-M3-I — BLOCKED
BLOCKED_BY_NEW_RUSTSEC_ADVISORIES
RUSTSEC-2026-0523
RUSTSEC-2026-0524
RUSTSEC-2026-0538
```

Implementacja RPM jest kompletna i zweryfikowana, ale bramka **nie może**
zostać zamknięta do czasu zielonego `cargo audit` w osobnej, autoryzowanej
bramce bezpieczeństwa. Nie użyto statusu
`COMPLETED_RPM_INTEGRATION_VERIFIED`.
