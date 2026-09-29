// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Emisja zdarzeń w formacie N1MM Logger+ UDP broadcast.
//!
//! N1MM Logger+ oraz narzędzia pokrewne (GridTracker, overlay-e, pomocnicze
//! programy) nasłuchują na `127.0.0.1:12060` i interpretują ramki XML wysyłane
//! jako datagramy UDP. Dzięki temu SPLogbook może integrować się z istniejącym
//! ekosystemem narzędzi N1MM bez żadnych dodatkowych wtyczek.

use crate::core::qso::QsoRecord;
use tokio::net::UdpSocket;

/// Domyślny adres nasłuchu N1MM Logger+ (loopback).
pub const N1MM_DEFAULT_HOST: &str = "127.0.0.1";

/// Domyślny port UDP, na którym nasłuchuje N1MM Logger+ i narzędzia pokrewne.
pub const N1MM_DEFAULT_PORT: u16 = 12060;

/// Nazwa aplikacji umieszczana w polu `<app>` ramek.
const APP_NAME: &str = "SPLogbook";

/// Zamienia znaki specjalne XML na encje, aby ramka była poprawna.
fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Zamienia łańcuch daty/czasu ADIF na format oczekiwany przez N1MM
/// (`YYYY-MM-DD HH:MM:SS`). W razie braku danych zwraca bieżący czas UTC.
fn n1mm_timestamp(qso_date: &str, time_on: &str) -> String {
    let date = if qso_date.len() == 8 && qso_date.is_ascii() {
        format!(
            "{}-{}-{}",
            &qso_date[0..4],
            &qso_date[4..6],
            &qso_date[6..8]
        )
    } else if qso_date.len() >= 10 && qso_date.is_ascii() {
        qso_date[0..10].to_string()
    } else {
        String::new()
    };
    let time = if time_on.len() == 4 && time_on.is_ascii() {
        format!("{}:{}:00", &time_on[0..2], &time_on[2..4])
    } else if time_on.len() == 6 && time_on.is_ascii() {
        format!("{}:{}:{}", &time_on[0..2], &time_on[2..4], &time_on[4..6])
    } else if time_on.len() >= 8 && time_on.is_ascii() {
        time_on[0..8].to_string()
    } else {
        String::new()
    };
    match (date.is_empty(), time.is_empty()) {
        (false, false) => format!("{date} {time}"),
        (false, true) => format!("{date} 00:00:00"),
        (true, false) => time,
        (true, true) => chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

/// Zwraca prefix dla danego znaku (używany do WPX w N1MM).
fn wpx_prefix(callsign: &str) -> String {
    let pfx = crate::core::awards::extract_wpx_prefix(callsign);
    if pfx.is_empty() {
        callsign.to_uppercase()
    } else {
        pfx
    }
}

/// Buduje ramkę `<contactinfo>` reprezentującą zapisane QSO.
pub fn contactinfo_xml(qso: &QsoRecord, my_call: &str, radio_nr: u8) -> String {
    // W systemie N1MM wartości txfreq/rxfreq muszą być w rozdzielczości 10 Hz (czyli MHz * 100_000).
    let freq_n1mm = qso.freq.map_or(0, |f| (f * 100_000.0).round() as i64);
    let band_meters = band_to_meters(&qso.band);
    let continent = qso.continent.as_deref().unwrap_or("");
    let cqz = qso.cqz.map(|z| z.to_string()).unwrap_or_default();
    let timestamp = n1mm_timestamp(&qso.qso_date, &qso.time_on);

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<contactinfo>\n\
  <app>{app}</app>\n\
  <contestname>{app}</contestname>\n\
  <contestnr>1</contestnr>\n\
  <timestamp>{timestamp}</timestamp>\n\
  <mycall>{my_call}</mycall>\n\
  <band>{band}</band>\n\
  <rxfreq>{freq}</rxfreq>\n\
  <txfreq>{freq}</txfreq>\n\
  <operator>{my_call}</operator>\n\
  <mode>{mode}</mode>\n\
  <call>{call}</call>\n\
  <countryprefix>{countryprefix}</countryprefix>\n\
  <wpxprefix>{wpxprefix}</wpxprefix>\n\
  <stationprefix>{my_call}</stationprefix>\n\
  <continent>{continent}</continent>\n\
  <snt>{snt}</snt>\n\
  <rcv>{rcv}</rcv>\n\
  <gridsquare>{grid}</gridsquare>\n\
  <exchange1></exchange1>\n\
  <section>{state}</section>\n\
  <comment>{comment}</comment>\n\
  <qth>{qth}</qth>\n\
  <name>{name}</name>\n\
  <power></power>\n\
  <misctext></misctext>\n\
  <zone>{zone}</zone>\n\
  <prec></prec>\n\
  <ck>0</ck>\n\
  <ismultiplier1>0</ismultiplier1>\n\
  <ismultiplier2>0</ismultiplier2>\n\
  <ismultiplier3>0</ismultiplier3>\n\
  <points>1</points>\n\
  <radionr>{radio}</radionr>\n\
  <RoverLocation></RoverLocation>\n\
  <RadioInterfaced>1</RadioInterfaced>\n\
  <NetworkedCompNr>0</NetworkedCompNr>\n\
  <IsOriginal>True</IsOriginal>\n\
  <NetBiosName></NetBiosName>\n\
  <IsRunQSO>1</IsRunQSO>\n\
  <StationName>{my_call}</StationName>\n\
  <ID>{id}</ID>\n\
  <IsClaimedQso>True</IsClaimedQso>\n\
</contactinfo>\n",
        app = APP_NAME,
        timestamp = xml_escape(&timestamp),
        my_call = xml_escape(my_call),
        band = xml_escape(&band_meters),
        freq = freq_n1mm,
        mode = xml_escape(&qso.mode),
        call = xml_escape(&qso.callsign),
        countryprefix = xml_escape(&wpx_prefix(&qso.callsign)),
        wpxprefix = xml_escape(&wpx_prefix(&qso.callsign)),
        continent = xml_escape(continent),
        snt = xml_escape(&qso.rst_sent),
        rcv = xml_escape(&qso.rst_rcvd),
        grid = xml_escape(qso.gridsquare.as_deref().unwrap_or("")),
        state = xml_escape(qso.state.as_deref().unwrap_or("")),
        comment = xml_escape(qso.comment.as_deref().unwrap_or("")),
        qth = xml_escape(qso.qth.as_deref().unwrap_or("")),
        name = xml_escape(qso.name.as_deref().unwrap_or("")),
        zone = xml_escape(&cqz),
        radio = radio_nr,
        id = qso.id.unwrap_or_default(),
    )
}

/// Buduje ramkę `<radioinfo>` opisującą bieżący stan radia (częstotliwość/tryb).
pub fn radioinfo_xml(freq_hz: u64, mode: &str, my_call: &str, radio_nr: u8) -> String {
    // W systemie N1MM wartości Freq/TXFreq również muszą być w rozdzielczości 10 Hz.
    let freq_n1mm = freq_hz / 10;
    
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<radioinfo>\n\
  <app>{app}</app>\n\
  <StationName>{my_call}</StationName>\n\
  <RadioNr>{radio}</RadioNr>\n\
  <Freq>{freq}</Freq>\n\
  <TXFreq>{freq}</TXFreq>\n\
  <Mode>{mode}</Mode>\n\
  <OpCall>{my_call}</OpCall>\n\
  <IsRunning>True</IsRunning>\n\
  <FocusEntry></FocusEntry>\n\
  <Antenna>1</Antenna>\n\
  <Rotors></Rotors>\n\
  <FocusRadioNr>{radio}</FocusRadioNr>\n\
  <IsStereo>False</IsStereo>\n\
  <ActiveRadioNr>{radio}</ActiveRadioNr>\n\
</radioinfo>\n",
        app = APP_NAME,
        my_call = xml_escape(my_call),
        radio = radio_nr,
        freq = freq_n1mm,
        mode = xml_escape(mode),
    )
}

/// Zamienia pasmo ADIF (np. "20m", "70cm") na wartość numeryczną akceptowaną przez N1MM (np. "20", "70").
fn band_to_meters(band: &str) -> String {
    band.trim_end_matches(|c| c == 'm' || c == 'c' || c == 'M' || c == 'C').to_string()
}

/// Wysyła ramkę XML jako datagram UDP do podanego hosta i portu.
/// Zwraca liczbę wysłanych bajtów.
pub async fn send_broadcast(host: &str, port: u16, xml: &str) -> std::io::Result<usize> {
    let addr = format!("{host}:{port}");
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.send_to(xml.as_bytes(), &addr).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_handles_special_chars() {
        assert_eq!(
            xml_escape("a<b&c>d\"e'f"),
            "a&lt;b&amp;c&gt;d&quot;e&apos;f"
        );
        assert_eq!(xml_escape("normal"), "normal");
    }

    #[test]
    fn band_to_meters_strips_suffix() {
        assert_eq!(band_to_meters("20m"), "20");
        assert_eq!(band_to_meters("2m"), "2");
        assert_eq!(band_to_meters("70cm"), "70");
        assert_eq!(band_to_meters("1.25cm"), "1.25");
        assert_eq!(band_to_meters("6mm"), "6");
    }

    #[test]
    fn n1mm_timestamp_normalizes_adif_dates() {
        assert_eq!(
            n1mm_timestamp("2026-01-15", "12:34:56"),
            "2026-01-15 12:34:56"
        );
        assert_eq!(n1mm_timestamp("20260115", "123456"), "2026-01-15 12:34:56");
        assert_eq!(n1mm_timestamp("", "").len(), 19);
    }

    #[test]
    fn contactinfo_contains_key_fields() {
        let qso = QsoRecord {
            callsign: "SP9ABC".to_string(),
            band: "20m".to_string(),
            mode: "SSB".to_string(),
            freq: Some(14.200),
            rst_sent: "59".to_string(),
            rst_rcvd: "59".to_string(),
            gridsquare: Some("JO90".to_string()),
            cqz: Some(15),
            qso_date: "2026-01-15".to_string(),
            time_on: "12:34:56".to_string(),
            ..Default::default()
        };
        let xml = contactinfo_xml(&qso, "SP6INA", 1);
        assert!(xml.contains("<contactinfo>"));
        assert!(xml.contains("<call>SP9ABC</call>"));
        assert!(xml.contains("<mycall>SP6INA</mycall>"));
        assert!(xml.contains("<rxfreq>1420000</rxfreq>"));
        assert!(xml.contains("<band>20</band>"));
        assert!(xml.contains("<gridsquare>JO90</gridsquare>"));
    }

    #[test]
    fn contactinfo_escapes_user_supplied_text() {
        let qso = QsoRecord {
            callsign: "SP9A<B>".to_string(),
            comment: Some("test & more".to_string()),
            ..Default::default()
        };
        let xml = contactinfo_xml(&qso, "SP6INA", 1);
        assert!(xml.contains("<call>SP9A&lt;B&gt;</call>"));
        assert!(xml.contains("<comment>test &amp; more</comment>"));
    }

    #[test]
    fn radioinfo_scales_frequency_correctly() {
        let xml = radioinfo_xml(14025000, "CW", "SP6INA", 1);
        assert!(xml.contains("<Freq>1402500</Freq>"));
        assert!(xml.contains("<TXFreq>1402500</TXFreq>"));
    }
}
