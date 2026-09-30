# Raport z Audytu rows.flatten() (CQ-2.4.B)

Zgodnie z planem CODE_QUALITY_NORMALIZATION_PLAN.md (PROB-P2-02), dokonalismy audytu uzycia `rows.flatten()` w modulach bazy danych.
Stosowanie `flatten()` powoduje ciche ignorowanie bledow parsowania i rzutowania w SQLite.

## Polityki zatwierdzone do implementacji w zadaniach CQ-2.4.C1-C4:

1. **src/core/database.rs** (Zadanie CQ-2.4.C1)
   - Znalezione uzycia: `get_upload_queue`
   - **Polityka: Fail-fast**
   - Uzasadnienie: Nalezy propagowac blad w przypadku uszkodzonej kolejki.

2. **src/core/database_stats.rs** (Zadanie CQ-2.4.C2)
   - Znalezione uzycia: Statystyki QSO, emisji, dyplomow, pasm.
   - **Polityka: Fail-fast**
   - Uzasadnienie: Zle sformatowany lub uszkodzony wiersz statystyk wskazuje na glebszy problem w glownej bazie QSO. Ciche zignorowanie go przeklamie uzytkownikowi liczniki zawodow i logu.

3. **src/core/prefix.rs** (Zadanie CQ-2.4.C3)
   - Znalezione uzycia: Wczytywanie prefiksow i wyjatkow panstw z pliku service_db.
   - **Polityka: Partial result + warning (logowanie bledu)**
   - Uzasadnienie: Jesli pojedyncza regula prefiksu w zewnetrznej bazie jest uszkodzona, logbook powinien dzialac w oparciu o pozostale 99% regul. Aplikacja nie moze przestac dzialac z powodu literowki w zewnetrznym pliku referencyjnym.

4. **src/core/service_db.rs** (Zadanie CQ-2.4.C4)
   - Znalezione uzycia: Wczytywanie list IOTA, stanow, QSL managerow.
   - **Polityka: Partial result + warning (logowanie bledu)**
   - Uzasadnienie: Analogicznie jak wyzej. Zewnetrzne bazy klubow i stanow moga miec bledy typograficzne (niedozwolony NULL dla wymaganej kolumny). Pomijamy wadliwe rekordy z logowaniem do stderr, chroniac glowna aplikacje.

## Wniosek
Zatwierdzam polityki dla modulow. Zadanie CQ-2.4.B uwaza sie za ZAKONCZONE.
Przekazuje gotowosc do implementacji konkretnych kodow (CQ-2.4.C1-C4).