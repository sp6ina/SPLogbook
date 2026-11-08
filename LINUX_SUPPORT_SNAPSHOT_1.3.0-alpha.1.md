# Oficjalny Linux Support Snapshot: SPLogbook 1.3.0-alpha.1
# Dokument: LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md

---

## 1. Metadane Snapshotu
- **Snapshot Schema:** `splogbook-linux-support-snapshot-v1`
- **Data i czas wyznaczenia (UTC):** `2026-11-08T18:41:25Z`
- **Wersja produktu:** `1.3.0-alpha.1`
- **Commit bazowy:** `9838dd261973a77faac9cd95fe59db3fb8048683`
- **Architektura:** `x86_64` (AMD64 / Intel 64)
- **Rust Target:** `x86_64-unknown-linux-gnu`
- **Wersja Rust:** `1.99.0`
- **Wersja Cargo:** `1.99.0`
- **Linker:** GNU `ld` 2.42 / LLVM `22.1.8`
- **Nazwa kanonicznego artefaktu:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **SHA-256 artefaktu:** `4b68e7f53a9482b813d1e1fa46c268f7739e1a8bb2cd30a38115682974fa2e51`
- **Liczba elementów w archiwum (Entry Count):** `12`

---

## 2. Dynamicznie Wyliczone Minimum Kernela (Dynamic Kernel Floor)

Zgodnie z wiążącą decyzją `OWNER-M3-F-KERNEL-POLICY-01`, minimalna wersja kernela nie jest ustalona na stałe, lecz podlega dynamicznemu przeliczeniu dla każdego wydania SPLogbook jako najniższa domyślna wersja kernela w obowiązującej macierzy:

```text
RESOLVED_MINIMUM_KERNEL = MINIMUM(
    Ubuntu stable (7.3),
    Ubuntu LTS (7.0),
    Debian stable (6.12),
    Fedora Final (6.18),
    Arch Linux snapshot (6.18),
    openSUSE Leap (6.12),
    openSUSE Tumbleweed snapshot (6.18)
) = Linux 6.12
```

- **RESOLVED MINIMUM KERNEL:** `Linux 6.12` (dokładnie Linux 6.12 LTS)
- **Dystrybucje wyznaczające minimum:** `Debian 13.7 (Trixie)`, `openSUSE Leap 16.1`
- **Bieżący najnowszy stabilny upstream Linux kernel:** `Linux 6.18`
- **Stałe wieczne minimum:** `NO` (wartość dynamiczna wyliczana na nowo w każdym wydaniu z wejść macierzy)

---

## 3. Zestawienie Profili Obowiązkowej Macierzy Dystrybucyjnej

| Profil Dystrybucji | Model Wydania | Oficjalna Wersja / Codenam | Domyślny Kernel | Wersja glibc | Oficjalny Obraz OCI | Przypięty Digest Obrazu | Status Dowodowy |
|---|---|---|---|---|---|---|---|
| **Ubuntu Stable** | Classic (Interim) | Ubuntu 26.10 (Stonking Stingray) | `7.3.0-12-generic` | `2.44` | `ubuntu:26.10` | `sha256:262df65c0ec6bf8d9dcc87454f4330a05fae4b97825f45854af3b136666584c2` | VERIFIED |
| **Ubuntu LTS** | LTS | Ubuntu 26.04.1 LTS (Resolute Raccoon) | `7.0.0-18-generic` | `2.43` | `ubuntu:26.04` | `sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` | VERIFIED |
| **Debian Stable** | Classic | Debian 13.7 (Trixie) | `6.12.0-12-amd64` | `2.40` | `debian:13.7` | `sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6` | VERIFIED (KERNEL FLOOR) |
| **Fedora Final/GA** | Classic | Fedora Linux 45 (Final/GA) | `6.18.1-300.fc45` | `2.44` | `fedora:45` | `sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c` | VERIFIED |
| **Arch Linux Rolling** | Rolling | Snapshot 2026.11.01 (official stable repos) | `6.18.2-arch1-1` | `2.44-2` | `archlinux:base-20261101.0.620205` | `sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9` | VERIFIED (ROLLING FORWARD) |
| **openSUSE Leap** | Classic | Leap 16.1 (final-supported) | `6.12.0-160100.14-default` | `2.40` | `opensuse/leap:16.1` | `sha256:756634f02c733fc52d5c9aa09aca6374c91c53c675dd445743736e13a8428858` | VERIFIED (KERNEL FLOOR) |
| **openSUSE Tumbleweed** | Rolling | Snapshot 20261103 (official published snapshot) | `6.18.2-1-default` | `2.44` | `opensuse/tumbleweed:latest` | `sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1` | VERIFIED (ROLLING FORWARD) |
| **Resolved Min Kernel** | Compatibility | Kernel Floor Compatibility Sandbox | `6.12 LTS` | `2.40` | `debian:13.7` | `sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6` | VERIFIED |
| **Latest Stable Kernel**| Upstream | Upstream Stable Kernel Sandbox | `6.18 Stable` | `2.44` | `archlinux:base-20261101.0.620205` | `sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9` | VERIFIED |

---

## 4. Analiza Zgodności ABI i glibc (Common Compatibility Intersection)
- Wymagania symboli biblioteki standardowej C dla uniwersalnego artefaktu wydawniczego są ograniczone do poziomu nie wyższego niż najstarsza glibc w macierzy (`glibc <= 2.40`).
- Przy budowie w kanonicznym kontrolowanym środowisku bazowym (sysroot `glibc 2.35`), generowane binaria linkują wyłącznie do symboli w wersjach `GLIBC_2.2.5` - `GLIBC_2.35`.
- Ponieważ biblioteka `glibc` gwarantuje 100% kompatybilności wstecznej symboli, binarka skompilowana z bazą symboli `GLIBC_2.35` ładuje się i wykonuje bezbłędnie na każdym profilu macierzy (`glibc 2.40`, `2.43`, `2.44`).
- **Wniosek ABI:** Jeden kanoniczny artefakt Linux portable tar.gz o identycznym skrócie SHA-256 spełnia warunek uniwersalności na wszystkich 7 obowiązkowych profilach dystrybucyjnych bez konieczności tworzenia wariantów dedykowanych.
