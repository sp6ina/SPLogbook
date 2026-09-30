# DESIGN 2.2 (FINAL): mapowanie emisji cyfrowych (`MODE` / `SUBMODE`) w ADIF

**Dokument:** `DESIGN_2_2_FINAL.md`
**Dotyczy:** `PLAN.md`, Etap 2.2
**Zastępuje:** `DESIGN_2_2_ADIF_MODE_MAPPING.md` (wraz z poprawkami z review)
**Data:** 2026-09-29
**Status:** Specyfikacja finalna (normatywna). Nie zawiera kodu i nie opisuje implementacji.

Słowa **MUSI**, **NIE WOLNO**, **POWINIEN** i **MOŻE** mają w tym dokumencie znaczenie normatywne.

---

## 1. Cel i zakres

Dokument określa:

1. jedną kanoniczną reprezentację wewnętrzną (`mode`, `submode`) dla emisji **FT8, FT4, Q65, JT65, JT9, WSPR i MSK144**,
2. odwzorowanie tej reprezentacji na zapis ADIF/ADX przy eksporcie,
3. odwzorowanie zapisów ADIF (także niestandardowych) na reprezentację kanoniczną przy imporcie,
4. zasady obsługi LoTW (upload przez TQSL, raport `lotwreport.adi`, dopasowanie potwierdzeń).

Dokument nie ocenia obecnego stanu kodu SPLogbook i nie określa sposobu implementacji.

**Odpowiedź na pytanie z Etapu 2.2:** SPLogbook **nie** przechodzi z `MODE=FT8` (ani `WSPR`, `JT65`, `JT9`, `MSK144`) na `MODE=MFSK` + `SUBMODE=...`. Parę `MFSK` + `SUBMODE` stosuje się wyłącznie przy zapisie ADIF emisji **FT4** i **Q65**, bo ADIF definiuje je jako submode'y `MFSK`.

---

## 2. Normatywna podstawa: ADIF 3.1.7

### 2.1. Wersja standardu

- Obowiązującą wersją jest **ADIF 3.1.7** (status *Released*, 2026-03-22). Potwierdzają to strona `https://www.adif.org.uk/` oraz plik `adiflatestrelease.txt`, który zawiera wartość `317`.
- **ADIF 3.1.8 nie istnieje jako wydanie.** Wszystkie deklaracje zgodności w SPLogbook odnoszą się do ADIF 3.1.7.
- Odwołania dotyczą sekcji **III.B.10 Mode Enumeration** oraz **III.B.25 Submode Enumeration** specyfikacji 3.1.7.

### 2.2. Zasady ogólne ADIF istotne dla tej specyfikacji

| Zasada | Treść w ADIF 3.1.7 | Konsekwencja |
|---|---|---|
| Typ pola `MODE` | `Enumeration` (Mode Enumeration) | Wartość `MODE` spoza tabeli Mode jest niezgodna ze standardem. |
| Typ pola `SUBMODE` | `String`, z zaleceniem *„use enumeration values for interoperability”* | Wartość spoza Submode Enumeration nie łamie typu pola, ale nie zapewnia interoperacyjności. |
| Wielkość liter | Wartości enumeracji nie rozróżniają wielkości liter | Import porównuje wartości bez rozróżniania wielkości liter. |
| Upward Compatibility | Plik zgodny z wersją N jest zgodny z każdą wersją M > N | Wartości przyjęte w tej specyfikacji pozostaną ważne. |
| Deprecation (*import-only*) | Wartości import-only należy **przyjmować przy imporcie**, ale **nie wolno ich emitować przy eksporcie** | Import mapuje je na postać bieżącą (§ 6.3). |

### 2.3. Status 7 emisji w ADIF 3.1.7 (zweryfikowane)

| Emisja | Miejsce w ADIF | Nadrzędny `MODE` | Dozwolone `SUBMODE` (Submode Enumeration) | Wprowadzona w ADIF |
|---|---|---|---|---|
| FT8 | Mode Enumeration | `FT8` | brak | 3.0.6 (2017-08-13) |
| FT4 | Submode Enumeration | `MFSK` | wartość `FT4` jest sama submode'em | 3.1.0 (2019-05-21) |
| Q65 | Submode Enumeration | `MFSK` | wartość `Q65` jest sama submode'em | 3.1.2 (2021-04-17) |
| JT65 | Mode Enumeration | `JT65` | `JT65A`, `JT65B`, `JT65B2`, `JT65C`, `JT65C2` | przed 3.0.6 |
| JT9 | Mode Enumeration | `JT9` | `JT9-1`, `JT9-2`, `JT9-5`, `JT9-10`, `JT9-30`, `JT9A`–`JT9H`, `JT9E FAST`, `JT9F FAST`, `JT9G FAST`, `JT9H FAST` | przed 3.0.6 |
| WSPR | Mode Enumeration | `WSPR` | brak | przed 3.0.6 |
| MSK144 | Mode Enumeration | `MSK144` | brak | przed 3.0.6 |

Dla kontekstu: w 3.1.7 submode'ami `MFSK` są `FSQCALL`, `FST4`, `FST4W`, `FT2`, `FT4`, `JS8`, `JTMS`, `MFSK4`–`MFSK128L` oraz `Q65`. Część z nich (`MFSK*`, `FSQCALL`) istniała przed 3.1.0, więc przynależność do `MFSK` nie wynika z daty wprowadzenia emisji.

---

## 3. Zweryfikowane zachowanie systemów zewnętrznych

Poniżej wymieniono tylko fakty potwierdzone w źródłach pierwotnych. Nie zweryfikowano zachowania Club Log, QRZ Logbook, eQSL, Log4OM ani N1MM Logger+, dlatego ta specyfikacja nie opiera się na żadnych twierdzeniach o nich. Zakłada jedynie, że systemy te przyjmują zapis zgodny z ADIF 3.1.7 (§ 5). Import MUSI być tolerancyjny (§ 6), niezależnie od tego, co te systemy faktycznie zwracają.

### 3.1. WSJT-X (źródło: `logbook/logbook.cpp`)

- Zapis ADIF: dla `FT4`, `FST4` i `Q65` WSJT-X emituje `<MODE:4>MFSK <SUBMODE:n>...`. Dla pozostałych emisji (w tym `FT8`, `JT65`, `JT9`, `MSK144`) emituje `<MODE:n>` z nazwą emisji.
- Ramka UDP typu 5 (*QSO Logged*) przekazuje nazwę emisji jako pojedynczy napis (np. `FT8`, `FT4`, `Q65`).
- Ramka UDP typu 12 (*Logged ADIF*) przekazuje gotowy rekord ADIF, podlegający regułom importu z § 6.
- Tryb WSPR w WSJT-X nie loguje łączności, więc emisja `WSPR` trafia do logu tylko przez import ADIF lub wpis ręczny.

### 3.2. TQSL (źródło: `config.xml` z repozytorium TrustedQSL)

TQSL tłumaczy parę ADIF (`MODE`, `SUBMODE`) na własną nazwę trybu LoTW:

| Wejście ADIF | Tryb LoTW | Grupa LoTW |
|---|---|---|
| `MODE=FT8` | `FT8` | DATA |
| `MODE=MFSK`, `SUBMODE=FT4` | `FT4` | DATA |
| `MODE=MFSK`, `SUBMODE=Q65` | `Q65` | DATA |
| `MODE=JT65` (z `SUBMODE` `JT65A`…`JT65C2` lub bez) | `JT65` | nie zweryfikowano w tej analizie |
| `MODE=JT9` (z dowolnym submode'em JT9 lub bez) | `JT9` | nie zweryfikowano w tej analizie |
| `MODE=WSPR` | `WSPR` | DATA |
| `MODE=MSK144` | `MSK144` | DATA |
| `MODE=MFSK` z nieznanym lub brakującym `SUBMODE` | `DATA` | DATA |

Wnioski:

- `config.xml` **nie zawiera** mapowania dla `MODE=FT4` ani `MODE=Q65`. Zapis `<MODE:3>FT4` nie ma więc w TQSL zdefiniowanej interpretacji.
- `MODE=MFSK` z nieuznawanym submode'em (np. `FT8`) trafia do trybu **`DATA`**, a nie `MFSK` ani `FT8`.

### 3.3. LoTW: dopasowanie i raport (źródło: LoTW Help *Key Concepts* oraz dokumentacja deweloperska raportu)

- Dwie łączności są dopasowywane, gdy tryby są **identyczne** (*exact mode match*) **albo należą do tej samej grupy trybów** (CW / PHONE / DATA).
- Część dyplomów WAS wymaga dokładnego dopasowania trybu. Informuje o tym pole `APP_LoTW_2xQSL=Y`.
- Pola raportu związane z trybem:
  - `MODE`: tryb łączności. LoTW zapowiada, że pole może zostać **pominięte**, gdy trybu nie da się jednoznacznie zapisać wartością zgodną z ADIF.
  - `APP_LoTW_MODE`: pojawia się, gdy tryb nie ma jednoznacznej reprezentacji ADIF.
  - `APP_LoTW_MODEGROUP`: `CW`, `PHONE` albo `DATA`.
  - `APP_LoTW_2xQSL`: `Y` lub `N`.
  - `APP_LoTW_QSLMODE`: tryb podany przez korespondenta (tylko gdy `APP_LoTW_2xQSL=N`).
- Dokumentacja raportu **nie wymienia pola `SUBMODE`**. Nie wiadomo, czy raport dla FT4/Q65 zawiera `MODE=FT4`, parę `MFSK/FT4`, czy `APP_LoTW_MODE=FT4`. Import MUSI obsłużyć wszystkie trzy postacie (§ 6, § 7).

---

## 4. Kanoniczna reprezentacja wewnętrzna (jedyna obowiązująca)

### 4.1. Definicja

Każda łączność przechowuje emisję jako parę:

- **`mode`**: *emisja efektywna*, czyli nazwa, pod którą operator zna emisję. Jest jedynym kluczem emisji używanym przez wszystkie funkcje aplikacji: wyświetlanie, filtry, statystyki, dyplomy, wykrywanie duplikatów i dopasowywanie potwierdzeń.
- **`submode`**: wyłącznie **wariant danej emisji** zdefiniowany w ADIF Submode Enumeration jako submode tej emisji. W pozostałych przypadkach pole jest puste.

### 4.2. Tabela kanoniczna

| Emisja | `mode` (kanoniczny) | `submode` (kanoniczny) |
|---|---|---|
| FT8 | `FT8` | puste |
| FT4 | `FT4` | puste |
| Q65 | `Q65` | puste |
| JT65 | `JT65` | puste **albo** jeden z: `JT65A`, `JT65B`, `JT65B2`, `JT65C`, `JT65C2` |
| JT9 | `JT9` | puste **albo** jeden z: `JT9-1`, `JT9-2`, `JT9-5`, `JT9-10`, `JT9-30`, `JT9A`, `JT9B`, `JT9C`, `JT9D`, `JT9E`, `JT9E FAST`, `JT9F`, `JT9F FAST`, `JT9G`, `JT9G FAST`, `JT9H`, `JT9H FAST` |
| WSPR | `WSPR` | puste |
| MSK144 | `MSK144` | puste |

### 4.3. Niezmienniki

1. **I1:** Dla tych 7 emisji `mode` NIE MOŻE przyjąć wartości `MFSK`. Wartość `MFSK` jest wyłącznie formą zapisu ADIF (§ 5).
2. **I2:** Dla `FT8`, `FT4`, `Q65`, `WSPR` i `MSK144` pole `submode` MUSI być puste. W szczególności NIE WOLNO zapisywać `submode=FT4` przy `mode=FT4`.
3. **I3:** Dla `JT65` i `JT9` pole `submode` MOŻE zawierać wyłącznie wartość z listy w § 4.2 dla tej samej emisji.
4. **I4:** Wartości `mode` i `submode` są przechowywane wielkimi literami, w dokładnej pisowni ADIF (np. `JT9E FAST` ze spacją).
5. **I5:** Emisja efektywna (`mode`) jest jedynym kluczem emisji. Obecność lub brak `submode` NIE MOŻE wpływać na filtrowanie po emisji, statystyki, dyplomy, duplikaty ani dopasowanie potwierdzeń. Przykładowo `JT65` z `submode=JT65A` jest dla tych funkcji emisją `JT65`.
6. **I6:** Te same dane wejściowe dają tę samą parę kanoniczną niezależnie od źródła: formularz GUI, UDP WSJT-X (typ 5 i 12), import ADIF/ADX, raport LoTW, skrzynka eQSL.

### 4.4. Wejścia inne niż ADIF

| Źródło | Wartość wejściowa | Reprezentacja kanoniczna |
|---|---|---|
| GUI (lista emisji) | `FT8` / `FT4` / `Q65` / `JT65` / `JT9` / `WSPR` / `MSK144` | `mode` = ta sama nazwa, `submode` puste (dla JT65/JT9 MOŻE zostać podany wariant z § 4.2) |
| WSJT-X UDP typ 5 | pole `Mode`, np. `FT8`, `FT4`, `Q65`, `JT65`, `JT9`, `MSK144` | `mode` = ta sama nazwa (wielkimi literami), `submode` puste |
| WSJT-X UDP typ 12 | rekord ADIF | reguły importu z § 6 |

---

## 5. Eksport: reprezentacja kanoniczna → ADIF/ADX

Reguły obowiązują jednakowo dla ADIF (`.adi`) i ADX (`.adx`) oraz dla wszystkich eksportów do usług zewnętrznych (LoTW/TQSL, eQSL, QRZ, Club Log).

| `mode` | `submode` | Emitowane `MODE` | Emitowane `SUBMODE` |
|---|---|---|---|
| `FT8` | puste | `FT8` | *(nie emitować)* |
| `FT4` | puste | `MFSK` | `FT4` |
| `Q65` | puste | `MFSK` | `Q65` |
| `JT65` | puste | `JT65` | *(nie emitować)* |
| `JT65` | `JT65A`…`JT65C2` | `JT65` | wartość `submode` |
| `JT9` | puste | `JT9` | *(nie emitować)* |
| `JT9` | wariant z § 4.2 | `JT9` | wartość `submode` |
| `WSPR` | puste | `WSPR` | *(nie emitować)* |
| `MSK144` | puste | `MSK144` | *(nie emitować)* |

Zakazy eksportu:

- **E1:** NIE WOLNO emitować `MODE=FT4` ani `MODE=Q65`, bo nie należą do Mode Enumeration i TQSL nie ma dla nich mapowania.
- **E2:** NIE WOLNO emitować `MODE=MFSK` z `SUBMODE` o wartości `FT8`, `JT65`, `JT9`, `WSPR` ani `MSK144`. W LoTW taki zapis zostałby zaklasyfikowany jako tryb `DATA`.
- **E3:** NIE WOLNO emitować wartości import-only (np. `MODE=JT65A`).
- **E4:** Eksport nie ma trybu alternatywnego ani przełącznika w ustawieniach. Obowiązuje wyłącznie zapis zgodny z ADIF 3.1.7.

---

## 6. Import: ADIF/ADX → reprezentacja kanoniczna

Porównania nie rozróżniają wielkości liter, a skrajne spacje są usuwane. Reguły stosuje się w podanej kolejności. Obowiązuje pierwsza pasująca.

### 6.1. Zapisy zgodne z ADIF 3.1.7

| Wejście `MODE` | Wejście `SUBMODE` | `mode` | `submode` |
|---|---|---|---|
| `FT8` | brak | `FT8` | puste |
| `MFSK` | `FT4` | `FT4` | puste |
| `MFSK` | `Q65` | `Q65` | puste |
| `JT65` | brak | `JT65` | puste |
| `JT65` | `JT65A`/`JT65B`/`JT65B2`/`JT65C`/`JT65C2` | `JT65` | wartość wejściowa |
| `JT9` | brak | `JT9` | puste |
| `JT9` | wariant z § 4.2 | `JT9` | wartość wejściowa |
| `WSPR` | brak | `WSPR` | puste |
| `MSK144` | brak | `MSK144` | puste |

### 6.2. Zapisy niestandardowe spotykane w praktyce (import tolerancyjny)

| Wejście | `mode` | `submode` | Uwagi |
|---|---|---|---|
| `MODE=FT4` (bez `MFSK`) | `FT4` | puste | Niezgodne z ADIF, ale przyjmowane. |
| `MODE=Q65` (bez `MFSK`) | `Q65` | puste | Jw. |
| `MODE=MFSK`, `SUBMODE=FT8` | `FT8` | puste | Niezgodne z Submode Enumeration, ale przyjmowane. |
| `MODE=MFSK`, `SUBMODE` ∈ {`JT65`, `JT9`, `WSPR`, `MSK144`} | wartość `SUBMODE` | puste | Jw. |
| `MODE` ∈ {`FT8`, `WSPR`, `MSK144`} z niepustym `SUBMODE` | wartość `MODE` | puste | `SUBMODE` odrzucany (brak wariantów w ADIF). |
| `MODE=FT4` lub `MODE=Q65` z `SUBMODE` równym tej samej nazwie | wartość `MODE` | puste | Jw. |
| `MODE=JT65`/`JT9` z `SUBMODE` spoza listy § 4.2 | wartość `MODE` | puste | Nieznany wariant odrzucany. |
| Brak `MODE`, obecne `APP_LoTW_MODE` z wartością `FT8`/`FT4`/`Q65`/`JT65`/`JT9`/`WSPR`/`MSK144` | wartość `APP_LoTW_MODE` | puste | Dotyczy raportu LoTW (§ 3.3). |

Każde zastosowanie reguły z § 6.2 POWINNO zostać odnotowane w raporcie importu jako ostrzeżenie o niestandardowym zapisie. Nie jest to błąd i rekord zostaje zaimportowany.

### 6.3. Wartości import-only (ADIF § II.B)

- Jeśli `MODE` jest wartością import-only, która w ADIF 3.1.7 jest submode'em jednej z 7 emisji (np. `JT65A` → submode `JT65`), to `mode` = emisja nadrzędna, a `submode` = wartość wejściowa. Przykład: `MODE=JT65A` → `mode=JT65`, `submode=JT65A`.
- Wartości import-only spoza 7 emisji nie są objęte tą specyfikacją (§ 9).

### 6.4. Pozostałe zapisy

- Pary `MODE`/`SUBMODE` nieobjęte § 6.1–6.3 MUSZĄ zostać **zachowane bez zmian** (wartości wejściowe zapisane wielkimi literami).
- NIE WOLNO zastępować nieznanych wartości wartością zastępczą (np. `OTHER`), której nie ma w Mode Enumeration. Taka zamiana jest stratna i daje zapis niezgodny z ADIF przy ponownym eksporcie.
- Rekord bez `MODE` i bez `APP_LoTW_MODE` jest importowany z pustą emisją i zgłaszany w raporcie importu jako błąd danych.

---

## 7. LoTW: zasady szczegółowe

### 7.1. Upload (TQSL)

- Plik przekazywany do TQSL jest generowany według § 5.
- Dzięki temu TQSL przypisuje dokładny tryb LoTW: `FT8`, `FT4`, `Q65`, `JT65`, `JT9`, `WSPR` lub `MSK144` (§ 3.2), co zachowuje kwalifikację do dyplomów wymagających dokładnego dopasowania trybu.

### 7.2. Raport potwierdzeń (`lotwreport.adi`)

- Każdy rekord raportu MUSI przejść przez te same reguły importu (§ 6), zanim zostanie użyty do dopasowania.
- W zakresie emisji raport nie jest traktowany inaczej niż zwykły import. Obsługiwane postacie dla FT4/Q65 to `MODE=FT4`/`Q65`, `MODE=MFSK` + `SUBMODE=FT4`/`Q65` oraz `APP_LoTW_MODE=FT4`/`Q65`. Każda z nich daje odpowiednio `mode=FT4` / `mode=Q65`.

### 7.3. Dopasowanie potwierdzenia do łączności lokalnej

1. Kluczem emisji jest kanoniczny `mode` rekordu z raportu, porównywany z kanonicznym `mode` łączności lokalnej (niezmiennik I5). `submode` nie bierze udziału w dopasowaniu.
2. Rekord raportu opisuje łączność wysłaną przez użytkownika, więc przy spójnym eksporcie (§ 5) i imporcie (§ 6) jego `mode` odpowiada lokalnemu `mode`.
3. `APP_LoTW_QSLMODE` (tryb korespondenta przy `APP_LoTW_2xQSL=N`) NIE MOŻE być używany jako klucz dopasowania. MOŻE być przechowywany wyłącznie informacyjnie.
4. `APP_LoTW_2xQSL` POWINIEN być zachowany, bo określa, czy potwierdzenie zalicza się do dyplomów wymagających dokładnego dopasowania trybu. Kwalifikacja dyplomowa do grupy DATA nie zależy od tego pola.

---

## 8. Gwarancje i kryteria akceptacji

### 8.1. Round-trip

Dla każdej pary kanonicznej z § 4.2: eksport (§ 5), a następnie import (§ 6), MUSI dać **identyczną** parę (`mode`, `submode`). Przykłady:

| Kanoniczne | Po eksporcie ADIF | Po ponownym imporcie |
|---|---|---|
| `FT8` / puste | `<MODE:3>FT8` | `FT8` / puste |
| `FT4` / puste | `<MODE:4>MFSK <SUBMODE:3>FT4` | `FT4` / puste |
| `Q65` / puste | `<MODE:4>MFSK <SUBMODE:3>Q65` | `Q65` / puste |
| `JT65` / `JT65B` | `<MODE:4>JT65 <SUBMODE:5>JT65B` | `JT65` / `JT65B` |
| `JT9` / `JT9E FAST` | `<MODE:3>JT9 <SUBMODE:9>JT9E FAST` | `JT9` / `JT9E FAST` |
| `WSPR` / puste | `<MODE:4>WSPR` | `WSPR` / puste |
| `MSK144` / puste | `<MODE:6>MSK144` | `MSK144` / puste |

### 8.2. Spójność źródeł (niezmiennik I6)

Ta sama łączność FT4 zalogowana przez UDP typ 5, przez UDP typ 12, przez import `wsjtx_log.adi` oraz odczytana z raportu LoTW MUSI dać `mode=FT4` z pustym `submode`. Import pliku zawierającego łączność już zalogowaną przez UDP MUSI zostać rozpoznany jako duplikat.

### 8.3. Zgodność eksportu

Żaden plik wygenerowany przez SPLogbook dla 7 emisji nie może zawierać wartości `MODE` spoza ADIF 3.1.7 Mode Enumeration, wartości import-only ani pary zakazanej przez E2.

### 8.4. Dane istniejące

Rekordy zapisane przed wdrożeniem, które nie spełniają § 4 (np. `mode=MFSK` z `submode=FT4`, `submode=FT4` przy `mode=FT4` albo wartości zapisane małymi literami), MUSZĄ zostać sprowadzone do postaci kanonicznej według reguł § 6. Sposób i moment migracji ustala plan implementacji. Ta specyfikacja wymaga jedynie, by po wdrożeniu w bazie nie było rekordów naruszających niezmienniki I1–I4.

---

## 9. Poza zakresem

- Pozostałe submode'y `MFSK` (`FST4`, `FST4W`, `FT2`, `JS8` i inne) oraz emisje spoza listy 7. Reguła z § 4–6 („emisja efektywna w `mode`, forma `MFSK` + `SUBMODE` tylko w ADIF”) jest dla nich naturalnym rozszerzeniem, ale wymaga osobnej decyzji.
- Warianty Q65 określane w WSJT-X literą i okresem (np. „Q65-60A”) nie są submode'ami w ADIF 3.1.7 i NIE MOGĄ być zapisywane w `submode`.
- Szczegóły zachowania Club Log, QRZ Logbook, eQSL, Log4OM i N1MM Logger+ (niezweryfikowane, § 3).
- Emisje fonii (np. `SSB` z `USB`/`LSB`) wymagają osobnej, symetrycznej reguły importu i eksportu, poza tym dokumentem.

---

## 10. Założenia usunięte względem `DESIGN_2_2_ADIF_MODE_MAPPING.md`

| # | Usunięte założenie | Stan faktyczny |
|---|---|---|
| 1 | Istnieje ADIF 3.1.8, a SPLogbook ma deklarować zgodność z „3.1.7 / 3.1.8” | Najnowsze wydanie to 3.1.7 (2026-03-22). Wersja 3.1.8 nie istnieje. |
| 2 | Od ADIF 3.1.0 „zamrożono” tabelę Mode, żeby nie obciążać LoTW | Po 3.1.0 dodano nowe emisje główne (`DYNAMIC`, `MTONE`, `OFDM`). Uzasadnienie nie ma źródła. |
| 3 | JS8, FST4, FST4W i Q65 dodano „później” niż FT4, a submode'y `MFSK` to „wyłącznie emisje od 3.1.0” | JS8 wszedł razem z FT4 w 3.1.0, FST4 w 3.1.1, FST4W i Q65 w 3.1.2. `MFSK4`–`MFSK128` i `FSQCALL` są starsze niż 3.1.0. |
| 4 | `MFSK` + `SUBMODE=FT8` jest „formalnie niezgodne” i odrzucane przez walidatory | `SUBMODE` ma typ String, a spec jedynie zaleca wartości z enumeracji. Zapis jest niezalecany i nie działa między programami (E2), ale nie łamie typu pola. Ścisłą niezgodnością jest natomiast `MODE=FT4`/`Q65`. |
| 5 | TQSL degraduje `MFSK/FT8` do trybu `MFSK` | TQSL mapuje taką parę na tryb `DATA`. Dopasowanie w grupie DATA zadziała, ale utracona zostanie dokładna zgodność trybu (np. WAS FT8). |
| 6 | TQSL traktuje FT4/Q65 jako „submode'y MFSK” i ma wpisy tolerancyjne dla `MODE=FT4` | TQSL mapuje `MFSK/FT4` i `MFSK/Q65` na osobne tryby LoTW `FT4` i `Q65`. Wpisów dla `MODE=FT4`/`MODE=Q65` nie ma. |
| 7 | `lotwreport.adi` zwraca `<MODE:4>MFSK <SUBMODE:3>FT4` | Dokumentacja raportu nie wymienia `SUBMODE`. Opisuje `MODE`, które może zostać pominięte, oraz `APP_LoTW_MODE`. Import musi obsłużyć każdą postać (§ 7.2). |
| 8 | Szczegółowe zachowania Club Log, QRZ, eQSL, Log4OM i N1MM (OQRS, `RESULT=FAIL`, `modelist.csv` itd.) | Niezweryfikowane i usunięte. Specyfikacja opiera się wyłącznie na zgodności z ADIF i tolerancyjnym imporcie. |
| 9 | WSJT-X UDP typ 5 przekazuje emisję `WSPR` | Tryb WSPR nie loguje łączności. Pominięto też ramkę typu 12 (rekord ADIF), którą ta specyfikacja obejmuje. |
| 10 | Wariant D dopuszczał kilka postaci wewnętrznych (`FT4`/puste, `FT4`/`FT4`, `MFSK`/`FT4`) | Obowiązuje jedna postać kanoniczna (§ 4). |
| 11 | Wartość zastępcza `OTHER` dla nierozpoznanych par | Zakazana. Nieznane pary są zachowywane bez zmian (§ 6.4). |
| 12 | Tryb eksportu „Legacy” emitujący `MODE=FT4` (Wariant C2) | Odrzucony. Tylko zapis zgodny z ADIF 3.1.7 (E4). |
| 13 | Niepodparte liczby („FT8 > 70% ruchu HF”, szacunki godzin pracy) | Usunięte. |

---

## 11. Decyzja

1. `FT8`, `JT65`, `JT9`, `WSPR` i `MSK144` pozostają w ADIF głównymi wartościami `MODE`. Migracja na `MFSK` jest odrzucona.
2. `FT4` i `Q65` są w ADIF zapisywane wyłącznie jako `MODE=MFSK` + `SUBMODE=FT4` / `Q65`.
3. Wewnętrznie wszystkie 7 emisji ma jedną postać kanoniczną: `mode` = nazwa emisji, `submode` puste albo wariant ADIF (tylko JT65/JT9). Tłumaczenie na `MFSK` + `SUBMODE` i z powrotem odbywa się wyłącznie na granicy importu i eksportu ADIF/ADX.
4. Import jest tolerancyjny (§ 6.2–6.3) i nie jest stratny (§ 6.4). Eksport jest ściśle zgodny z ADIF 3.1.7 (§ 5).
5. Potwierdzenia LoTW są dopasowywane po kanonicznym `mode` po normalizacji raportu (§ 7).

---

## Źródła

- ADIF 3.1.7 (Released, 2026-03-22): `https://www.adif.org.uk/317/ADIF_317.htm`, sekcje II.A–II.C, III.B.10, III.B.25. Wersje archiwalne 3.0.6, 3.1.0, 3.1.1 i 3.1.2 posłużyły do ustalenia dat wprowadzenia emisji.
- `https://www.adif.org.uk/adiflatestrelease.txt` (wartość `317`).
- TrustedQSL `src/config.xml` (SourceForge, gałąź `master`): elementy `adifmode` i `mode group`.
- ARRL LoTW Help: *Key Concepts* (reguły dopasowania) oraz dokumentacja deweloperska raportu (pola `MODE`, `APP_LoTW_*`).
- WSJT-X `logbook/logbook.cpp` (reguła emisji `MFSK` + `SUBMODE` dla FT4, FST4 i Q65).
