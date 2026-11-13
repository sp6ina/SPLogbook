# STAGE-M3-I-SEC — RAPORT PRZEDIMPLEMENTACYJNY (Remediacja RustSec dla M3-I)

**Dokument ID:** DSH-SPLOGBOOK-M3ISEC-PRE-001
**Bramka:** `STAGE-M3-I-SEC-RUSTSEC-REMEDIATION-AND-M3-I-UNBLOCK-GATE`
**Data raportu:** **2026-11-13** (`Europe/Warsaw`)
**Autoryzacja właściciela:** `OWNER_GO_M3_I_SECURITY_REMEDIATION=true`
**Status bramki na wejściu:** `IN_PROGRESS`
**Zakres:** wyłącznie usunięcie `RUSTSEC-2026-0523`, `RUSTSEC-2026-0524`, `RUSTSEC-2026-0538`
i odblokowanie M3-I. **M3-J pozostaje `NOT_STARTED`.**

> Raport powstał **przed** jakąkolwiek zmianą `Cargo.toml` / `Cargo.lock`.
> Zawiera wyłącznie pomiary wykonane na `HEAD = 8a7c745d5707c5aecd118b92d398dc3dbf52e831`.
> Nie użyto `--ignore`, nie wyciszono advisory, nie zmieniono workflowów ani progów CI.

---

## 1. Dokładny stan repozytorium na wejściu

### 1.1. Git — pomiar 2026-11-13, po `fetch --prune`

| Pole | Wartość |
|---|---|
| `HEAD` | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| `main` | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| `origin/main` | `8a7c745d5707c5aecd118b92d398dc3dbf52e831` |
| `main...origin/main` (ahead/behind) | `0 / 0` |
| Zmiany śledzone | **brak** (0 staged, 0 unstaged) |
| `git diff --check` | EXIT 0 |
| `git diff --cached --check` | EXIT 0 |
| Pliki nieśledzone i nieignorowane | `?? scratch/` (wyłącznie katalog lokalny) |
| Pliki ignorowane | `target/`, `dist/`, `SPLogbook-backup/`, wzorce `*.log` / `*.py` w `scratch/` |
| Czystość ścisła | **FALSE** — `STRICT_WORKTREE_CLEAN=false`, `TRACKED_WORKTREE_CLEAN=true`, `UNTRACKED_SCRATCH_PRESENT=true` |
| Worktree | dokładnie jeden: `C:/Users/sp6in/Documents/SPLogbook/SPLogbook` |
| Aktywna operacja Git | brak (`MERGE_HEAD`, `rebase-merge`, `rebase-apply`, `CHERRY_PICK_HEAD` nie istnieją) |
| Stash | 2 historyczne wpisy (`clubs_csv_untracked`, `uncommitted pre-stage-10-work`) — nietknięte |

`scratch/` nie jest commitowany w tej bramce. Nie utworzono nowego worktree.

### 1.2. Tagi, release, publikacja

| Pole | Wartość |
|---|---|
| `v1.2.0` (obiekt) | `3ea8d51aab436805107a1cf422e0b35cc301458f` — `VERIFIED` |
| `v1.2.0` (peeled commit) | `6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592` — `VERIFIED` |
| `refs/tags/v1.2.0` na `origin` | `3ea8d51aab436805107a1cf422e0b35cc301458f` — identyczny |
| Nowy tag w tej bramce | **BRAK** (i nie zostanie utworzony) |
| GitHub Releases | `SPLogbook v1.2.0` (2026-10-26) i wcześniejsze; **brak nowego release** |
| Publikacja artefaktów | **BRAK**; `Publish Release Assets` musi pozostać `skipped` |

### 1.3. Stan poprzednich bramek (wejściowy)

| Bramka | Status |
|---|---|
| M3-G AppImage | `COMPLETED_APPIMAGE_INTEGRATION_VERIFIED` — `CLOSED` |
| M3-H DEB | `COMPLETED_DEB_INTEGRATION_VERIFIED` — `CLOSED` |
| F-03 fail-closed | `CLOSED` |
| **M3-I RPM** | **`BLOCKED` / `BLOCKED_BY_NEW_RUSTSEC_ADVISORIES`** |
| M3-J Flatpak | **`NOT_STARTED`** — katalog `installer/linux/flatpak` **nie istnieje** |

`installer/linux/` zawiera wyłącznie `appimage`, `deb`, `portable`, `rpm`. Brak manifestu Flatpak
i brak jakiegokolwiek joba M3-J.

### 1.4. Ostatnie commity (kontekst)

```text
8a7c745 docs(linux): record M3-I RPM verification and RustSec blocker
aa4c9e5 fix(linux): count rpmlint warnings in package-qualified output
3afd3ee fix(linux): install the intentionally unsigned RPM explicitly
...
9427e57 ci: pin the documented Rust toolchain in CI
```

### 1.5. Środowisko pomiarowe

| Pole | Wartość |
|---|---|
| Host | Windows NT 10.0 (x86_64), `Europe/Warsaw` (+01:00) |
| Data hosta | `2026-11-13T18:57` — zgodna z datą projektu; zegara nie zmieniano |
| `rustc` | `1.99.0 (b940084d7 2026-09-28)` |
| `cargo` | `1.99.0 (5f94df478 2026-08-27)` |
| `cargo-audit` | **`cargo-audit-audit 0.22.2`** (wersja przypięta w CI) |
| MSRV projektu | `rust-version = "1.99.0"` (`Cargo.toml:5`) |
| `Cargo.toml` SHA-256 | `4f50f6a67d7273f0646c2b7768f800ec2b55195d781a32ca56123ba94a5863fa` |
| `Cargo.lock` SHA-256 | `2102af83278c59a8a9b9a384f01d3b0f8dc7d6e543608cd9476b17c5b00f91c8` |
| `Cargo.lock` blob Git | `f695aec5357d7b00a805cb8f2d7c3889ffec0893` |
| `Cargo.toml` blob Git | `03b7a62ea5dab8ce89a76a21ec3e478bb874f031` |

Lokalny baseline obowiązuje `-j 1` (limit pliku stronicowania hosta, `os error 1455`).

---

## 2. Dokładny wynik `cargo audit` (stan przed zmianą)

Polecenie dokładnie jak w CI (`.github/workflows/ci.yml:91-94`):

```text
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --deny warnings --no-yanked \
  --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2025-0141 \
  --ignore RUSTSEC-2026-0249 --ignore RUSTSEC-2026-0370 \
  --ignore RUSTSEC-2026-0371 --ignore RUSTSEC-2026-0372 \
  --ignore RUSTSEC-2026-0097
```

Wynik (EXIT **1**):

```text
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1509 security advisories (from C:\Users\sp6in\.cargo\advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (674 crate dependencies)

Crate:     quinn-proto
Version:   0.11.18
Title:     MAX_STREAM_DATA frame can open streams beyond the stream limit
Date:      2026-09-30
ID:        RUSTSEC-2026-0523
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0523
Solution:  Upgrade to >=0.11.19

Crate:     quinn-proto
Version:   0.11.18
Title:     Long Retry token can panic or exhaust memory in a QUIC client
Date:      2026-09-30
ID:        RUSTSEC-2026-0524
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0524
Solution:  Upgrade to >=0.11.19

Crate:     quinn-udp
Version:   0.5.15
Title:     Receive loop can spin after a truncated datagram
Date:      2026-09-30
ID:        RUSTSEC-2026-0538
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0538
Solution:  Upgrade to >=0.5.16, <0.6.0 OR >=0.6.3

error: 3 vulnerabilities found!
warning: 7 allowed warnings found
```

Dodatkowo, to samo polecenie **bez** siedmiu wcześniej zatwierdzonych `--ignore` (dowód, że lista
`--ignore` nie maskuje nowych advisory) zwraca identyczne trzy podatności i EXIT **1**.
Siedem wcześniejszych `--ignore` jest udokumentowaną, zatwierdzoną bazą (CI-002,
`STAGE_10_FINAL_GATE_REPORT.md:115`, strażnik `ci_audit_allowed_advisories_match_documented_baseline`).
**W tej bramce nie dodano żadnego nowego `--ignore` i nie zmieniono listy.**

Pozostałe 7 pozycji to wyłącznie `warning: unmaintained` / `warning: yanked` z zatwierdzonej bazy;
żadna nie jest podatnością i żadna nie należy do zakresu tej bramki.

---

## 3. Wersja `cargo-audit`

```text
$ cargo audit --version
cargo-audit-audit 0.22.2
```

Wersja zgodna z przypięciem w CI (`cargo install cargo-audit --version 0.22.2 --locked`).
Nie aktualizowano narzędzia w tej bramce.

---

## 4. Baza advisory — data i stan

| Pole | Wartość |
|---|---|
| Lokalizacja | `C:\Users\sp6in\.cargo\advisory-db` |
| `HEAD` bazy | `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` |
| Data commita bazy | `2026-11-13T13:33:03+01:00` |
| Temat commita | `Assigned RUSTSEC-2026-0523 to quinn-proto, RUSTSEC-2026-0524 to quinn-proto, ...` |
| Advisory załadowane przez narzędzie | **1509** |
| Zależności w `Cargo.lock` | **674** |

Uwaga metodologiczna: w katalogu bazy znajduje się 6 **nieśledzonych** plików roboczych
(`RUSTSEC-0000-0000.*` w `crates/hickory-*`) — są to szkice bez przypisanego ID, nie wpływają na
skan (narzędzie ich nie raportuje) i nie są częścią repozytorium SPLogbook. Baza jest pobierana
świeżo przy każdym uruchomieniu `cargo audit`; data bazy jest zapisana powyżej.

Baza **nie zawiera** nowszych, nierozwiązanych advisory dla pakietów SPLogbook poza trzema
przedmiotowymi — potwierdza to pełny wynik z §2 (3 vulnerabilities + 7 allowed warnings).

---

## 5. Pełne dane advisory

### 5.1. `RUSTSEC-2026-0523`

| Pole | Wartość |
|---|---|
| Pakiet | `quinn-proto` |
| Wersja podatna (w lockfile) | **`0.11.18`** |
| Wersje naprawione | **`>= 0.11.19`** |
| Wersje nieafektowane | `< 0.11.0` |
| Data advisory | `2026-09-30` |
| Kategorie | `denial-of-service` |
| Słowa kluczowe | `quic`, `stream-limit`, `oom` |
| Aliases | `GHSA-53rm-773f-4q8c` |
| URL | https://rustsec.org/advisories/RUSTSEC-2026-0523 |
| Ścieżka zależności | `splogbook` → `reqwest 0.13.5` (opcjonalny feature `http3`) → `quinn 0.11.12` → `quinn-proto 0.11.18` |

**Charakter:** ramka `MAX_STREAM_DATA` może otworzyć strumienie poza zadeklarowanym limitem;
`accept` może zwrócić ponad 2^40 strumieni → wyczerpanie pamięci, zaprzestanie akceptacji połączeń
albo pętla busy. Dotyczy klientów i serwerów akceptujących strumienie dwukierunkowe inicjowane
przez peera. Poprawka waliduje stream ID w `received_max_stream_data` i dodaje kontrolę
defensywną przy niejawnym otwieraniu strumieni (0.11.19).

**Potencjalny wpływ na SPLogbook:** **brak w obecnym drzewie kompilacji** — SPLogbook nie używa
QUIC ani `reqwest/http3`; `quinn-proto` występuje wyłącznie w `Cargo.lock` jako opcjonalna
zależność `reqwest`, która nie jest aktywowana. Szczegóły i dowód: §6–§9.

### 5.2. `RUSTSEC-2026-0524`

| Pole | Wartość |
|---|---|
| Pakiet | `quinn-proto` |
| Wersja podatna (w lockfile) | **`0.11.18`** |
| Wersje naprawione | **`>= 0.11.19`** |
| Wersje nieafektowane | `< 0.11.0` |
| Data advisory | `2026-09-30` |
| Kategorie | `denial-of-service` |
| Słowa kluczowe | `quic`, `panic`, `oom` |
| Aliases | `GHSA-wppq-2f6r-wfvm` |
| URL | https://rustsec.org/advisories/RUSTSEC-2026-0524 |
| Ścieżka zależności | `splogbook` → `reqwest 0.13.5` (opcjonalny `http3`) → `quinn 0.11.12` → `quinn-proto 0.11.18` |

**Charakter:** klient QUIC przechowuje token z pakietu Retry bez limitu długości i wstawia go do
każdego Initial; przy zbyt długim tokenie `PacketBuilder::new` liczy minimalny rozmiar pakietu
większy niż maksymalny. W buildzie debug → panic; w release → próba alokacji > 10 GiB i abort.
Dotyczy wyłącznie klientów wychodzących; serwery nie przetwarzają Retry. Atakującym może być
serwer, z którym się łączymy, lub obserwator pierwszej paczki Initial.

**Potencjalny wpływ na SPLogbook:** **brak w obecnym drzewie kompilacji** — jak §5.1.

### 5.3. `RUSTSEC-2026-0538`

| Pole | Wartość |
|---|---|
| Pakiet | `quinn-udp` |
| Wersja podatna (w lockfile) | **`0.5.15`** |
| Wersje naprawione | **`>= 0.5.16, < 0.6.0`** **albo** **`>= 0.6.3`** |
| Data advisory | `2026-09-30` |
| Kategorie | `denial-of-service` |
| Słowa kluczowe | `udp`, `infinite-loop` |
| Aliases | `GHSA-6pp4-4cxf-xf88` |
| URL | https://rustsec.org/advisories/RUSTSEC-2026-0538 |
| Ścieżka zależności | `splogbook` → `reqwest 0.13.5` (opcjonalny `http3`) → `quinn 0.11.12` → `quinn-udp 0.5.15` |

**Charakter:** na częściach targetów (Apple bez `fast-apple-datapath`, OpenBSD, NetBSD, Solaris,
illumos; od 0.6 dodatkowo DragonFly BSD, Redox, GNU/Hurd) pętla odbioru ustawia `msghdr` tylko raz
i sprawdza `MSG_TRUNC` przed błędem `recvmsg`. Po odrzuceniu zbyt dużego datagramu kolejne `EAGAIN`
widzi stary `MSG_TRUNC` → pętla busy zamiast `WouldBlock`. **Linux, Android, FreeBSD i Windows
używają innych ścieżek odbioru i nie są dotknięte.** Poprawka przebudowuje nagłówek przed każdym
`recvmsg` i zwraca błędy odbioru przed sprawdzeniem flagi (0.5.16 i 0.6.3).

**Potencjalny wpływ na SPLogbook:** **brak** — pakiet nie jest kompilowany (§9), a wspierane
platformy (Linux x86_64, Windows x64) i tak używają nieafektowanych ścieżek.

---

## 6. `cargo tree -i quinn-proto`

### 6.1. Drzewo aktywne (feature set projektu — `Cargo.toml` bez `http3`)

```text
$ cargo tree -i quinn-proto
warning: nothing to print.

To find dependencies that require specific target platforms, try to use option `--target all` first, ...
```

To samo dla `--target all` i `--all-features` — **`quinn-proto` nie należy do aktywnego grafu
kompilacji**. Nie jest kompilowany dla żadnego targetu obsługiwanego przez projekt.

### 6.2. Ścieżka w lockfile (dowód pochodzenia pozycji w `Cargo.lock`)

Aby pokazać pochodzenie wpisu, aktywowano wyłącznie opcjonalny feature `reqwest/http3`
(**nie jest to zmiana manifestu — wyłącznie flaga odczytu drzewa**):

```text
$ cargo tree -i quinn-proto --features reqwest/http3
quinn-proto v0.11.18
└── quinn v0.11.12
    ├── h3-quinn v0.0.10
    │   └── reqwest v0.13.5
    │       └── splogbook v1.3.0-alpha.1 (C:\Users\sp6in\Documents\SPLogbook\SPLogbook)
    └── reqwest v0.13.5 (*)
```

---

## 7. `cargo tree -i quinn-udp`

### 7.1. Drzewo aktywne

```text
$ cargo tree -i quinn-udp
warning: nothing to print.
```

### 7.2. Ścieżka w lockfile

```text
$ cargo tree -i quinn-udp --features reqwest/http3
quinn-udp v0.5.15
└── quinn v0.11.12
    ├── h3-quinn v0.0.10
    │   └── reqwest v0.13.5
    │       └── splogbook v1.3.0-alpha.1 (C:\Users\sp6in\Documents\SPLogbook\SPLogbook)
    └── reqwest v0.13.5 (*)
```

---

## 8. Wszystkie bezpośrednie i pośrednie zależności wprowadzające podatne pakiety

| Poziom | Pakiet | Wersja | Rola |
|---|---|---|---|
| Bezpośrednia | `reqwest` | `0.13.5` | `Cargo.toml:38`; `default-features = false`, features: `json`, `rustls`, `multipart`, `query`, `form` |
| Pośrednia (opcjonalna `http3`) | `quinn` | `0.11.12` | zależność `reqwest` za feature `http3`; **feature nieaktywowany** |
| Pośrednia (opcjonalna `http3`) | `h3-quinn` | `0.0.10` | zależność `reqwest` za feature `http3`; **feature nieaktywowany** |
| Podatna | `quinn-proto` | `0.11.18` | `quinn` (i `h3-quinn`) → `quinn-proto` |
| Podatna | `quinn-udp` | `0.5.15` | `quinn` → `quinn-udp` |

Dodatkowo `quinn` zależy od `tokio` (już w drzewie) i `rustls` (już w drzewie przez `reqwest/rustls`),
ale sam `quinn` **nie jest** aktywowany. `h3` / `h3-quinn` pojawiają się wyłącznie jako opcjonalne
zależności `reqwest`.

**Cargo.lock jest niezależny od feature'ów** — resolver zapisuje wszystkie opcjonalne zależności
wybranych pakietów, dlatego `cargo audit` (skanujący lockfile) widzi podatne pakiety, mimo że
kompilacja ich nie obejmuje. To jest bezpośrednia przyczyna czerwonego CI.

---

## 9. Czy podatne komponenty są kompilowane / trafiają do binariów

| Pytanie | Odpowiedź | Dowód |
|---|---|---|
| Czy `quinn-proto`/`quinn-udp` są kompilowane? | **NIE** | `cargo tree -i` dla domyślnego, `--all-features` i `--target all` → `nothing to print` |
| Czy trafiają do produkcyjnych binariów? | **NIE** | skan binarki `SPLogbook` (33 105 897 B) z artefaktu CI runu `43469745732`: `quinn` = **0** wystąpień, `quinn-proto` = **0**; `reqwest` = 18 (dowód, że skan działa) |
| Czy są używane wyłącznie warunkowo? | **TAK** | wyłącznie za opcjonalnym feature `http3` pakietu `reqwest`, nieaktywowanym w `Cargo.toml:38` |
| Czy są aktywowane przez konkretne features? | `reqwest/http3` → `quinn`, `h3-quinn` → `quinn-proto`, `quinn-udp` | `cargo tree --features reqwest/http3` |
| Czy są używane na wspieranych platformach? | **NIE** — nie są kompilowane na żadnej platformie; dodatkowo `RUSTSEC-2026-0538` nie dotyczy Linux/Windows | §5.3, §6–§7 |

**Wniosek bezpieczeństwa:** obecna ekspozycja runtime SPLogbook na te trzy advisory = **brak**.
Blokada ma charakter **łańcucha dostaw / higieny lockfile** (`cargo audit` skanuje `Cargo.lock`),
a nie aktywnej podatności w produkcie. Remediacja jest mimo to obowiązkowa, bo CI jest fail-closed.

**Wniosek o zakresie:** usunięcie advisory **nie wymaga** zmian `src/**`, zmian API, zmian
funkcjonalnych ani zmian `Cargo.toml` — wystarczy precyzyjna aktualizacja dwóch wpisów
`Cargo.lock` w tej samej linii semver.

---

## 10. Dostępne warianty naprawy

### Wariant A — celowana aktualizacja lockfile: `quinn-proto 0.11.19` + `quinn-udp 0.5.16` **(WYBRANY)**

| Wymiar | Ocena |
|---|---|
| Zakres | 2 pakiety w `Cargo.lock`; usunięcie duplikatu `windows-sys 0.60.2` (skutek uboczny nowej wersji `quinn-udp`, która akceptuje `windows-sys >=0.52, <=0.61` i wiąże się z już obecnym `0.61.2`) |
| `Cargo.toml` | **bez zmian** — `quinn` 0.11.12 akceptuje `quinn-proto ^0.11` i `quinn-udp ^0.5` |
| `Cargo.lock` | 1 aktualizacja + 1 aktualizacja + usunięcie 10 wpisów duplikatu `windows-sys`/`windows-targets` |
| MSRV | `quinn-proto 0.11.19`: `rust-version = 1.85`; `quinn-udp 0.5.16`: `1.85` → **≤ 1.99.0** |
| API | bez zmian (pakiet nie jest kompilowany; feature nieaktywowany) |
| `src/**` | **bez zmian** |
| Funkcjonalność | bez zmian |
| Rozmiary artefaktów | kod niekompilowany → payload bez zmian; zmieni się wyłącznie `build_commit` w binariach i metadanych → nowe SHA-256 artefaktów (oczekiwane i wymagane) |
| Wszystkie platformy | bez zmian (pakiet niekompilowany na żadnej platformie) |
| Ryzyko regresji | **niskie** — zmiana wyłącznie w obrębie nieaktywnego grafu; pełny baseline i CI jako brama |
| Minimalny diff | **TAK** — 2 precyzyjne aktualizacje, zero zmian manifestu i kodu |

### Wariant B — aktualizacja `quinn` 0.11.12 → nowszy 0.11.x

| Wymiar | Ocena |
|---|---|
| Zakres | `quinn` + potencjalnie `h3-quinn`, `h3` |
| `Cargo.toml` / lock | większy diff; `quinn` jest zależnością tranzytywną, nie ma potrzeby |
| MSRV | prawdopodobnie > 1.85, wymaga weryfikacji |
| Ryzyko | wyższe; zmienia więcej pakietów bez potrzeby |
| Minimalny diff | **NIE** — odrzucony |

### Wariant C — `quinn-udp 0.6.3` zamiast `0.5.16`

| Wymiar | Ocena |
|---|---|
| Zakres | wymaga `quinn` w wersji zależnej od `quinn-udp ^0.6`; `quinn 0.11.12` wymaga `^0.5` |
| Skutek | konflikt lub duplikat `quinn-udp` 0.5 + 0.6 i aktualizacja `quinn` |
| Minimalny diff | **NIE** — odrzucony; `0.5.16` jest wprost dopuszczoną wersją naprawioną |

### Wariant D — usunięcie QUIC z grafu przez zmianę `reqwest` / vendorowanie

| Wymiar | Ocena |
|---|---|
| Zakres | zmiana bezpośredniej zależności (`reqwest`), feature'ów lub vendorowanie lockfile |
| `Cargo.toml` | **zmiana** poza minimalnym zakresem; ryzyko funkcjonalne (TLS/HTTP) |
| Możliwość techniczna | `Cargo.lock` nie pozwala „wyłączyć” opcjonalnej zależności; resolver zawsze ją zapisuje |
| Minimalny diff | **NIE** — odrzucony |

### Wariant E — `--ignore` / suppression / wyłączenie audytu

**ZAKAZANY** przez autoryzację właściciela, `AGENTS.md` §7 i kryteria bramki. Nie brany pod uwagę.

### Wariant F — aktualizacja toolchainu Rust

**NIE jest potrzebna** — oba pakiety deklarują MSRV `1.85`, projekt pozostaje na `1.99.0`.
Toolchain pozostaje przypięty (`Cargo.toml:5`, CI `toolchain: 1.99.0`).

---

## 11. Wybór minimalnego bezpiecznego wariantu

**Wariant A.** Dokładne polecenia (w tej kolejności, pojedynczo):

```text
cargo update -p quinn-proto --precise 0.11.19
cargo update -p quinn-udp  --precise 0.5.16
```

Uzasadnienie:

1. usuwa wszystkie trzy advisory (`quinn-proto >= 0.11.19`, `quinn-udp >= 0.5.16, < 0.6.0`);
2. pozostaje w zgodnej linii semver wymaganej przez `quinn 0.11.12` (bez zmiany `Cargo.toml`);
3. MSRV 1.85 ≤ 1.99.0;
4. nie wymaga zmian `src/**` ani API;
5. nie wprowadza nowych zależności git/path; redukuje duplikat `windows-sys`;
6. nie zmienia funkcjonalności ani układu artefaktów;
7. jest minimalnym diffem lockfile.

Przewidywany diff `Cargo.lock` (z `--dry-run --verbose`):

```text
Updating quinn-proto v0.11.18 -> v0.11.19
Updating quinn-udp   v0.5.15 -> v0.5.16
Removing windows-sys v0.60.2
Removing windows-targets v0.53.5
Removing windows_aarch64_gnullvm v0.53.1
Removing windows_aarch64_msvc v0.53.1
Removing windows_i686_gnu v0.53.1
Removing windows_i686_gnullvm v0.53.1
Removing windows_i686_msvc v0.53.1
Removing windows_x86_64_gnu v0.53.1
Removing windows_x86_64_gnullvm v0.53.1
Removing windows_x86_64_msvc v0.53.1
```

`Cargo.toml` **nie zostanie zmieniony**.

---

## 12. Plan wycofania zmiany

Rollback jest trywialny, ponieważ zmieniany jest wyłącznie `Cargo.lock`:

1. `git revert --no-edit <commit-kokpitu-zaleznosci>` (zakaz resetu/force-push) **albo**
   `git checkout <HEAD-przed> -- Cargo.lock` przed commitem;
2. `cargo metadata --locked` → EXIT 0;
3. `cargo audit` → EXIT 1 z trzema advisory (oczekiwane odtworzenie blokady);
4. pełny baseline `-j 1` → musi wrócić do liczników z §13;
5. brak zmian w `src/**`, więc rollback nie wymaga zmian kodu.

Punkt odniesienia `Cargo.lock` do przywrócenia: blob `f695aec5357d7b00a805cb8f2d7c3889ffec0893`,
SHA-256 `2102af83278c59a8a9b9a384f01d3b0f8dc7d6e543608cd9476b17c5b00f91c8`.

---

## 13. Plan pełnej weryfikacji

| Faza | Zakres | Warunek PASS |
|---|---|---|
| 3 | `cargo audit` (bez nowych `--ignore`), `cargo tree -i` obu pakietów, `cargo tree -d`, `cargo metadata --locked`, źródła pakietów, brak git/path, integralność lockfile | EXIT 0; zero trzech advisory; 674→674 lub mniej pakietów; 0 źródeł innych niż crates.io |
| 4 | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-targets -j 1`, `security_guard`, `cargo doc --workspace --no-deps`, `git diff --check`, walidacja YAML/JSON | 0 failed; bez nowych warningów; liczniki zgodne z `docs/PROJECT_CURRENT_STATE.md` ±0 |
| 5 | testy funkcjonalne i sieciowe istniejące w projekcie (QUIC nieużywany; ścieżki HTTP/update/health/sync/timeout/cancel/offline) | wszystkie zielone; brak sztucznych testów |
| 6 | przebudowa artefaktów w CI: Windows x64, portable Linux, AppImage, DEB, RPM, updater | wszystkie joby `success`; nowe SHA-256 zmierzone |
| 7 | pełna weryfikacja RPM: 2 budowy → identyczny SHA-256, metadane, payload, zależności, RUNPATH, Hamlib, launcher, desktop, ikony, manifest, sumy; rpmlint 0 errors; 34/34 negatywnych; cykl życia Fedora 45 + openSUSE Tumbleweed; dane użytkownika SHA-256 przed/po | wszystkie asercje PASS |
| 8 | regresja Windows / portable / Verify Linux 4 profile / negative / AppImage FUSE / DEB / RPM / F-03 / `security_guard` / testy pakowania | wszystkie joby `success` |
| 9–11 | raport bezpieczeństwa, aktualizacja raportu M3-I i dokumentów stanu **po zielonym CI** | spójność dokumentów z rzeczywistym stanem |

Kryteria zamykające bramkę: `cargo audit` EXIT 0 bez nowych `--ignore`; brak nowych advisory;
Rust 1.99.0; pełny baseline zielony; `security_guard` zielony; wszystkie formaty zielone;
RPM ponownie zweryfikowany (determinizm + 34/34 + cykl życia); finalne `CI` i
`Build and Release SPLogbook` zielone na finalnym HEAD; `Publish Release Assets` = `skipped`;
brak tagu/release/publikacji; `HEAD == main == origin/main`, `0/0`, tracked worktree clean,
`scratch/` niecommitowany; M3-J `NOT_STARTED`.

---

## 14. Ryzyka i ograniczenia tej bramki

| Ryzyko | Mitygacja |
|---|---|
| Baza RustSec może w trakcie bramki ujawnić kolejne advisory | każde nowe advisory = STOP i raport; nie rozszerzamy zakresu bez decyzji OWNER |
| `quinn-udp 0.5.16` usuwa duplikat `windows-sys 0.60.2` | zmiana wyłącznie w lockfile, weryfikowana `cargo metadata --locked`; brak wpływu na kod |
| Artefakty muszą zostać przebudowane, więc ich SHA-256 się zmienią | nowe sumy mierzone z finalnego runu CI; stare sumy nie są przenoszone |
| Brak lokalnego środowiska Linux/OCI | natywna weryfikacja RPM/DEB/AppImage wyłącznie w CI; dowód = joby i logi |
| `commit dokumentacyjny po zielonym CI` zmienia HEAD | raport opisuje code HEAD i run na nim; finalny run na HEAD dokumentacyjnym musi być zielony |
| HIL/PTT/RF | poza zakresem; 10 testów `#[ignore]` pozostaje nieuruchomionych |
| M3-J | `NOT_STARTED`; brak plików Flatpak, brak jobów |

---

## 15. Plan commitów

1. `docs(security): add M3-I RustSec remediation pre-implementation report` — ten plik;
2. `fix(deps): remediate RUSTSEC-2026-0523/0524/0538` — wyłącznie `Cargo.lock`;
3. `test(...)` — **tylko jeśli** okaże się konieczny test regresyjny (nie jest planowany:
   pakiet nie jest kompilowany i nie ma aktywnej ścieżki funkcjonalnej);
4. `docs(security): record M3-I RustSec remediation verification` — raport bezpieczeństwa;
5. `docs(linux): unblock and close M3-I after RustSec remediation` — raport M3-I i dokumenty stanu,
   **wyłącznie po zielonym CI na finalnym HEAD**.

Push wyłącznie fast-forward, po zielonym `cargo audit` i pełnym baseline.

---

## 16. Kryteria wejścia do implementacji

- [x] autoryzacja `OWNER_GO_M3_I_SECURITY_REMEDIATION=true`,
- [x] `HEAD == main == origin/main` = `8a7c745…`, `0/0`,
- [x] tracked worktree clean, `scratch/` nieśledzony,
- [x] advisory potwierdzone rzeczywistym wynikiem `cargo audit` (EXIT 1) na bazie
      `4ef2df8e…` z 2026-11-13,
- [x] wersje naprawione dostępne i zgodne z MSRV 1.99.0,
- [x] brak potrzeby zmian `src/**` i `Cargo.toml`,
- [x] plan rollbacku i plan weryfikacji gotowe,
- [x] M3-J `NOT_STARTED`, brak nowego tagu/release.

**Status po tym raporcie:** gotowe do Fazy 2 (minimalna aktualizacja zależności).
