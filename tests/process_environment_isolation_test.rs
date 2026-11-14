// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Testy kontraktu wspólnego mechanizmu izolacji środowiska procesu.
//
// Bramka: STAGE-QA-XDG-ENVIRONMENT-DETERMINISM (2026-11-14).
//
// Każdy test używa własnych, unikalnych nazw zmiennych, więc nie zależy od kolejności
// wykonania binarki. Wszystkie mutacje przechodzą przez `EnvGuard`; żaden test nie
// wywołuje `std::env::set_var`/`remove_var` bezpośrednio ani nie używa `sleep`.

use std::ffi::OsStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Barrier;
use std::sync::mpsc::channel;

#[path = "support/process_env.rs"]
mod process_env;

use process_env::{EnvGuard, EnvGuardError};

const RESTORE_VAR: &str = "SPLOG_ENV_CONTRACT_RESTORE";
const MISSING_VAR: &str = "SPLOG_ENV_CONTRACT_MISSING";
const MULTI_VAR_A: &str = "SPLOG_ENV_CONTRACT_MULTI_A";
const MULTI_VAR_B: &str = "SPLOG_ENV_CONTRACT_MULTI_B";
const MULTI_VAR_C: &str = "SPLOG_ENV_CONTRACT_MULTI_C";
const EMPTY_VAR: &str = "SPLOG_ENV_CONTRACT_EMPTY";
const PANIC_VAR: &str = "SPLOG_ENV_CONTRACT_PANIC";
const POISON_VAR: &str = "SPLOG_ENV_CONTRACT_POISON";
const OWN_VALUE_VAR: &str = "SPLOG_ENV_CONTRACT_OWN_VALUE";
const CONCURRENT_VAR: &str = "SPLOG_ENV_CONTRACT_CONCURRENT";

/// Wartość zmiennej bez jej tworzenia (tylko odczyt).
fn snapshot(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name)
}

#[test]
fn existing_variable_is_restored() {
    // `PATH` istnieje w każdym procesie testowym; ten test nie uruchamia procesów
    // potomnych, więc chwilowa zmiana PATH nie wpływa na inne testy tej binarki.
    let original_path = snapshot("PATH").expect("PATH must exist in the test process");
    {
        let _env = EnvGuard::new(&[("PATH", Some(OsStr::new("SPLOG-ENV-CONTRACT-NOT-A-PATH")))]);
        assert_eq!(
            snapshot("PATH"),
            Some(std::ffi::OsString::from("SPLOG-ENV-CONTRACT-NOT-A-PATH"))
        );
    }
    assert_eq!(snapshot("PATH"), Some(original_path));
}

#[test]
fn missing_variable_stays_missing() {
    let _env = EnvGuard::new(&[(MISSING_VAR, None)]);
    assert_eq!(snapshot(MISSING_VAR), None);
    assert!(std::env::var(MISSING_VAR).is_err());
}

#[test]
fn multiple_variables_are_restored() {
    assert_eq!(snapshot(MULTI_VAR_A), None);
    assert_eq!(snapshot(MULTI_VAR_B), None);
    assert_eq!(snapshot(MULTI_VAR_C), None);
    {
        let _env = EnvGuard::new(&[
            (MULTI_VAR_A, Some(OsStr::new("alpha"))),
            (MULTI_VAR_B, Some(OsStr::new("beta"))),
            (MULTI_VAR_C, Some(OsStr::new("gamma"))),
        ]);
        assert_eq!(snapshot(MULTI_VAR_A).as_deref(), Some(OsStr::new("alpha")));
        assert_eq!(snapshot(MULTI_VAR_B).as_deref(), Some(OsStr::new("beta")));
        assert_eq!(snapshot(MULTI_VAR_C).as_deref(), Some(OsStr::new("gamma")));
    }
    assert_eq!(snapshot(MULTI_VAR_A), None);
    assert_eq!(snapshot(MULTI_VAR_B), None);
    assert_eq!(snapshot(MULTI_VAR_C), None);
}

#[test]
fn empty_value_is_distinct_from_missing() {
    assert_eq!(snapshot(EMPTY_VAR), None);
    let _env = EnvGuard::new(&[(EMPTY_VAR, Some(OsStr::new("")))]);
    assert_eq!(
        snapshot(EMPTY_VAR),
        Some(std::ffi::OsString::new()),
        "pusta wartość musi być odróżniona od braku zmiennej"
    );
    assert_eq!(std::env::var(EMPTY_VAR).as_deref(), Ok(""));
}

#[test]
fn guard_observes_requested_value_inside_section() {
    let _env = EnvGuard::new(&[(OWN_VALUE_VAR, Some(OsStr::new("widoczna-w-sekcji")))]);
    assert_eq!(
        std::env::var(OWN_VALUE_VAR).as_deref(),
        Ok("widoczna-w-sekcji")
    );
}

#[test]
fn environment_is_restored_after_panic() {
    assert_eq!(snapshot(PANIC_VAR), None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _env = EnvGuard::new(&[(PANIC_VAR, Some(OsStr::new("panika")))]);
        assert_eq!(snapshot(PANIC_VAR).as_deref(), Some(OsStr::new("panika")));
        panic!("zamierzona panika wewnątrz sekcji chronionej");
    }));
    assert!(
        result.is_err(),
        "panic musi zostać zgłoszony, nie połknięty"
    );
    assert_eq!(
        snapshot(PANIC_VAR),
        None,
        "Drop guarda musi przywrócić środowisko podczas unwinding"
    );
}

#[test]
fn lock_is_not_permanently_poisoned_after_panic() {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _env = EnvGuard::new(&[(POISON_VAR, Some(OsStr::new("zatrucie")))]);
        panic!("zamierzone zatrucie blokady");
    }));
    // Kolejna akwizycja musi się udać mimo PoisonError; `new` blokuje i odzyskuje guard.
    let recovered = EnvGuard::acquire();
    assert_eq!(snapshot(POISON_VAR), None);
    drop(recovered);
}

#[test]
fn nested_guard_is_explicitly_rejected() {
    let outer = EnvGuard::acquire();
    assert_eq!(
        EnvGuard::try_new(&[]).unwrap_err(),
        EnvGuardError::NestedGuard,
        "zagnieżdżenie w tym samym wątku musi być jawnie odrzucone, a nie zakleszczone"
    );
    drop(outer);
}

#[test]
fn concurrent_acquisition_reports_busy_and_then_succeeds() {
    let guard = EnvGuard::acquire();
    let (tx, rx) = channel();
    let probe = std::thread::spawn(move || {
        tx.send(EnvGuard::try_new(&[]).map(drop))
            .expect("probe channel");
    });
    let observed = rx.recv().expect("probe result");
    probe.join().expect("probe thread");
    assert_eq!(
        observed,
        Err(EnvGuardError::Busy),
        "próba z innego wątku musi zgłosić Busy, zamiast czekać"
    );
    drop(guard);
    // Po zwolnieniu blokady kolejny guard musi zostać przejęty (także po poison).
    drop(EnvGuard::acquire());
}

#[test]
fn concurrent_mutations_are_serialized() {
    const THREADS: usize = 8;
    let barrier = Barrier::new(THREADS);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(THREADS);
        for index in 0..THREADS {
            let barrier = &barrier;
            handles.push(scope.spawn(move || {
                let value = format!("thread-{index}");
                barrier.wait();
                let _env = EnvGuard::new(&[(CONCURRENT_VAR, Some(OsStr::new(&value)))]);
                assert_eq!(
                    snapshot(CONCURRENT_VAR).as_deref(),
                    Some(OsStr::new(&value)),
                    "wątek {index} odczytał wartość zapisaną przez inny wątek"
                );
            }));
        }
        for handle in handles {
            handle.join().expect("wątek mutujący nie może panikować");
        }
    });
    assert_eq!(snapshot(CONCURRENT_VAR), None);
}

#[test]
fn baseline_is_restored_after_drop() {
    let before = snapshot(RESTORE_VAR);
    {
        let _env = EnvGuard::new(&[(RESTORE_VAR, Some(OsStr::new("chwilowa")))]);
        assert_ne!(snapshot(RESTORE_VAR), before);
    }
    assert_eq!(snapshot(RESTORE_VAR), before);
}

#[test]
fn guard_does_not_change_current_directory() {
    let before = std::env::current_dir().expect("current_dir");
    {
        let _env = EnvGuard::new(&[(OWN_VALUE_VAR, Some(OsStr::new("cwd-nie-zmienia-sie")))]);
        assert_eq!(std::env::current_dir().expect("current_dir"), before);
    }
    assert_eq!(std::env::current_dir().expect("current_dir"), before);
}
