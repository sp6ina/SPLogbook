// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Natywny program pomocniczy aktualizatora Windows: `splogbook-updater.exe` (M3-B).
//!
//! Zgodnie z decyzjami architektonicznymi i zasadami bezpieczeństwa projektu:
//! - Helper jest wywoływany wyłącznie z parametrem `--request <LOCAL_PATH>`;
//! - Wszelkie próby przekazania adresu URL, pobierania z sieci lub eskalacji uprawnień
//!   są bezwzględnie blokowane i odrzucane z kodem 10 (InvalidArguments);
//! - Helper współpracuje wyłącznie ze zweryfikowanym, wersjonowanym kontraktem `WindowsHelperRequest`;
//! - Wszystkie operacje (backup, apply, health, rollback, cleanup) wykonywane są przez
//!   biblioteczny silnik `WindowsHelperEngine` i kończą się typowanym kodem wyjścia.

use std::path::PathBuf;
use std::process::ExitCode;

use splogbook::cloud::updater::helper_protocol::{WindowsHelperExitCode, WindowsHelperRequest};
use splogbook::cloud::updater::windows_helper::{
    SystemCandidateLauncher, SystemProcessMonitor, WindowsHelperEngine,
};

/// Tryb działania pomocnika wywnioskowany z argumentów CLI.
#[derive(Debug, PartialEq, Eq)]
enum HelperCliMode {
    /// Pełne wykonanie procedury aktualizacji na podstawie pliku żądania.
    Execute { request_path: PathBuf },
    /// Wyłącznie walidacja poprawności i integralności pliku żądania.
    ValidateRequestOnly { request_path: PathBuf },
    /// Wypisanie wersji diagnostycznej programu pomocniczego.
    DiagnosticVersion,
}

/// Bezpieczny, minimalistyczny parser argumentów CLI pomocnika Windows.
///
/// Odrzuca wszelkie nieznane flagi, parametry sieciowe, flagi omijania zabezpieczeń
/// oraz wielokrotnie przekazane ścieżki żądania.
fn parse_cli_args<I>(args: I) -> Result<HelperCliMode, WindowsHelperExitCode>
where
    I: Iterator<Item = String>,
{
    let mut request_path: Option<PathBuf> = None;
    let mut validate_only = false;
    let mut diagnostic_version = false;

    // Pomijamy nazwę programu (args[0])
    let mut iter = args.skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--diagnostic-version" => {
                diagnostic_version = true;
            }
            "--validate-request-only" => {
                validate_only = true;
            }
            "--request" => {
                if request_path.is_some() {
                    eprintln!("Błąd: Wielokrotne przekazanie flagi --request jest zabronione");
                    return Err(WindowsHelperExitCode::InvalidArguments);
                }
                let val = match iter.next() {
                    Some(v) if !v.trim().is_empty() => v,
                    _ => {
                        eprintln!("Błąd: Flaga --request wymaga podania lokalnej ścieżki pliku");
                        return Err(WindowsHelperExitCode::InvalidArguments);
                    }
                };

                let path = PathBuf::from(val);
                request_path = Some(path);
            }
            // Bezwzględny zakaz flag pobierania, url, omijania podpisów, eskalacji uprawnień
            "--url" | "--download" | "--manifest-url" | "--skip-signature" | "--skip-hash"
            | "--force" | "--ignore-lock" | "--ignore-health" | "--no-rollback" | "--elevate"
            | "--runas" | "--unsafe" => {
                eprintln!("Błąd krytyczny: Odrzucono zabronioną flagę bezpieczeństwa: '{arg}'");
                return Err(WindowsHelperExitCode::InvalidArguments);
            }
            other => {
                eprintln!("Błąd: Nierozpoznany argument: '{other}'");
                return Err(WindowsHelperExitCode::InvalidArguments);
            }
        }
    }

    if diagnostic_version {
        return Ok(HelperCliMode::DiagnosticVersion);
    }

    let req_path = match request_path {
        Some(p) => p,
        None => {
            eprintln!("Błąd: Wymagane jest wskazanie pliku żądania przez flagę --request <PATH>");
            return Err(WindowsHelperExitCode::InvalidArguments);
        }
    };

    if validate_only {
        Ok(HelperCliMode::ValidateRequestOnly {
            request_path: req_path,
        })
    } else {
        Ok(HelperCliMode::Execute {
            request_path: req_path,
        })
    }
}

fn main() -> ExitCode {
    let mode = match parse_cli_args(std::env::args()) {
        Ok(m) => m,
        Err(code) => return ExitCode::from(code.as_i32() as u8),
    };

    match mode {
        HelperCliMode::DiagnosticVersion => {
            println!("splogbook-updater 1.2.0 (Windows Helper M3-B)");
            ExitCode::SUCCESS
        }
        HelperCliMode::ValidateRequestOnly { request_path } => {
            match WindowsHelperRequest::load_from_path(&request_path) {
                Ok(req) => {
                    println!(
                        "Żądanie pomocnika jest poprawne: attempt_id={}",
                        req.attempt_id
                    );
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("Błąd walidacji żądania pomocnika: {err}");
                    ExitCode::from(WindowsHelperExitCode::InvalidArguments.as_i32() as u8)
                }
            }
        }
        HelperCliMode::Execute { request_path } => {
            let request = match WindowsHelperRequest::load_from_path(&request_path) {
                Ok(req) => req,
                Err(err) => {
                    eprintln!(
                        "Nie udało się załadować żądania pomocnika z {}: {err}",
                        request_path.display()
                    );
                    return ExitCode::from(WindowsHelperExitCode::InvalidArguments.as_i32() as u8);
                }
            };

            let monitor = SystemProcessMonitor;
            let launcher = SystemCandidateLauncher;
            let mut engine = WindowsHelperEngine::new(&request, &monitor, &launcher);

            let (exit_code, _result) = engine.execute();
            ExitCode::from(exit_code.as_i32() as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parser_accepts_valid_request_arg() {
        let args = vec![
            "splogbook-updater.exe".to_string(),
            "--request".to_string(),
            "C:\\test\\request.json".to_string(),
        ];
        let mode = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(
            mode,
            HelperCliMode::Execute {
                request_path: PathBuf::from("C:\\test\\request.json")
            }
        );
    }

    #[test]
    fn cli_parser_accepts_validate_request_only() {
        let args = vec![
            "splogbook-updater.exe".to_string(),
            "--validate-request-only".to_string(),
            "--request".to_string(),
            "C:\\test\\request.json".to_string(),
        ];
        let mode = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(
            mode,
            HelperCliMode::ValidateRequestOnly {
                request_path: PathBuf::from("C:\\test\\request.json")
            }
        );
    }

    #[test]
    fn cli_parser_accepts_diagnostic_version() {
        let args = vec![
            "splogbook-updater.exe".to_string(),
            "--diagnostic-version".to_string(),
        ];
        let mode = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(mode, HelperCliMode::DiagnosticVersion);
    }

    #[test]
    fn cli_parser_rejects_forbidden_flags() {
        let forbidden = [
            "--url",
            "--download",
            "--manifest-url",
            "--skip-signature",
            "--skip-hash",
            "--force",
            "--ignore-lock",
            "--ignore-health",
            "--no-rollback",
            "--elevate",
            "--runas",
            "--unsafe",
        ];

        for flag in forbidden {
            let args = vec![
                "splogbook-updater.exe".to_string(),
                flag.to_string(),
                "--request".to_string(),
                "C:\\test\\request.json".to_string(),
            ];
            let res = parse_cli_args(args.into_iter());
            assert_eq!(
                res,
                Err(WindowsHelperExitCode::InvalidArguments),
                "Flaga '{flag}' powinna zostać odrzucona"
            );
        }
    }

    #[test]
    fn cli_parser_rejects_missing_request_file() {
        let args = vec!["splogbook-updater.exe".to_string(), "--request".to_string()];
        assert_eq!(
            parse_cli_args(args.into_iter()),
            Err(WindowsHelperExitCode::InvalidArguments)
        );

        let args_empty = vec!["splogbook-updater.exe".to_string()];
        assert_eq!(
            parse_cli_args(args_empty.into_iter()),
            Err(WindowsHelperExitCode::InvalidArguments)
        );
    }
}
