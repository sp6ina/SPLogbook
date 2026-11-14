// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Wspólny, testowy mechanizm izolacji środowiska procesu.
//
// Bramka: STAGE-QA-XDG-ENVIRONMENT-DETERMINISM (2026-11-14).
//
// Problem: testy tego samego procesu równolegle nadpisywały procesowe `HOME` i `XDG_*`,
// a `XdgPaths::resolve()` czyta je w kilku krokach. Efektem był wyścig: jeden test
// rozwiązywał ścieżki z wartości zapisanych przez drugi test (CI run 43571747469,
// attempt 1, `tests/linux_xdg_data_preservation_test.rs:47`).
//
// Kontrakt tego modułu:
// 1. Jedna, wspólna blokada dla **wszystkich** mutacji środowiska procesu w danej
//    binarce testowej; blokada jest per proces, bo środowisko jest per proces.
// 2. RAII: `EnvGuard` zapisuje pierwotne wartości (także „zmiennej nie było”), stosuje
//    żądane zmiany i przywraca stan w `Drop` — również podczas unwinding po panic.
// 3. `Drop` przywraca zmienne w odwrotnej kolejności i odróżnia brak zmiennej od
//    pustej wartości (`Option<OsString>`).
// 4. Zatruta blokada (po panic) nie powoduje kaskady: każda akwizycja odzyskuje guard
//    przez `PoisonError::into_inner`. Blokada chroni wyłącznie wzajemne wykluczenie,
//    nie chroni żadnych danych, a środowisko jest przywracane przed jej zwolnieniem.
// 5. Zagnieżdżona akwizycja w tym samym wątku jest jawnie odrzucana (`NestedGuard`),
//    więc nie może dojść do deadlocka; `try_new` zwraca też `Busy`, gdy blokadę
//    trzyma inny wątek.
//
// Moduł jest wyłącznie testowy: jest włączany przez `#[path = "support/process_env.rs"]`
// w binarkach testowych i nie istnieje w produkcyjnym buildzie.

#![allow(dead_code)]

use std::cell::Cell;
use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

/// Jedyny mutex chroniący mutacje środowiska procesu w danej binarce testowej.
static ENV_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    /// Czy bieżący wątek trzyma już `EnvGuard` (wykrywanie zagnieżdżenia).
    static GUARD_HELD: Cell<bool> = const { Cell::new(false) };
}

/// Błąd akwizycji [`EnvGuard`] przez [`EnvGuard::try_new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvGuardError {
    /// Bieżący wątek trzyma już guard; kolejna akwizycja zablokowałaby sam siebie.
    NestedGuard,
    /// Blokadę trzyma inny wątek; `try_new` nigdy nie czeka.
    Busy,
}

impl std::fmt::Display for EnvGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NestedGuard => write!(
                f,
                "zagnieżdżony EnvGuard w tym samym wątku jest zabroniony (deadlock)"
            ),
            Self::Busy => write!(f, "inny wątek trzyma blokadę środowiska"),
        }
    }
}

impl std::error::Error for EnvGuardError {}

impl std::fmt::Debug for EnvGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nie ujawniamy nazw ani wartości zmiennych środowiskowych.
        f.debug_struct("EnvGuard")
            .field("guarded_variables", &self.originals.len())
            .finish_non_exhaustive()
    }
}

/// RAII-guard mutacji środowiska procesu.
///
/// `EnvGuard::new(&[("HOME", Some(path.as_os_str())), ("XDG_DATA_HOME", None)])`
/// czeka na wspólną blokadę, zapisuje pierwotny stan wskazanych zmiennych i stosuje
/// żądane wartości. Zwalnia blokadę i przywraca środowisko w `Drop`.
pub struct EnvGuard {
    originals: Vec<(String, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
    _held: HeldFlag,
}

/// Czyści znacznik zagnieżdżenia po zwolnieniu guarda.
struct HeldFlag;

impl HeldFlag {
    fn enter() -> Self {
        GUARD_HELD.with(|held| {
            debug_assert!(
                !held.get(),
                "zagnieżdżony EnvGuard musi zostać odrzucony przed ustawieniem znacznika"
            );
            held.set(true);
        });
        Self
    }
}

impl Drop for HeldFlag {
    fn drop(&mut self) {
        GUARD_HELD.with(|held| held.set(false));
    }
}

fn current_thread_holds_guard() -> bool {
    GUARD_HELD.with(Cell::get)
}

/// Blokująca akwizycja z odzyskaniem zatrutej blokady.
fn lock_environment() -> MutexGuard<'static, ()> {
    // Zatrucie nie niesie informacji o stanie środowiska: guard przywraca je w `Drop`
    // przed zwolnieniem blokady, a mutex nie chroni żadnych danych. Odzyskanie jest
    // bezpieczne i zapobiega kaskadzie błędów po panic jednego testu.
    ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Nieblokująca akwizycja; `Busy` zamiast czekania.
fn try_lock_environment() -> Result<MutexGuard<'static, ()>, EnvGuardError> {
    match ENV_LOCK.try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
        Err(TryLockError::WouldBlock) => Err(EnvGuardError::Busy),
    }
}

/// Stosuje wartość zmiennej: `Some` ustawia, `None` usuwa.
///
/// # Safety
///
/// Wywołujący musi trzymać `ENV_LOCK` przez cały czas życia [`EnvGuard`]. Wszystkie
/// mutacje środowiska procesu w danej binarce testowej przechodzą przez ten moduł, więc
/// żaden inny wątek nie może równolegle modyfikować środowiska. Odczyty, które muszą być
/// spójne z zapisem, wykonują się w sekcji chronionej guardem. Blokada jest odzyskiwana
/// po poison, ponieważ nie chroni danych — tylko wzajemne wykluczenie.
unsafe fn apply(name: &str, value: Option<&OsStr>) {
    match value {
        // SAFETY: `ENV_LOCK` jest trzymany przez cały czas życia guarda; patrz wyżej.
        Some(value) => unsafe { env::set_var(name, value) },
        // SAFETY: `ENV_LOCK` jest trzymany przez cały czas życia guarda; patrz wyżej.
        None => unsafe { env::remove_var(name) },
    }
}

impl EnvGuard {
    /// Blokująco przejmuje wspólną blokadę i stosuje `vars`.
    ///
    /// # Panics
    ///
    /// Panikuje, gdy bieżący wątek trzyma już inny `EnvGuard`. Zagnieżdżenie jest
    /// jawnie zabronione, ponieważ prowadziłoby do deadlocka. Testy kontraktu używają
    /// [`EnvGuard::try_new`], aby sprawdzić to bez paniki.
    pub fn new(vars: &[(&str, Option<&OsStr>)]) -> Self {
        if current_thread_holds_guard() {
            panic!(
                "zagnieżdżony EnvGuard w tym samym wątku jest zabroniony: \
                 użyj EnvGuard::try_new, aby otrzymać EnvGuardError::NestedGuard"
            );
        }
        Self::install(lock_environment(), vars)
    }

    /// Przejmuje wspólną blokadę (blokująco), nie zmieniając żadnej zmiennej.
    pub fn acquire() -> Self {
        Self::new(&[])
    }

    /// Nieblokująca akwizycja; nigdy nie czeka i nigdy nie panikuje.
    pub fn try_new(vars: &[(&str, Option<&OsStr>)]) -> Result<Self, EnvGuardError> {
        if current_thread_holds_guard() {
            return Err(EnvGuardError::NestedGuard);
        }
        Ok(Self::install(try_lock_environment()?, vars))
    }

    fn install(lock: MutexGuard<'static, ()>, vars: &[(&str, Option<&OsStr>)]) -> Self {
        let held = HeldFlag::enter();
        let mut originals = Vec::with_capacity(vars.len());
        for (name, value) in vars {
            originals.push(((*name).to_owned(), env::var_os(name)));
            // SAFETY: `lock` jest trzymany przez cały czas życia tego guarda.
            unsafe { apply(name, *value) };
        }
        Self {
            originals,
            _lock: lock,
            _held: held,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Przywracanie w odwrotnej kolejności względem zapisu; odróżnia brak zmiennej
        // (`None`) od pustej wartości (`Some("")`).
        for (name, original) in self.originals.iter().rev() {
            // SAFETY: blokada jest nadal trzymana przez pole `_lock`, które jest
            // zwalniane dopiero po zakończeniu tego `Drop`.
            unsafe { apply(name, original.as_deref()) };
        }
    }
}
