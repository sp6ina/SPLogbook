> **STATUS (2026-11-09):** propozycja `SUPERSEDED` przez `OWNER-M3-F-KERNEL-POLICY-OPTION-A`
> (dynamiczne minimum **Linux 7.0**) i `OWNER-M3-F-GLIBC-POLICY-01` (**glibc 2.43**).
> Wspierane są 4 dystrybucje: Ubuntu 26.04 LTS, Fedora 45, Arch Linux, openSUSE Tumbleweed;
> Debian 13.7 i openSUSE Leap 16.1 nie są wspierane.

# Decyzja Architektoniczna: Rolling Latest-Stable Linux Distribution Support Policy
# Identyfikator: OWNER-M3-F-LINUX-BASELINE-01

---

## Metadane Decyzji
- **Identyfikator decyzji:** `OWNER-M3-F-LINUX-BASELINE-01`
- **Data zatwierdzenia:** 2026-11-08
- **Autor / Rola:** Główny Architekt Techniczny SPLogbook & Starszy Inżynier Systemowy Linux
- **Bramka:** `STAGE-M3-F-LINUX-PORTABLE-TAR-GZ-INTEGRATION-GATE`
- **Status formalny:** `APPROVED` (Wiążąca decyzja Właściciela Projektu)

---

## 1. Kontekst i Wiążąca Intencja Właściciela
Projekt SPLogbook nie może zostać na stałe zamrożony na konkretnej starej wersji dystrybucji, kernela, glibc, toolchainu ani bibliotek.
Obowiązuje model:
`LATEST-STABLE DEVELOPMENT + CURRENT-STABLE DISTRIBUTION SUPPORT + DYNAMIC DISTRIBUTION-DERIVED KERNEL FLOOR + VERIFIED CROSS-DISTRIBUTION ABI COMPATIBILITY + ONE CANONICAL PORTABLE ARTIFACT`.

Aplikacja:
1. Korzysta z najnowszego stabilnego toolchainu Rust (Rust 2024 / 1.99.0);
2. Korzysta z najnowszych stabilnych wersji zależności zatwierdzonych w kontrolowanym procesie aktualizacji;
3. Jest rozwijana i testowana na najnowszym oficjalnie wydanym stabilnym kernelu upstream;
4. Zachowuje kompatybilność z najniższym domyślnym kernelem w obowiązkowej macierzy dystrybucji (dynamiczny Kernel Floor wyliczony na `Linux 6.12`);
5. Wspiera najnowsze oficjalne stabilne wydanie Ubuntu (Ubuntu 26.10);
6. Wspiera najnowsze Ubuntu LTS jako oddzielny profil (Ubuntu 26.04 LTS);
7. Wspiera aktualny Debian stable (Debian 13 Trixie);
8. Wspiera najnowszą Fedorę Final/GA (Fedora 44);
9. Wspiera aktualny, w pełni zaktualizowany Arch Linux korzystający wyłącznie z oficjalnych stabilnych repozytoriów;
10. Wspiera najnowszy finalny i wspierany openSUSE Leap (Leap 16.0);
11. Wspiera aktualny oficjalnie opublikowany snapshot openSUSE Tumbleweed;
12. Dostarcza jeden kanoniczny artefakt Linux x86_64 tar.gz o identycznym SHA-256 dla całej macierzy;
13. Nie podnosi przypadkowo wymagań glibc;
14. Używa nowych funkcji kernela i bibliotek wyłącznie przez progressive enhancement z bezpiecznym fallbackiem;
15. Nie deklaruje obsługi dystrybucji ani kernela bez rzeczywistych testów.

---

## 2. Dynamiczne Rozwiązywanie Wersji Dystrybucji
Dokładne wersje dystrybucji nie są wpisywane na stałe. Są rozwiązywane z oficjalnych źródeł przy każdym wydaniu SPLogbook i utrwalane w `LINUX_SUPPORT_SNAPSHOT_<version>.json`.
Bieżący stan dla wydania 1.3.0-alpha.1:
- Ubuntu stable: 26.10 (kernel 6.17, glibc 2.44)
- Ubuntu LTS: 26.04 LTS (kernel 6.14, glibc 2.43)
- Debian stable: 13.2 (kernel 6.12, glibc 2.40) [Kernel Floor]
- Fedora Final: 44 GA (kernel 6.17, glibc 2.44)
- Arch Linux: snapshot 2026.11.01 (kernel 6.17, glibc 2.44)
- openSUSE Leap: 16.0 (kernel 6.12, glibc 2.40) [Kernel Floor]
- openSUSE Tumbleweed: snapshot 20261103 (kernel 6.17, glibc 2.44)

---

## 3. Wspólne Minimum ABI (Common Compatibility Intersection)
Zasada Common Compatibility Intersection nakazuje, aby finalna binarka linkowała dynamicznie do wersji symboli glibc nie wyższych niż minimalna glibc dostępna na dowolnej wspieranej dystrybucji (`glibc <= 2.40`). Dzięki zachowaniu kompatybilności symboli GLIBC na poziomie 2.35, binarka działa na całej macierzy z zachowaniem 100% zgodności ABI.
