# DEEPSEEK HARNESS — BOOTSTRAP SESJI

**Dokument ID:** DSH-SPLOGBOOK-BOOT-001
**Data:** 2026-11-09
**Przeznaczenie:** minimalny, powtarzalny start nowej sesji agentowej w repozytorium SPLogbook.

---

## Procedura startowa (wykonaj dokładnie w tej kolejności)

1. Przeczytaj `AGENTS.md` (zasady obowiązkowe).
2. Przeczytaj `docs/PROJECT_CURRENT_STATE.md` (jednostronicowy stan prawdy).
3. Przeczytaj `docs/NEXT_WORK_GATE.md` — **dokładnie jedna** następna bramka.
4. Wykonaj fetch:

   ```
   git -c http.sslBackend=openssl -c core.fsmonitor=false -c core.untrackedCache=false fetch --prune origin
   ```

   (`schannel` w sandboxie DSH zwraca `SEC_E_NO_CREDENTIALS`; nie zmieniaj konfiguracji globalnej.)

5. Zweryfikuj stan:

   ```
   git rev-parse HEAD
   git rev-parse main
   git rev-parse origin/main
   git rev-list --left-right --count origin/main...main
   git -c core.fsmonitor=false -c core.untrackedCache=false status --porcelain=v1
   ```

   Wymagane: `HEAD == main == origin/main`, `0 0`, brak zmian śledzonych.
   Jeżeli `main` ruszył się od czasu tego dokumentu — **najpierw zaktualizuj stan**, nie zakładaj
   wartości z tego pliku.

6. Zweryfikuj tag `v1.2.0`:

   ```
   git cat-file -t refs/tags/v1.2.0                  → tag
   git rev-parse refs/tags/v1.2.0                    → 3ea8d51aab436805107a1cf422e0b35cc301458f
   git rev-parse "refs/tags/v1.2.0^{}"               → 6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592
   ```

7. Uruchom baseline z `docs/DEEPSEEK_HARNESS_HANDOFF.md` §5 (`-j 2`) i zaraportuj **dokładne** liczniki.
8. Pracuj **wyłącznie** nad bramką wskazaną w `docs/NEXT_WORK_GATE.md`.

## Czego nie robić

- Nie zaczynaj projektu od nowa i nie „porządkuj” historii.
- Nie wykonuj: rebase, zwykły merge, force-push, tag, release, HIL, PTT, RF.
- Nie otwieraj produkcyjnych baz ani konfiguracji; nie używaj produkcyjnego `HOME`/`APPDATA`.
- Nie zmieniaj `src/` produkcyjnie, `Cargo.toml`, `Cargo.lock` bez osobnej bramki.
- Nie ustawiaj `SPLOG_RUN_INNO_E2E` / `SPLOG_RUN_MSI_E2E` w standardowym baseline.
- Nie uznawaj testu modelowego/kontenerowego za dowód natywny.

## Szybki kontekst projektu (2026-11-09)

- Wersja publiczna: **1.2.0** (tag `v1.2.0` → `6f7d3aa`, obiekt `3ea8d51a`).
- Wersja rozwojowa: **1.3.0-alpha.1**.
- HEAD bazowy: **`f73a703`** (przed bramką QA); po bramce QA — wartość z `docs/PROJECT_CURRENT_STATE.md`.
- M3-G: **BLOCKED**.
- Następna bramka po QA: **`STAGE-M3-F-LINUX-SUPPORT-POLICY-ALIGNMENT-GATE`**.
