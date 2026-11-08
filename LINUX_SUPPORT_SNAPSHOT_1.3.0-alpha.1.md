# Oficjalny Linux Support Snapshot: SPLogbook 1.3.0-alpha.1
# Dokument: LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md

---

## 1. Metadane Snapshotu
- **Snapshot Schema:** `splogbook-linux-support-snapshot-v1`
- **Data i czas wyznaczenia (UTC):** `2026-11-08T15:32:00Z`
- **Wersja produktu:** `1.3.0-alpha.1`
- **Commit bazowy:** `a8541075e14de1537b7940fa3fd48b44d854151f`
- **Architektura:** `x86_64` (AMD64 / Intel 64)
- **Rust Target:** `x86_64-unknown-linux-gnu`
- **Wersja Rust:** `1.99.0`
- **Wersja Cargo:** `1.99.0`
- **Linker:** GNU `ld` / LLVM `22.1.8`
- **Nazwa kanonicznego artefaktu:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **SHA-256 artefaktu:** `3cb49a5b3a6ef4167e43d9aa360812739fa4be0429713606a46973685e13d1ee`

---

## 2. Dynamicznie Wyliczone Minimum Kernela (Dynamic Kernel Floor)

Zgodnie z wiążącą decyzją `OWNER-M3-F-KERNEL-POLICY-01`, minimalna wersja kernela nie jest ustalona na stałe, lecz podlega przeliczeniu dla każdego wydania SPLogbook jako najniższa domyślna wersja kernela w obowiązującej macierzy:

```text
RESOLVED_MINIMUM_KERNEL = MINIMUM(
    Ubuntu stable (6.17),
    Ubuntu LTS (6.14),
    Debian stable (6.12),
    Fedora Final (6.17),
    Arch Linux snapshot (6.17),
    openSUSE Leap (6.12),
    openSUSE Tumbleweed snapshot (6.17)
) = Linux 6.12
```

- **RESOLVED MINIMUM KERNEL:** `Linux 6.12` (dokładnie Linux 6.12.38 LTS)
- **Dystrybucje wyznaczające minimum:** `Debian 13 (Trixie)`, `openSUSE Leap 16.0`
- **Bieżący najnowszy stabilny upstream Linux kernel:** `Linux 6.17.8`
- **Stałe wieczne minimum:** `NO` (wartość dynamiczna wyliczana na nowo w każdym wydaniu)

---

## 3. Zestawienie Profili Obowiązkowej Macierzy Dystrybucyjnej

| Profil Dystrybucji | Model Wydania | Oficjalna Wersja / Snapshot | Domyślny Kernel | Wersja glibc | Status Profilu |
|---|---|---|---|---|---|
| **Ubuntu Stable** | Classic | Ubuntu 26.10 (snapshot 20261022) | `6.17.0-14-generic` | `2.44` | VERIFIED |
| **Ubuntu LTS** | LTS | Ubuntu 26.04 LTS (snapshot 20260423) | `6.14.0-21-generic` | `2.43` | VERIFIED |
| **Debian Stable** | Classic | Debian 13.2 (Trixie, snapshot 20261011) | `6.12.0-9-amd64` | `2.40` | VERIFIED (KERNEL FLOOR) |
| **Fedora Final/GA** | Classic | Fedora 44 Final/GA (snapshot 20261028) | `6.17.6-300.fc44` | `2.44` | VERIFIED |
| **Arch Linux Rolling** | Rolling | Snapshot 2026.11.01 (official stable repos only) | `6.17.7-arch1-1` | `2.44-2` | VERIFIED (ROLLING FORWARD) |
| **openSUSE Leap** | Classic | Leap 16.0 (final-supported, snapshot 20260930) | `6.12.0-160000.12` | `2.40` | VERIFIED (KERNEL FLOOR) |
| **openSUSE Tumbleweed** | Rolling | Snapshot 20261103 (official published snapshot) | `6.17.7-1-default` | `2.44` | VERIFIED (ROLLING FORWARD) |
| **Resolved Min Kernel** | Compatibility | Kernel Floor Compatibility Sandbox | `6.12.38 LTS` | `2.40` | VERIFIED |
| **Latest Stable Kernel**| Upstream | Upstream Stable Kernel Sandbox | `6.17.8 Stable` | `2.44` | VERIFIED |

---

## 4. Analiza Zgodności ABI i glibc (Common Compatibility Intersection)
- Wymagania symboli biblioteki standardowej C dla uniwersalnego artefaktu wydawniczego są ograniczone do poziomu nie wyższego niż najstarsza glibc w macierzy (`glibc <= 2.40`).
- Przy budowie w kanonicznym kontrolowanym środowisku bazowym (np. Ubuntu 22.04 LTS / sysroot `glibc 2.35`), generowane binaria linkują wyłącznie do symboli w wersjach `GLIBC_2.2.5` - `GLIBC_2.35`.
- Ponieważ biblioteka `glibc` gwarantuje 100% kompatybilności wstecznej symboli, binarka skompilowana z bazą symboli `GLIBC_2.35` ładuje się i wykonuje bezbłędnie na każdym profilu macierzy (`glibc 2.40`, `2.43`, `2.44`).
- **Wniosek ABI:** Jeden kanoniczny artefakt Linux portable tar.gz o identycznym skrócie SHA-256 spełnia warunek uniwersalności na wszystkich 7 obowiązkowych profilach dystrybucyjnych bez konieczności tworzenia wariantów dedykowanych.
