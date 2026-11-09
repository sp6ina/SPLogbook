# Oficjalny Linux Support Snapshot: SPLogbook 1.3.0-alpha.1
# Dokument: LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md

---

## 1. Metadane Snapshotu

- **Snapshot Schema:** `splogbook-linux-support-snapshot-v2`
- **Data snapshotu (projekt):** `2026-11-09`
- **Data i czas wyznaczenia (UTC):** `2026-11-09T13:28:29Z`
- **Wersja produktu:** `1.3.0-alpha.1`
- **Architektura:** `x86_64` (AMD64 / Intel 64)
- **Rust Target:** `x86_64-unknown-linux-gnu`
- **Wersja Rust (artefakt):** `1.99.1`
- **Nazwa kanonicznego artefaktu:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **Rozmiar artefaktu:** `19500951` bajtów
- **SHA-256 artefaktu:** `9b31fd2437b89a80d58784338f4702bbe175d84907824d635a9a4caa9002127c`
- **Commit budujący artefakt:** `abdb767d41bd07da1304c1c8f5d8993726a6575a`
- **Liczba elementów archiwum:** `17`
- **Pakiet Hamlib:** Hamlib 4.7.2 (`libhamlib.so.4`), RUNPATH `$ORIGIN/lib:$ORIGIN/../lib`
- **Środowisko budowania:** Ubuntu 26.04.1 LTS, runner `ubuntu-latest` (**nieprzypięty** — ryzyko rezydualne)

---

## 2. Polityka Kernela — `OWNER-M3-F-KERNEL-POLICY-OPTION-A`

Obowiązuje decyzja OWNER **OPTION A** (2026-11-09), która zastępuje zarówno dynamiczny floor
`6.12`, jak i sztywne minimum `7.2` (`OWNER_APPROVED_FIXED_MINIMUM`).

Zasada: minimalna obsługiwana linia kernela to **najstarsza domyślna linia kernela spośród
aktualnych stabilnych wydań dystrybucji oficjalnie wspieranych**. Kanały `testing`, `rawhide`,
`backports`, repozytoria eksperymentalne, niestabilne snapshoty i release candidates **nie**
wyznaczają minimum. Wersja kernela w CI nie jest automatycznie minimalną wersją runtime.

```text
MINIMUM_KERNEL = MIN(
    Ubuntu 26.04 LTS  7.0,
    Fedora 45         7.2,
    Arch Linux        7.2,
    openSUSE Tumbleweed 7.2
) = Linux 7.0
```

- **RESOLVED MINIMUM KERNEL:** `Linux 7.0`
- **Dystrybucja wyznaczająca minimum:** `Ubuntu 26.04.1 LTS`
- **Najnowsza wspierana linia kernela:** `Linux 7.2`
- **Stałe wieczne minimum:** `NO` — wartość jest przeliczana przy każdym wydaniu i przy każdej
  zmianie domyślnego kernela wspieranej dystrybucji (`revision_policy`).
- **Zakaz podnoszenia minimum bez dowodu technicznego:** tak.

### 2.1. Dlaczego Debian 13.7 i openSUSE Leap nie są wspierane

Artefakt wymaga maksymalnego symbolu `GLIBC_2.43`. Debian 13.7 ma glibc `2.41`, a openSUSE Leap
16.1 — `2.40`. Obie dystrybucje są **starsze ABI** niż wymagane, więc nie mogą uruchomić
kanonicznego artefaktu i nie wchodzą do zbioru wspieranego. Są ewidencjonowane w sekcji
`evaluated_not_supported`, a nie usuwane z analizy.

Gdyby OWNER zdecydował o wsparciu Debiana, minimum kernela spadłoby do `6.12`, ale wymagałoby to
**przebudowy artefaktu pod glibc ≤ 2.41** — poza zakresem tej bramki.

---

## 3. Polityka glibc — `OWNER-M3-F-GLIBC-POLICY-01`

- **Minimalna glibc:** `2.43`
- **Maksymalny wymagany symbol:** `GLIBC_2.43`
- **Bundled glibc:** `NIE`
- **Przebudowa do 2.41:** `NIE`
- Najstarsza glibc wśród wspieranych dystrybucji: `2.43` (Ubuntu 26.04 LTS) — zgodna z wymaganiem
  artefaktu.

---

## 4. Zestawienie Profili Wspieranych

| Profil | Model | Wersja / snapshot | Linia kernela | glibc | Domyślna sesja | Źródło |
|---|---|---|---|---|---|---|
| **Ubuntu LTS** | LTS | 26.04.1 LTS (Resolute Raccoon) | `7.0` (`7.0.0-42-generic`) | `2.43` | GNOME / Wayland | packages.ubuntu.com/resolute |
| **Fedora Final** | Classic | Fedora Linux 45 (Final/GA) | `7.2` (`7.2.8-300.fc45`) | `2.44` | GNOME / Wayland | packages.fedoraproject.org |
| **Arch Linux** | Rolling | core/linux `7.2.14.arch1-1` | `7.2` | `2.44` | wybór użytkownika (X11/Wayland) | archlinux.org/packages/core/x86_64/linux |
| **openSUSE Tumbleweed** | Rolling | snapshot `20261021` | `7.2` (`7.2.11-1-default`) | `2.44` | KDE Plasma / GNOME | news.opensuse.org |

Wszystkie profile: `x86_64`, systemd, biblioteki wymagane przez SPLogbook dostępne w oficjalnych
repozytoriach.

## 5. Profile Zbadane i Niewspierane

| Profil | Wersja | Kernel | glibc | Status | Powód |
|---|---|---|---|---|---|
| Debian stable | 13.7 (Trixie) | `6.12` | `2.41` | `NOT_SUPPORTED` | glibc < wymagane `GLIBC_2.43` |
| openSUSE Leap | 16.1 | `6.12` | `2.40` | `NOT_SUPPORTED` | glibc < `GLIBC_2.43`; poza zakresem wsparcia |
| Ubuntu interim | 26.10 | `7.3` | `2.44` | `OUT_OF_SCOPE` | wydanie interim; polityka obejmuje tylko LTS |

---

## 6. Analiza ABI

- `libc.so.6`: maksymalny wymagany symbol `GLIBC_2.39`.
- `libm.so.6`: maksymalny wymagany symbol `GLIBC_2.43` — **to ograniczenie wyznacza minimum glibc**.
- `libstdc++.so.6`: `NOT_APPLICABLE` (Rust, brak linkowania C++).
- Hamlib: dołączony `libhamlib.so.4`, RUNPATH `$ORIGIN/lib:$ORIGIN/../lib`.
- Minimum glibc `2.43` jest zgodne ze wszystkimi czterema wspieranymi profilami.

---

## 7. Status Dowodów Natywnych

**Status:** `FAIL_CLOSED_VERIFIED` (run `42685105375`, commit `abdb767`, 2026-11-09).

Kanoniczny artefakt `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
(SHA-256 `9b31fd2437b89a80d58784338f4702bbe175d84907824d635a9a4caa9002127c`,
`19500951` B, 17 wpisów) został zweryfikowany w 4 przypiętych obrazach OCI:

| Profil | Biblioteki runtime | `not found` | `--version` | `--build-info` | GLIBC max | Wynik |
|---|---|---|---|---|---|---|
| Ubuntu 26.04.1 LTS | zainstalowane (apt) | **0** | exit 0 | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.43) | PASS |
| Fedora 45 | zainstalowane (dnf) | **0** | exit 0 | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |
| Arch Linux | zainstalowane (pacman) | **0** | exit 0 | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |
| openSUSE Tumbleweed | zainstalowane (zypper) | **0** | exit 0 | exit 0, `abdb767…` | `GLIBC_2.43` ≤ 2.43 (dostępne 2.44) | PASS |

**Test negatywny:** goły obraz `ubuntu:26.04` bez bibliotek runtime →
`libasound.so.2 => not found`, `FAIL: ldd reports an unresolved shared library`, **exit 1**,
brak komunikatu sukcesu. `Publish Release Assets` = `skipped`; brak nowego tagu i release.

Runy `42518873662` (7 profili) i `42638736785` (4 profile) są **historycznym dowodem legacy
non-fail-closed** (wszystkie profile: `libasound.so.2 => not found` przy komunikacie sukcesu)
i nie stanowią dowodu natywnego PASS.

---

## 8. Zasada Rewizji

1. Snapshot jest przeliczany przy każdym wydaniu produktu.
2. Zmiana domyślnego kernela lub glibc wspieranej dystrybucji wymaga aktualizacji snapshotu.
3. Minimum nie może być podniesione bez dowodu technicznego (wymóg API, ABI albo biblioteki).
4. Kanały niestabilne nie wyznaczają minimum.
5. Data snapshotu i źródła oficjalne są obowiązkowe dla każdej wartości.
