// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Application configuration and database paths.

use std::path::PathBuf;

/// Application file paths and secret store identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppPaths {
    pub config_file_path: PathBuf,
    pub active_db_path: PathBuf,
    pub secret_store_id: String,
}

impl AppPaths {
    pub fn new(
        config_file_path: PathBuf,
        active_db_path: PathBuf,
        secret_store_id: String,
    ) -> Self {
        Self {
            config_file_path,
            active_db_path,
            secret_store_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let paths = AppPaths::default();
        assert!(paths.config_file_path.as_os_str().is_empty());
        assert!(paths.active_db_path.as_os_str().is_empty());
        assert!(paths.secret_store_id.is_empty());
    }
}
