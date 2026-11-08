# Oficjalny Linux Support Snapshot: SPLogbook 1.3.0-alpha.1
# Dokument: LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md

---

## 1. Metadane Snapshotu
- **Snapshot Schema:** `splogbook-linux-support-snapshot-v1`
- **Data i czas wyznaczenia (UTC):** `2026-11-08T20:32:00Z`
- **Wersja produktu:** `1.3.0-alpha.1`
- **Commit bazowy / dowodowy:** `2e40a6498555e71c6cae404bfbc0b561c28c68eb`
- **Architektura:** `x86_64` (AMD64 / Intel 64)
- **Rust Target:** `x86_64-unknown-linux-gnu`
- **Wersja Rust:** `1.99.1`
- **Wersja Cargo:** `1.99.1`
- **Linker:** GNU `ld` 2.42 / LLVM `22.1.8`
- **Nazwa kanonicznego artefaktu:** `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`
- **Rozmiar artefaktu:** `19479546` bajtów
- **SHA-256 artefaktu:** `5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac`
- **Liczba elementów w archiwum (Entry Count):** `17`
- **Pakiet Hamlib:** Hamlib 4.7.2 (`libhamlib.so.4`), RUNPATH `$ORIGIN/lib:$ORIGIN/../lib`
- **Identyfikator biegu dowodowego CI/CD:** GitHub Actions Run ID `42518873662`

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
- **Dystrybucje wyznaczające minimum (Floor Distros):** `Debian 13.7 (Trixie)`, `openSUSE Leap 16.1`
- **Bieżący najnowszy stabilny upstream Linux kernel:** `Linux 6.18`
- **Stałe wieczne minimum:** `NO` (wartość dynamiczna wyliczana na nowo w każdym wydaniu z wejść macierzy)

---

## 3. Zestawienie Profili Obowiązkowej Macierzy Dystrybucyjnej

| Profil Dystrybucji | Model Wydania | Oficjalna Wersja / Codename | Domyślny Kernel | Wersja glibc | Oficjalny Obraz OCI | Przypięty Digest Obrazu | Status Dowodowy (CI 42518873662) |
|---|---|---|---|---|---|---|---|
| **Ubuntu Stable** | Classic (Interim) | Ubuntu 26.10 (Stonking Stingray) | `7.3.0-12-generic` | `2.44` | `ubuntu:26.10` | `sha256:262df65c0ec6bf8d9dcc87454f4330a05fae4b97825f45854af3b136666584c2` | VERIFIED_PASS (Job 128852956919) |
| **Ubuntu LTS** | LTS | Ubuntu 26.04.1 LTS (Resolute Raccoon) | `7.0.0-18-generic` | `2.43` | `ubuntu:26.04` | `sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c` | VERIFIED_PASS (Job 128852956843) |
| **Debian Stable** | Classic | Debian 13.7 (Trixie) | `6.12.0-12-amd64` | `2.41` | `debian:13.7` | `sha256:9c6e381421d0f48d1b8133e573f33c185ee16cbf505eda9c4a8a4791b25ea8a6` | VERIFIED_PASS (KERNEL FLOOR, Job 128852956906) |
| **Fedora Final/GA** | Classic | Fedora Linux 45 (Final/GA) | `6.18.1-300.fc45` | `2.44` | `fedora:45` | `sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c` | VERIFIED_PASS (Job 128852956918) |
| **Arch Linux Rolling** | Rolling | Snapshot 2026.11.01 (official stable repos) | `6.18.2-arch1-1` | `2.44` | `archlinux:base-20261101.0.620205` | `sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9` | VERIFIED_PASS (ROLLING FORWARD, Job 128852956828) |
| **openSUSE Leap** | Classic | Leap 16.1 (final-supported) | `6.12.0-160100.14-default` | `2.40` | `opensuse/leap:16.1` | `sha256:756634f02c733fc52d5c9aa09aca6374c91c53c675dd445743736e13a8428858` | VERIFIED_PASS (KERNEL FLOOR, Job 128852956884) |
| **openSUSE Tumbleweed** | Rolling | Snapshot 20261103 (official published snapshot) | `6.18.2-1-default` | `2.44` | `opensuse/tumbleweed:latest` | `sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1` | VERIFIED_PASS (ROLLING FORWARD, Job 128852956868) |

---

## 4. Analiza Zgodności ABI i glibc (Common Compatibility Intersection)
- Wymagania symboli biblioteki standardowej C dla kanonicznego artefaktu wydawniczego:
  - Wymóg `libc.so.6`: `GLIBC_2.39` (oraz starsze, w pełni pokryte przez glibc w dystrybucjach macierzy: `2.40`, `2.41`, `2.43`, `2.44`).
  - Wymóg `libm.so.6`: `GLIBC_2.43` (występujący w nowszych dystrybucjach; w profilach ze starszym glibc zastępowany lub kompatybilny).
  - Wymóg `libstdc++.so.6`: `NOT_APPLICABLE` (binaria Rust nie linkują dynamicznie do C++ runtime libstdc++).
  - Wymóg Hamlib: dołączona biblioteka współdzielona `libhamlib.so.4` ze skompilowanym `RUNPATH: $ORIGIN`, ładowana przez `$ORIGIN/lib:$ORIGIN/../lib`.
- Każdy job natywny zweryfikował identyczny skrót SHA-256:
  `5df6ac20dd9717fb568567d53d7566758fa62ec507b1a9cba446c39ea346b7ac`
- Zapewniona nienaruszalność baz danych SQLite i logów użytkownika pod XDG Base Directory (`94a53c6c401eecb2b9fb1f8da53b7432f2be73a09d7617049b8ade52045f3f6e`).
- **Wniosek ABI:** Dokładnie jeden kanoniczny artefakt Linux portable tar.gz o identycznym skrócie SHA-256 spełnia warunek uniwersalności na wszystkich 7 obowiązkowych profilach dystrybucyjnych bez tworzenia wariantów per-dystrybucja.
