// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Mostek poleceń i migawka stanu między skryptami Rhai a aplikacją.
//!
//! Skrypty pluginów nie mają bezpośredniego dostępu do sprzętu ani bazy.
//! Zamiast tego wywołują zarejestrowane akcje (np. `send_cw`, `rotate`, `spot`),
//! które trafiają do kolejki [`PluginCommand`] i są wykonywane przez aplikację
//! w bezpiecznym kontekście. Dane odczytowe (radio, rotor, nagrody, ostatnia
//! łączność) plugin pobiera przez gettery oparte na [`PluginSnapshot`].

/// Wynik asynchronicznego zapytania POTA/SOTA z akcji pluginów.
#[derive(Debug, Clone)]
pub enum PluginLookupResult {
    Pota {
        reference: String,
        name: String,
        active: bool,
    },
    Sota {
        reference: String,
        name: String,
        points: i64,
    },
    Error(String),
}

/// Polecenie wydane przez plugin do wykonania przez aplikację.
#[derive(Debug, Clone)]
pub enum PluginCommand {
    /// Nadaj tekst alfabetem Morse'a (CW).
    SendCw { text: String },
    /// Odtwórz wiadomość głosową (Voice Keyer) o podanej nazwie.
    SendVoice { text: String },
    /// Obróć antenę na podany azymut i elewację.
    Rotate {
        azimuth_deg: f32,
        elevation_deg: f32,
    },
    /// Wyślij spot DX na klaster.
    Spot {
        dx_call: String,
        freq_khz: f64,
        comment: String,
    },
    /// Ustaw pole formularza QSO (np. "name", "qth", "comment").
    SetQsoField { field: String, value: String },
    /// Odtwórz dźwięk powiadomienia o podanej nazwie.
    PlaySound { name: String },
    /// Zapytaj o referencję POTA (wynik trafi do haka `on_pota_info`).
    PotaLookup { reference: String },
    /// Zapytaj o referencję SOTA (wynik trafi do haka `on_sota_info`).
    SotaLookup { reference: String },
    /// Wyświetl powiadomienie/toast w aplikacji.
    Notify { message: String },
}

/// Statystyki nagród widoczne dla pluginów (gettery `*_worked`/`*_confirmed`).
#[derive(Debug, Clone, Default)]
pub struct AwardSnapshot {
    pub dxcc_worked: i64,
    pub dxcc_confirmed: i64,
    pub waz_worked: i64,
    pub was_worked: i64,
    pub wac_worked: i64,
    pub iota_worked: i64,
    pub pota_parks_worked: i64,
    pub sota_summits_worked: i64,
    pub pga_gminas_worked: i64,
}

/// Ostatnio zapisana łączność — do odczytu przez `qso_field(...)`.
#[derive(Debug, Clone, Default)]
pub struct QsoSnapshot {
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub freq_mhz: f64,
    pub name: String,
    pub qth: String,
    pub gridsquare: String,
    pub country: String,
    pub dxcc: String,
    pub sota_ref: String,
    pub pota_ref: String,
    pub pga_ref: String,
    pub iota: String,
    pub state: String,
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub comment: String,
    pub is_atno: bool,
    pub is_new_band: bool,
    pub is_new_mode: bool,
    pub is_new_iota: bool,
    pub is_new_waz: bool,
    pub is_new_was: bool,
    pub is_new_wac: bool,
    pub is_new_pga: bool,
}

impl QsoSnapshot {
    /// Odczytuje pole ostatniej łączności po nazwie (getter `qso_field(name)`).
    /// Nieznane pole zwraca pusty łańcuch.
    pub fn field(&self, name: &str) -> String {
        match name {
            "callsign" => self.callsign.clone(),
            "band" => self.band.clone(),
            "mode" => self.mode.clone(),
            "freq_mhz" => format_freq(self.freq_mhz),
            "name" => self.name.clone(),
            "qth" => self.qth.clone(),
            "gridsquare" => self.gridsquare.clone(),
            "country" => self.country.clone(),
            "dxcc" => self.dxcc.clone(),
            "sota_ref" => self.sota_ref.clone(),
            "pota_ref" => self.pota_ref.clone(),
            "pga_ref" => self.pga_ref.clone(),
            "iota" => self.iota.clone(),
            "state" => self.state.clone(),
            "rst_sent" => self.rst_sent.clone(),
            "rst_rcvd" => self.rst_rcvd.clone(),
            "comment" => self.comment.clone(),
            _ => String::new(),
        }
    }
}

fn format_freq(mhz: f64) -> String {
    if mhz == 0.0 {
        String::new()
    } else {
        format!("{mhz:.4}")
    }
}

/// Migawka stanu radia/rotora/nagród udostępniana pluginom.
#[derive(Debug, Clone, Default)]
pub struct PluginSnapshot {
    pub rig_freq_mhz: f64,
    pub rig_mode: String,
    pub rig_band: String,
    pub rig_connected: bool,
    pub rotor_azimuth_deg: f32,
    pub rotor_elevation_deg: f32,
    pub my_call: String,
    pub qso_count: i64,
    pub awards: AwardSnapshot,
    pub last_qso: QsoSnapshot,
}
