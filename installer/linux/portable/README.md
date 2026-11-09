# SPLogbook Linux Portable tar.gz Packaging (M3-F)

---

## 1. Przeznaczenie i Zakres
Katalog `installer/linux/portable/` zawiera definicje, szablony oraz specyfikację pakietu przenośnego `SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz`.

Pakiet jest przeznaczony do uruchamiania w przestrzeni użytkownika bez uprawnień administracyjnych (root/sudo) na wspieranej macierzy Linux x86_64 (**kernel >= 7.0, glibc >= 2.43**):
- Ubuntu 26.04 LTS
- Fedora Linux 45
- Arch Linux rolling (wyłącznie oficjalne repozytoria)
- openSUSE Tumbleweed (snapshot oficjalny)

Debian 13.7 (glibc 2.41) i openSUSE Leap 16.1 (glibc 2.40) **nie są wspierane** — nie spełniają wymagania ABI `GLIBC_2.43`. Pełna polityka: `LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.md`.

---

## 2. Kanoniczny Układ Archiwum
Archiwum zawiera dokładnie jeden katalog główny `SPLogbook/`:

```text
SPLogbook/
├── SPLogbook                           # Główny plik wykonywalny (0755)
├── splogbook-updater                   # Narzędzie aktualizatora (0755)
├── lib/                                # Hermetyzowane biblioteki dynamiczne (0755)
│   └── libhamlib.so.4                  # Biblioteka Hamlib z RUNPATH=$ORIGIN/lib (0755)
├── assets/                             # Zasoby graficzne i szablony (0755)
│   ├── splogbook.desktop               # Opcjonalny szablon skrótu (0644)
│   └── icon.png                        # Ikona programu (0644)
├── locales/                            # Tłumaczenia interfejsu (0755)
├── README.txt                          # Instrukcja obsługi (0644)
├── LICENSE                             # Licencja GPL-3.0-or-later (0644)
├── THIRD_PARTY_LICENSES/               # Licencje bibliotek zewnętrznych (0755/0644)
└── .splogbook-linux-portable-marker    # Strukturalny marker instalacji (0644)
```

---

## 3. Uprawnienia POSIX i Zasady Integralności
- Pliki wykonywalne i katalogi: `0755` (`rwxr-xr-x`)
- Pliki zwykłe, zasoby i marker: `0644` (`rw-r--r--`)
- Dowiązania symboliczne (`symlinks`): **Kategorycznie zabronione**
- Dowiązania twarde (`hard links`): **Kategorycznie zabronione**
- Pliki specjalne (`block`, `char`, `FIFO`, `socket`): **Kategorycznie zabronione**
- Bity specjalne (`setuid`, `setgid`, `sticky`): **Kategorycznie zabronione**
- Ścieżki bezwzględne i sekwencje ucieczki (`..`): **Odrzucane przed zapisem**

---

## 4. Ochrona Danych Użytkownika i XDG Base Directory
Aplikacja w wersji portable nie przechowuje danych użytkownika obok binariów:
- Konfiguracja: `$XDG_CONFIG_HOME/splogbook` (fallback: `$HOME/.config/splogbook`)
- Bazy danych łączności: `$XDG_DATA_HOME/splogbook` (fallback: `$HOME/.local/share/splogbook`)
- Dzienniki i stan: `$XDG_STATE_HOME/splogbook` (fallback: `$HOME/.local/state/splogbook`)
- Cache: `$XDG_CACHE_HOME/splogbook` (fallback: `$HOME/.cache/splogbook`)

Podczas aktualizacji, deinstalacji lub rollbacku katalogu programu żadne dane użytkownika nie ulegają modyfikacji ani usunięciu.
