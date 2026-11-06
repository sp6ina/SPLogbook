// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Deterministyczny parser i audytor wyników testów obszaru roboczego Cargo.
//!
//! Zapewnia rygorystyczne rozdzielenie celów testowych (lib, binaries, integration, guards, doc tests)
//! oraz eliminuje artefakty notacji matematycznej (np. duplikaty KaTeX).

/// Kategoria celu testowego w repozytorium Cargo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceTestTargetKind {
    /// Główna biblioteka (`src/lib.rs`).
    LibInCrate,
    /// Pliki wykonywalne (`src/main.rs`, `src/bin/*.rs`).
    Binary,
    /// Testy integracyjne (`tests/cat_native_*.rs`, inne testy podsystemów).
    Integration,
    /// Strażnicy architektury i bezpieczeństwa (`tests/*_guard.rs`).
    Guard,
    /// Testy dokumentacyjne (`Doc-tests ...`).
    DocTest,
}

/// Podsumowanie wykonania pojedynczego celu testowego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTestSummary {
    pub target_name: String,
    pub kind: WorkspaceTestTargetKind,
    pub registered: usize,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

/// Zbiorczy raport zliczania testów w całym obszarze roboczym.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceTestReport {
    pub targets: Vec<TargetTestSummary>,
}

impl WorkspaceTestReport {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    /// Dodaje podsumowanie celu.
    pub fn push(&mut self, summary: TargetTestSummary) {
        self.targets.push(summary);
    }

    /// Liczba testów w bibliotece (`src/lib.rs`).
    #[must_use]
    pub fn lib_tests(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.kind == WorkspaceTestTargetKind::LibInCrate)
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Liczba testów w plikach binarnych (`src/main.rs`, `src/bin/*`).
    #[must_use]
    pub fn binary_tests(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.kind == WorkspaceTestTargetKind::Binary)
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Liczba testów integracyjnych (`tests/cat_native_*.rs`).
    #[must_use]
    pub fn integration_tests(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.kind == WorkspaceTestTargetKind::Integration)
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Liczba testów strażników (`tests/*_guard.rs`).
    #[must_use]
    pub fn guard_tests(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.kind == WorkspaceTestTargetKind::Guard)
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Liczba testów dokumentacyjnych (`Doc-tests`).
    #[must_use]
    pub fn doc_tests(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.kind == WorkspaceTestTargetKind::DocTest)
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Łączna liczba zarejestrowanych testów.
    #[must_use]
    pub fn total_registered(&self) -> usize {
        self.targets
            .iter()
            .map(|t| t.registered.max(t.passed + t.failed + t.ignored))
            .sum()
    }

    /// Łączna liczba wykonanych testów (`passed + failed`).
    #[must_use]
    pub fn total_executed(&self) -> usize {
        self.total_passed() + self.total_failed()
    }

    /// Łączna liczba zdanych testów.
    #[must_use]
    pub fn total_passed(&self) -> usize {
        self.targets.iter().map(|t| t.passed).sum()
    }

    /// Łączna liczba nieudanych testów.
    #[must_use]
    pub fn total_failed(&self) -> usize {
        self.targets.iter().map(|t| t.failed).sum()
    }

    /// Łączna liczba ignorowanych testów.
    #[must_use]
    pub fn total_ignored(&self) -> usize {
        self.targets.iter().map(|t| t.ignored).sum()
    }

    /// Liczba ignorowanych testów sprzętowych HIL (w suicie CAT 7).
    #[must_use]
    pub fn hil_ignored(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| t.target_name.contains("cat_native_7"))
            .map(|t| t.ignored)
            .sum()
    }
}

/// Klasyfikuje ścieżkę lub nazwę celu do odpowiedniej kategorii.
#[must_use]
pub fn classify_target(name: &str) -> WorkspaceTestTargetKind {
    let normalized = name.replace('\\', "/");
    if normalized.contains("Doc-tests") {
        WorkspaceTestTargetKind::DocTest
    } else if normalized.contains("src/lib.rs") {
        WorkspaceTestTargetKind::LibInCrate
    } else if normalized.contains("src/main.rs") || normalized.contains("src/bin/") {
        WorkspaceTestTargetKind::Binary
    } else if normalized.ends_with("_guard.rs") || normalized.contains("_guard-") {
        WorkspaceTestTargetKind::Guard
    } else {
        WorkspaceTestTargetKind::Integration
    }
}

/// Parsuje wyjście wykonania testów Cargo (`cargo test --workspace --all-targets`).
#[must_use]
pub fn parse_cargo_test_output(raw_output: &str) -> WorkspaceTestReport {
    let mut report = WorkspaceTestReport::new();
    let mut current_target: Option<(String, WorkspaceTestTargetKind)> = None;

    for line in raw_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Running ") {
            let target_part = rest.split(" (").next().unwrap_or(rest).trim();
            let kind = classify_target(target_part);
            current_target = Some((target_part.to_string(), kind));
        } else if trimmed.starts_with("Doc-tests") {
            let kind = WorkspaceTestTargetKind::DocTest;
            current_target = Some((trimmed.to_string(), kind));
        } else if let Some(rest) = trimmed.strip_prefix("test result: ") {
            let (target_name, kind) = current_target
                .take()
                .unwrap_or_else(|| ("unknown".to_string(), WorkspaceTestTargetKind::Integration));

            let mut passed = 0;
            let mut failed = 0;
            let mut ignored = 0;

            for part in rest.split(';') {
                let p = part.trim();
                if let Some(num_str) = p.strip_suffix("passed") {
                    let token = num_str.split_whitespace().last().unwrap_or("0");
                    passed = token.parse::<usize>().unwrap_or(0);
                } else if let Some(num_str) = p.strip_suffix("failed") {
                    let token = num_str.split_whitespace().last().unwrap_or("0");
                    failed = token.parse::<usize>().unwrap_or(0);
                } else if let Some(num_str) = p.strip_suffix("ignored") {
                    let token = num_str.split_whitespace().last().unwrap_or("0");
                    ignored = token.parse::<usize>().unwrap_or(0);
                }
            }

            let registered = passed + failed + ignored;
            report.push(TargetTestSummary {
                target_name,
                kind,
                registered,
                passed,
                failed,
                ignored,
            });
        }
    }

    report
}

/// Usuwa zduplikowane notacje matematyczne LaTeX (np. `TOTAL REGISTERED=2146\text{TOTAL REGISTERED}=2146`)
/// i zastępuje je czystym formatem tekstowym.
#[must_use]
pub fn sanitize_katex_duplicates(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            let mut math = String::new();
            while let Some(&next_c) = chars.peek() {
                chars.next();
                if next_c == '$' {
                    break;
                }
                math.push(next_c);
            }
            let cleaned = math
                .replace("\\text{", "")
                .replace('}', "")
                .replace("\\times", "×")
                .replace("\\,", " ");
            out.push_str(&cleaned);
        } else {
            out.push(c);
        }
    }

    // Usunięcie ewentualnych pozostałości \text{...}
    let mut result = String::with_capacity(out.len());
    let mut remaining = out.as_str();
    while let Some(idx) = remaining.find(r"\text{") {
        result.push_str(&remaining[..idx]);
        let after = &remaining[idx + 6..];
        if let Some(end_brace) = after.find('}') {
            remaining = &after[end_brace + 1..];
        } else {
            remaining = after;
        }
    }
    result.push_str(remaining);

    result
        .replace("O(N)O(N)O(N)", "O(N)")
        .replace(r"O(S×N)O(S \times N)O(S×N)", "O(S × N)")
        .replace(r"O(S×N)O(S \times N)O(S\times N)", "O(S × N)")
        .replace(r"O(S\times N)O(S \times N)O(S\times N)", "O(S × N)")
        .replace("O(S×N)O(S × N)O(S×N)", "O(S × N)")
        .replace("O(1)O(1)O(1)", "O(1)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_workspace_test_output() {
        let sample = r#"
     Running unittests src\lib.rs (target\debug\deps\splogbook-123.exe)
test result: ok. 1527 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.0s
     Running unittests src\main.rs (target\debug\deps\SPLogbook-456.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
     Running unittests src\bin\signer.rs (target\debug\deps\splogbook_signer-789.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
     Running tests\cat_native_6a.rs (target\debug\deps\cat_native_6a-abc.exe)
test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\cat_native_6b.rs (target\debug\deps\cat_native_6b-def.exe)
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\cat_native_7.rs (target\debug\deps\cat_native_7-ghi.exe)
test result: ok. 297 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\i18n_guard.rs (target\debug\deps\i18n_guard-jkl.exe)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.0s
     Running tests\security_guard.rs (target\debug\deps\security_guard-mno.exe)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\theme_guard.rs (target\debug\deps\theme_guard-pqr.exe)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s
   Doc-tests splogbook
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
        "#;

        let report = parse_cargo_test_output(sample);

        assert_eq!(report.lib_tests(), 1527);
        assert_eq!(report.binary_tests(), 0);
        assert_eq!(report.integration_tests(), 165 + 99 + 307);
        assert_eq!(report.guard_tests(), 15 + 32 + 1);
        assert_eq!(report.doc_tests(), 0);

        assert_eq!(report.total_registered(), 2146);
        assert_eq!(report.total_executed(), 2136);
        assert_eq!(report.total_passed(), 2136);
        assert_eq!(report.total_failed(), 0);
        assert_eq!(report.total_ignored(), 10);
        assert_eq!(report.hil_ignored(), 10);
    }

    #[test]
    fn classifies_targets_independently_of_path_separators() {
        assert_eq!(
            classify_target(r"src\lib.rs"),
            WorkspaceTestTargetKind::LibInCrate
        );
        assert_eq!(
            classify_target("src/lib.rs"),
            WorkspaceTestTargetKind::LibInCrate
        );
        assert_eq!(
            classify_target(r"src\main.rs"),
            WorkspaceTestTargetKind::Binary
        );
        assert_eq!(
            classify_target(r"src\bin\signer.rs"),
            WorkspaceTestTargetKind::Binary
        );
        assert_eq!(
            classify_target(r"tests\security_guard.rs"),
            WorkspaceTestTargetKind::Guard
        );
        assert_eq!(
            classify_target("tests/i18n_guard.rs"),
            WorkspaceTestTargetKind::Guard
        );
        assert_eq!(
            classify_target(r"tests\cat_native_6a.rs"),
            WorkspaceTestTargetKind::Integration
        );
        assert_eq!(
            classify_target("Doc-tests splogbook"),
            WorkspaceTestTargetKind::DocTest
        );
    }

    #[test]
    fn sanitizes_latex_math_duplicates() {
        let input = r"O(N)O(N)O(N) and O(S×N)O(S \times N)O(S×N) and $TOTAL REGISTERED=2146\text{TOTAL REGISTERED}=2146$";
        let cleaned = sanitize_katex_duplicates(input);
        assert!(!cleaned.contains("O(N)O(N)O(N)"));
        assert!(cleaned.contains("O(N)"));
        assert!(cleaned.contains("O(S × N)"));
        assert!(!cleaned.contains(r"\text{"));
    }
}
