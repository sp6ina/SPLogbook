// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Klucz CW działający w tle: K1EL WinKeyer, linia DTR/RTS portu szeregowego albo
//! polecenie `send_morse` demona rigctld (Hamlib).
//!
//! Każdy rodzaj klucza to zadanie tokio, które przyjmuje polecenia [`CwCommand`]
//! i zgłasza zdarzenia [`CwEvent`]. Postęp nadawania jest liczony w znakach tekstu
//! przekazanego w [`CwCommand::Send`], więc terminal CW może pokazać, co już poszło
//! w eter. Zamknięcie uchwytu kończy zadanie i zawsze zwalnia klucz.

use crate::cat::morse;
use crate::cat::winkeyer::{WinkeyerByte, WinkeyerProtocol};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;

/// Czas na odpowiedź WinKeyera po `Host Open` (bajt wersji firmware).
const WINKEYER_OPEN_TIMEOUT: Duration = Duration::from_secs(2);
/// Brak jakiejkolwiek odpowiedzi WinKeyera przy niepustym buforze przez ten czas = awaria.
const WINKEYER_STALL_TIMEOUT: Duration = Duration::from_secs(10);

/// Rodzaj klucza CW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CwKeyerKind {
    #[default]
    WinKeyer,
    /// Kluczowanie linią DTR portu szeregowego.
    SerialDtr,
    /// Kluczowanie linią RTS portu szeregowego.
    SerialRts,
    /// `send_morse` przez rigctld (radio nadaje tekst swoim kluczem).
    Hamlib,
}

/// Ustawienia klucza zapisywane w konfiguracji.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CwKeyerSettings {
    pub kind: CwKeyerKind,
    /// Port szeregowy WinKeyera albo kluczowanej linii, np. `COM5` lub `/dev/ttyUSB0`.
    pub port: String,
    pub wpm: u8,
    pub sidetone_hz: u16,
    /// Kluczowanie linią: druga linia portu (RTS przy DTR, DTR przy RTS) jako PTT.
    pub line_ptt: bool,
    /// Wyprzedzenie PTT przed pierwszym elementem (kluczowanie linią).
    pub ptt_lead_ms: u16,
    /// Opóźnienie zwolnienia PTT po ostatnim elemencie (kluczowanie linią).
    pub ptt_tail_ms: u16,
}

impl Default for CwKeyerSettings {
    fn default() -> Self {
        Self {
            kind: CwKeyerKind::WinKeyer,
            port: String::new(),
            wpm: 24,
            sidetone_hz: 700,
            line_ptt: false,
            ptt_lead_ms: 50,
            ptt_tail_ms: 150,
        }
    }
}

/// Polecenie dla klucza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwCommand {
    Send(String),
    /// Natychmiastowe przerwanie: bufor klucza jest czyszczony, klucz zwolniony.
    Abort,
    SetSpeed(u8),
}

/// Zdarzenie zgłaszane przez klucz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwEvent {
    /// Klucz gotowy; opis (np. wersja firmware WinKeyera).
    Ready(String),
    /// Nadano kolejne `n` znaków tekstu z [`CwCommand::Send`].
    Sent(usize),
    /// Bufor pusty, klucz zwolniony.
    Idle,
    Error(CwKeyerError),
    /// Zadanie klucza zakończyło się (po zamknięciu uchwytu albo błędzie połączenia).
    Closed,
}

/// Błąd klucza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwKeyerError {
    /// Nie udało się otworzyć portu.
    Open { port: String, detail: String },
    /// WinKeyer nie odpowiedział na `Host Open`.
    NoAnswer,
    /// WinKeyer przestał odpowiadać w trakcie nadawania.
    Stalled,
    /// Błąd odczytu lub zapisu.
    Io(String),
    /// rigctld odrzucił polecenie (`RPRT <kod>`).
    Rejected(i32),
}

impl std::fmt::Display for CwKeyerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { port, detail } => write!(f, "{port}: {detail}"),
            Self::NoAnswer => write!(f, "no answer to Host Open"),
            Self::Stalled => write!(f, "the keyer stopped answering"),
            Self::Io(detail) => write!(f, "{detail}"),
            Self::Rejected(code) => write!(f, "RPRT {code}"),
        }
    }
}

impl std::error::Error for CwKeyerError {}

/// Uchwyt działającego klucza. Upuszczenie uchwytu zamyka klucz.
pub struct CwKeyerHandle {
    commands: mpsc::UnboundedSender<CwCommand>,
    events: std::sync::mpsc::Receiver<CwEvent>,
}

impl CwKeyerHandle {
    /// Wysyła polecenie; `false`, gdy zadanie klucza już się zakończyło.
    pub fn send(&self, command: CwCommand) -> bool {
        self.commands.send(command).is_ok()
    }

    /// Zdarzenia zebrane od ostatniego wywołania.
    pub fn drain_events(&self) -> Vec<CwEvent> {
        self.events.try_iter().collect()
    }
}

type EventSink = std::sync::mpsc::Sender<CwEvent>;

fn channels() -> (
    mpsc::UnboundedSender<CwCommand>,
    mpsc::UnboundedReceiver<CwCommand>,
    EventSink,
    std::sync::mpsc::Receiver<CwEvent>,
) {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (ev_tx, ev_rx) = std::sync::mpsc::channel();
    (cmd_tx, cmd_rx, ev_tx, ev_rx)
}

// ---------------------------------------------------------------------------------------------
// Tekst → bajty WinKeyera ze śledzeniem znaków źródłowych
// ---------------------------------------------------------------------------------------------

/// Bajty dla WinKeyera wraz z liczbą znaków tekstu źródłowego, którą każdy z nich
/// „pokrywa”. Znaki odrzucone (bez odpowiednika w WinKeyerze) doliczają się do
/// następnego bajtu, a na końcu tekstu — do ostatniego. `leftover` to znaki, którym
/// nie odpowiada żaden bajt (tekst bez znaków do nadania).
fn encode_with_spans(text: &str) -> (Vec<(u8, usize)>, usize) {
    let mut out: Vec<(u8, usize)> = Vec::new();
    let mut pending = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c == '<'
            && let Some(rel_end) = text[idx..].find('>')
        {
            let token = &text[idx..=idx + rel_end];
            let encoded = WinkeyerProtocol::encode_text(token);
            if encoded.len() == 1 {
                let span = token.chars().count();
                for _ in 1..span {
                    chars.next();
                }
                out.push((encoded[0], pending + span));
                pending = 0;
                continue;
            }
        }
        let mut buf = [0u8; 4];
        let encoded = WinkeyerProtocol::encode_text(c.encode_utf8(&mut buf));
        match encoded.first() {
            Some(&b) => {
                out.push((b, pending + 1));
                pending = 0;
            }
            None => pending += 1,
        }
    }
    match out.last_mut() {
        Some(last) => {
            last.1 += pending;
            (out, 0)
        }
        None => (out, pending),
    }
}

// ---------------------------------------------------------------------------------------------
// WinKeyer
// ---------------------------------------------------------------------------------------------

/// Otwiera WinKeyera na porcie szeregowym (1200 Bd, 8N2) i uruchamia jego zadanie.
pub fn open_winkeyer(settings: &CwKeyerSettings) -> Result<CwKeyerHandle, CwKeyerError> {
    use tokio_serial::SerialPortBuilderExt;
    let stream = tokio_serial::new(&settings.port, 1200)
        .data_bits(tokio_serial::DataBits::Eight)
        .stop_bits(tokio_serial::StopBits::Two)
        .parity(tokio_serial::Parity::None)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()
        .map_err(|e| CwKeyerError::Open {
            port: settings.port.clone(),
            detail: e.to_string(),
        })?;
    Ok(spawn_winkeyer(stream, settings.clone()))
}

/// Uruchamia zadanie WinKeyera na dowolnym strumieniu (port szeregowy albo, w testach,
/// wirtualny port w pamięci).
pub fn spawn_winkeyer<S>(stream: S, settings: CwKeyerSettings) -> CwKeyerHandle
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (cmd_tx, cmd_rx, ev_tx, ev_rx) = channels();
    tokio::spawn(async move {
        if let Err(e) = run_winkeyer(stream, settings, cmd_rx, &ev_tx).await {
            let _ = ev_tx.send(CwEvent::Error(e));
        }
        let _ = ev_tx.send(CwEvent::Closed);
    });
    CwKeyerHandle {
        commands: cmd_tx,
        events: ev_rx,
    }
}

async fn write_all<S: AsyncWrite + Unpin>(
    stream: &mut S,
    bytes: &[u8],
) -> Result<(), CwKeyerError> {
    stream
        .write_all(bytes)
        .await
        .map_err(|e| CwKeyerError::Io(e.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|e| CwKeyerError::Io(e.to_string()))
}

async fn run_winkeyer<S>(
    mut stream: S,
    settings: CwKeyerSettings,
    mut commands: mpsc::UnboundedReceiver<CwCommand>,
    events: &EventSink,
) -> Result<(), CwKeyerError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Host Open: WinKeyer odsyła numer wersji firmware; dopiero potem przyjmuje polecenia.
    write_all(&mut stream, &WinkeyerProtocol::host_open()).await?;
    let mut first = [0u8; 1];
    let version = match tokio::time::timeout(WINKEYER_OPEN_TIMEOUT, stream.read(&mut first)).await {
        Ok(Ok(1)) => first[0],
        Ok(Ok(_)) | Err(_) => return Err(CwKeyerError::NoAnswer),
        Ok(Err(e)) => return Err(CwKeyerError::Io(e.to_string())),
    };
    // Po Host Open WinKeyer (WK2 i WK3) pracuje w trybie zgodności WK1, więc sidetone
    // ustawia się z tabeli WK1/WK2.
    write_all(&mut stream, &WinkeyerProtocol::set_mode_serial_echo()).await?;
    write_all(&mut stream, &WinkeyerProtocol::set_speed(settings.wpm)).await?;
    write_all(
        &mut stream,
        &WinkeyerProtocol::set_sidetone(settings.sidetone_hz, false),
    )
    .await?;
    let _ = events.send(CwEvent::Ready(format!("WinKeyer v{version}")));

    let mut queued: VecDeque<(u8, usize)> = VecDeque::new();
    let mut in_flight: VecDeque<usize> = VecDeque::new();
    let mut xoff = false;
    let mut busy = false;
    let mut buf = [0u8; 64];

    let result = loop {
        // Bajty do bufora WinKeyera, dopóki nie ma XOFF i jest miejsce.
        let mut chunk = Vec::new();
        while !xoff && in_flight.len() < WinkeyerProtocol::MAX_IN_FLIGHT {
            let Some((byte, span)) = queued.pop_front() else {
                break;
            };
            chunk.push(byte);
            in_flight.push_back(span);
        }
        if !chunk.is_empty()
            && let Err(e) = write_all(&mut stream, &chunk).await
        {
            break Err(e);
        }

        let waiting = !in_flight.is_empty();
        tokio::select! {
            command = commands.recv() => match command {
                Some(CwCommand::Send(text)) => {
                    let (bytes, leftover) = encode_with_spans(&text);
                    if leftover > 0 {
                        let _ = events.send(CwEvent::Sent(leftover));
                    }
                    queued.extend(bytes);
                }
                Some(CwCommand::Abort) => {
                    queued.clear();
                    in_flight.clear();
                    if let Err(e) = write_all(&mut stream, &WinkeyerProtocol::abort_buffer()).await {
                        break Err(e);
                    }
                    let _ = events.send(CwEvent::Idle);
                }
                Some(CwCommand::SetSpeed(wpm)) => {
                    if let Err(e) = write_all(&mut stream, &WinkeyerProtocol::set_speed(wpm)).await {
                        break Err(e);
                    }
                }
                None => break Ok(()),
            },
            read = tokio::time::timeout(WINKEYER_STALL_TIMEOUT, stream.read(&mut buf)) => match read {
                Ok(Ok(0)) => break Err(CwKeyerError::Io("port closed".to_string())),
                Ok(Ok(n)) => {
                    for &byte in &buf[..n] {
                        match WinkeyerProtocol::classify(byte) {
                            WinkeyerByte::Echo(_) => {
                                if let Some(span) = in_flight.pop_front() {
                                    let _ = events.send(CwEvent::Sent(span));
                                }
                            }
                            WinkeyerByte::Status(status) if !status.is_pushbutton() => {
                                xoff = status.xoff();
                                let was_busy = busy;
                                busy = status.busy();
                                // Koniec nadawania: znaki bez echa (np. odstępy) też poszły.
                                if was_busy && !busy && queued.is_empty() {
                                    let rest: usize = in_flight.drain(..).sum();
                                    if rest > 0 {
                                        let _ = events.send(CwEvent::Sent(rest));
                                    }
                                    let _ = events.send(CwEvent::Idle);
                                }
                            }
                            WinkeyerByte::Status(_) | WinkeyerByte::SpeedPot(_) => {}
                        }
                    }
                }
                Ok(Err(e)) => break Err(CwKeyerError::Io(e.to_string())),
                Err(_) if waiting => break Err(CwKeyerError::Stalled),
                Err(_) => {}
            },
        }
    };

    // Zawsze: przerwij bufor i oddaj WinKeyera do trybu samodzielnego.
    let _ = write_all(&mut stream, &WinkeyerProtocol::abort_buffer()).await;
    let _ = write_all(&mut stream, &WinkeyerProtocol::host_close()).await;
    result
}

// ---------------------------------------------------------------------------------------------
// Kluczowanie linią DTR/RTS
// ---------------------------------------------------------------------------------------------

/// Linie portu szeregowego używane do kluczowania i PTT.
pub trait KeyLines: Send + 'static {
    fn set_key(&mut self, down: bool) -> std::io::Result<()>;
    fn set_ptt(&mut self, on: bool) -> std::io::Result<()>;
}

struct SerialKeyLines {
    port: tokio_serial::SerialStream,
    key_on_dtr: bool,
    line_ptt: bool,
}

impl KeyLines for SerialKeyLines {
    fn set_key(&mut self, down: bool) -> std::io::Result<()> {
        use tokio_serial::SerialPort;
        let result = if self.key_on_dtr {
            self.port.write_data_terminal_ready(down)
        } else {
            self.port.write_request_to_send(down)
        };
        result.map_err(std::io::Error::other)
    }

    fn set_ptt(&mut self, on: bool) -> std::io::Result<()> {
        use tokio_serial::SerialPort;
        if !self.line_ptt {
            return Ok(());
        }
        let result = if self.key_on_dtr {
            self.port.write_request_to_send(on)
        } else {
            self.port.write_data_terminal_ready(on)
        };
        result.map_err(std::io::Error::other)
    }
}

/// Otwiera port szeregowy do kluczowania linią DTR albo RTS.
pub fn open_serial_line(settings: &CwKeyerSettings) -> Result<CwKeyerHandle, CwKeyerError> {
    use tokio_serial::{SerialPort, SerialPortBuilderExt};
    let mut port = tokio_serial::new(&settings.port, 9600)
        .open_native_async()
        .map_err(|e| CwKeyerError::Open {
            port: settings.port.clone(),
            detail: e.to_string(),
        })?;
    // Port otwiera się z liniami w stanie zależnym od sterownika — najpierw je zwolnij.
    let _ = port.write_data_terminal_ready(false);
    let _ = port.write_request_to_send(false);
    let lines = SerialKeyLines {
        port,
        key_on_dtr: settings.kind != CwKeyerKind::SerialRts,
        line_ptt: settings.line_ptt,
    };
    Ok(spawn_line_keyer(lines, settings.clone()))
}

/// Uruchamia kluczowanie dowolnymi liniami (port szeregowy albo, w testach, atrapa).
pub fn spawn_line_keyer<L: KeyLines>(lines: L, settings: CwKeyerSettings) -> CwKeyerHandle {
    let (cmd_tx, cmd_rx, ev_tx, ev_rx) = channels();
    tokio::spawn(async move {
        let _ = ev_tx.send(CwEvent::Ready(String::new()));
        if let Err(e) = run_line_keyer(lines, settings, cmd_rx, &ev_tx).await {
            let _ = ev_tx.send(CwEvent::Error(e));
        }
        let _ = ev_tx.send(CwEvent::Closed);
    });
    CwKeyerHandle {
        commands: cmd_tx,
        events: ev_rx,
    }
}

/// Znak (albo prosign) do zakluczowania razem z liczbą znaków źródłowych.
fn line_tokens(text: &str) -> VecDeque<(String, usize)> {
    let mut out = VecDeque::new();
    let mut chars = text.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c == '<'
            && let Some(rel_end) = text[idx..].find('>')
        {
            let token = &text[idx..=idx + rel_end];
            for _ in 1..token.chars().count() {
                chars.next();
            }
            out.push_back((token.to_string(), token.chars().count()));
            continue;
        }
        out.push_back((c.to_string(), 1));
    }
    out
}

async fn run_line_keyer<L: KeyLines>(
    mut lines: L,
    settings: CwKeyerSettings,
    mut commands: mpsc::UnboundedReceiver<CwCommand>,
    events: &EventSink,
) -> Result<(), CwKeyerError> {
    let io = |e: std::io::Error| CwKeyerError::Io(e.to_string());
    let mut wpm = settings.wpm;
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    // Trwa nadawanie (PTT linią albo sam klucz): po ostatnim znaku czekamy ogon PTT.
    let mut transmitting = false;
    let mut previous_was_char = false;

    let result: Result<(), CwKeyerError> = 'run: loop {
        if queue.is_empty() {
            if transmitting {
                // Ogon PTT; nowy tekst w tym czasie podtrzymuje nadawanie.
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_millis(settings.ptt_tail_ms.into())) => {
                        if let Err(e) = lines.set_ptt(false) {
                            break 'run Err(io(e));
                        }
                        transmitting = false;
                        previous_was_char = false;
                        let _ = events.send(CwEvent::Idle);
                        continue 'run;
                    }
                    command = commands.recv() => match command {
                        Some(command) => {
                            apply_line_command(command, &mut queue, &mut wpm, events);
                            continue 'run;
                        }
                        None => break 'run Ok(()),
                    },
                }
            }
            match commands.recv().await {
                Some(command) => {
                    apply_line_command(command, &mut queue, &mut wpm, events);
                    continue 'run;
                }
                None => break 'run Ok(()),
            }
        }

        if !transmitting {
            if settings.line_ptt {
                if let Err(e) = lines.set_ptt(true) {
                    break 'run Err(io(e));
                }
                tokio::time::sleep(Duration::from_millis(settings.ptt_lead_ms.into())).await;
            }
            transmitting = true;
        }

        let Some((token, span)) = queue.pop_front() else {
            continue 'run;
        };
        let steps = morse::keying_steps(&token);
        if steps.is_empty() {
            // Odstęp słów: 7 jednostek po znaku. Znak bez kodu Morse'a jest pomijany.
            if token.trim().is_empty() && previous_was_char {
                tokio::time::sleep(morse::unit(wpm) * 7).await;
                previous_was_char = false;
            }
            let _ = events.send(CwEvent::Sent(span));
            continue 'run;
        }
        if previous_was_char {
            tokio::time::sleep(morse::unit(wpm) * 3).await;
        }
        for step in steps {
            if let Err(e) = lines.set_key(step.key_down) {
                break 'run Err(io(e));
            }
            let pause = tokio::time::sleep(morse::unit(wpm) * step.units);
            tokio::pin!(pause);
            loop {
                tokio::select! {
                    () = &mut pause => break,
                    command = commands.recv() => match command {
                        Some(CwCommand::Abort) => {
                            if let Err(e) = lines.set_key(false) {
                                break 'run Err(io(e));
                            }
                            queue.clear();
                            previous_was_char = false;
                            let _ = events.send(CwEvent::Idle);
                            continue 'run;
                        }
                        Some(command) => apply_line_command(command, &mut queue, &mut wpm, events),
                        None => break 'run Ok(()),
                    },
                }
            }
        }
        if let Err(e) = lines.set_key(false) {
            break 'run Err(io(e));
        }
        previous_was_char = true;
        let _ = events.send(CwEvent::Sent(span));
    };

    // Zawsze zwolnij klucz i PTT.
    let _ = lines.set_key(false);
    let _ = lines.set_ptt(false);
    result
}

fn apply_line_command(
    command: CwCommand,
    queue: &mut VecDeque<(String, usize)>,
    wpm: &mut u8,
    events: &EventSink,
) {
    match command {
        CwCommand::Send(text) => queue.extend(line_tokens(&text)),
        CwCommand::Abort => {
            queue.clear();
            let _ = events.send(CwEvent::Idle);
        }
        CwCommand::SetSpeed(new) => *wpm = new.clamp(morse::MIN_WPM, morse::MAX_WPM),
    }
}

// ---------------------------------------------------------------------------------------------
// rigctld (Hamlib) send_morse
// ---------------------------------------------------------------------------------------------

/// Tekst dla `send_morse`: wielkie litery, tylko znaki z kodem Morse'a i spacje;
/// prosigny są zapisywane literami (`<SK>` → `SK`), bo rigctld nie zna nawiasów.
fn hamlib_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = c.to_ascii_uppercase();
        if c == ' ' || (morse::code(c).is_some() && c != ';') {
            out.push(c);
        } else if c.is_whitespace() {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Uruchamia klucz nadający przez rigctld pod adresem `host:port`.
pub fn spawn_hamlib(host: String, port: u16, wpm: u8) -> CwKeyerHandle {
    let (cmd_tx, mut cmd_rx, ev_tx, ev_rx) = channels();
    tokio::spawn(async move {
        let addr = format!("{host}:{port}");
        let send = |cmd: String| {
            let addr = addr.clone();
            async move {
                let reply = crate::cat::tcp::send_cmd_read_reply(&addr, &cmd, "rigctld")
                    .await
                    .map_err(|e| CwKeyerError::Io(e.to_string()))?;
                match reply.as_deref().and_then(rprt_code) {
                    Some(code) if code != 0 => Err(CwKeyerError::Rejected(code)),
                    _ => Ok(()),
                }
            }
        };
        let speed = send(format!("L KEYSPD {wpm}\n")).await;
        match speed {
            Ok(()) => {
                let _ = ev_tx.send(CwEvent::Ready(addr.clone()));
            }
            Err(e) => {
                let _ = ev_tx.send(CwEvent::Error(e));
                let _ = ev_tx.send(CwEvent::Closed);
                return;
            }
        }
        while let Some(command) = cmd_rx.recv().await {
            let outcome = match command {
                CwCommand::Send(text) => {
                    let total = text.chars().count();
                    let morse_text = hamlib_text(&text);
                    let result = if morse_text.is_empty() {
                        Ok(())
                    } else {
                        send(format!("b {morse_text}\n")).await
                    };
                    if result.is_ok() {
                        let _ = ev_tx.send(CwEvent::Sent(total));
                        let _ = ev_tx.send(CwEvent::Idle);
                    }
                    result
                }
                CwCommand::Abort => {
                    let result = send("\\stop_morse\n".to_string()).await;
                    let _ = ev_tx.send(CwEvent::Idle);
                    result
                }
                CwCommand::SetSpeed(wpm) => send(format!("L KEYSPD {wpm}\n")).await,
            };
            if let Err(e) = outcome {
                let _ = ev_tx.send(CwEvent::Error(e));
            }
        }
        let _ = ev_tx.send(CwEvent::Closed);
    });
    CwKeyerHandle {
        commands: cmd_tx,
        events: ev_rx,
    }
}

fn rprt_code(reply: &str) -> Option<i32> {
    reply.trim().strip_prefix("RPRT")?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::io::DuplexStream;

    /// Odbiór zdarzeń klucza po kolei (bez gubienia zdarzeń zebranych naraz).
    struct Events<'a> {
        handle: &'a CwKeyerHandle,
        pending: VecDeque<CwEvent>,
    }

    impl<'a> Events<'a> {
        fn new(handle: &'a CwKeyerHandle) -> Self {
            Self {
                handle,
                pending: VecDeque::new(),
            }
        }

        async fn next(&mut self) -> CwEvent {
            for _ in 0..20_000 {
                self.pending.extend(self.handle.drain_events());
                if let Some(event) = self.pending.pop_front() {
                    return event;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            panic!("no keyer event");
        }

        /// Zdarzenia aż do `until` włącznie.
        async fn until(&mut self, until: &CwEvent) -> Vec<CwEvent> {
            let mut seen = Vec::new();
            loop {
                let event = self.next().await;
                let done = event == *until;
                seen.push(event);
                if done {
                    return seen;
                }
            }
        }
    }

    async fn read_exact(device: &mut DuplexStream, n: usize) -> Vec<u8> {
        let mut buf = vec![0u8; n];
        device.read_exact(&mut buf).await.unwrap();
        buf
    }

    #[test]
    fn text_spans_cover_every_source_character() {
        let (bytes, leftover) = encode_with_spans("TU <SK>");
        assert_eq!(bytes, vec![(b'T', 1), (b'U', 1), (b' ', 1), (b'>', 4)]);
        assert_eq!(leftover, 0);
        // Znaki odrzucone doliczają się do następnego bajtu, a na końcu do ostatniego.
        let (bytes, _) = encode_with_spans("Ł5NN#");
        assert_eq!(bytes, vec![(b'5', 2), (b'N', 1), (b'N', 2)]);
        let (bytes, leftover) = encode_with_spans("ŁÓ");
        assert_eq!((bytes, leftover), (Vec::new(), 2));
    }

    #[tokio::test]
    async fn winkeyer_opens_sends_reports_echo_and_closes_cleanly() {
        let (host, mut device) = tokio::io::duplex(256);
        let handle = spawn_winkeyer(host, CwKeyerSettings::default());
        let mut ev = Events::new(&handle);

        assert_eq!(
            read_exact(&mut device, 2).await,
            vec![0x00, 0x02],
            "Host Open"
        );
        device.write_all(&[31]).await.unwrap();
        // Tryb z echem, prędkość 24 WPM, sidetone 700 Hz (tabela WK1/WK2: n = 6).
        assert_eq!(
            read_exact(&mut device, 6).await,
            vec![0x0E, 0x04, 0x02, 24, 0x01, 6]
        );
        assert_eq!(ev.next().await, CwEvent::Ready("WinKeyer v31".into()));

        assert!(handle.send(CwCommand::Send("CQ <SK>".to_string())));
        assert_eq!(read_exact(&mut device, 4).await, b"CQ >".to_vec());
        device
            .write_all(&[0xC4, b'C', b'Q', b'>', 0xC0])
            .await
            .unwrap();
        let events = ev.until(&CwEvent::Idle).await;
        assert_eq!(
            events,
            vec![
                CwEvent::Sent(1),
                CwEvent::Sent(1),
                CwEvent::Sent(1),
                CwEvent::Sent(4),
                CwEvent::Idle
            ],
            "the space has no echo and is accounted for when the keyer goes idle"
        );

        drop(ev);
        drop(handle);
        assert_eq!(
            read_exact(&mut device, 3).await,
            vec![0x0A, 0x00, 0x03],
            "Clear Buffer and Host Close on shutdown"
        );
    }

    #[tokio::test]
    async fn winkeyer_abort_clears_the_buffer_immediately() {
        let (host, mut device) = tokio::io::duplex(256);
        let handle = spawn_winkeyer(host, CwKeyerSettings::default());
        let mut ev = Events::new(&handle);
        read_exact(&mut device, 2).await;
        device.write_all(&[23]).await.unwrap();
        read_exact(&mut device, 6).await;
        ev.next().await;

        handle.send(CwCommand::Send("TEST".to_string()));
        assert_eq!(read_exact(&mut device, 4).await, b"TEST".to_vec());
        handle.send(CwCommand::Abort);
        assert_eq!(read_exact(&mut device, 1).await, vec![0x0A]);
        assert_eq!(ev.next().await, CwEvent::Idle);
    }

    #[tokio::test]
    async fn winkeyer_respects_xoff_and_the_in_flight_limit() {
        let (host, mut device) = tokio::io::duplex(1024);
        let handle = spawn_winkeyer(host, CwKeyerSettings::default());
        let mut ev = Events::new(&handle);
        read_exact(&mut device, 2).await;
        device.write_all(&[31]).await.unwrap();
        read_exact(&mut device, 6).await;
        ev.next().await;

        handle.send(CwCommand::Send("E".repeat(100)));
        let first = read_exact(&mut device, WinkeyerProtocol::MAX_IN_FLIGHT).await;
        assert_eq!(first.len(), WinkeyerProtocol::MAX_IN_FLIGHT);
        // Nic więcej, dopóki nic nie zostało nadane.
        let mut probe = [0u8; 1];
        assert!(
            tokio::time::timeout(Duration::from_millis(50), device.read(&mut probe))
                .await
                .is_err()
        );
        // XOFF, potem echo jednego znaku: wciąż wstrzymane.
        device.write_all(&[0xC5, b'E']).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(50), device.read(&mut probe))
                .await
                .is_err()
        );
        // Zdjęcie XOFF: host dosyła jeden bajt w miejsce nadanego.
        device.write_all(&[0xC4]).await.unwrap();
        assert_eq!(read_exact(&mut device, 1).await, b"E".to_vec());
    }

    #[tokio::test(start_paused = true)]
    async fn winkeyer_that_does_not_answer_is_reported() {
        let (host, mut device) = tokio::io::duplex(64);
        let handle = spawn_winkeyer(host, CwKeyerSettings::default());
        let mut ev = Events::new(&handle);
        read_exact(&mut device, 2).await;
        assert_eq!(ev.next().await, CwEvent::Error(CwKeyerError::NoAnswer));
        assert_eq!(ev.next().await, CwEvent::Closed);
    }

    /// Atrapa linii portu: zapisuje zmiany stanu z czasem od startu.
    #[derive(Clone, Default)]
    struct RecordingLines {
        log: Arc<Mutex<Vec<(Duration, &'static str, bool)>>>,
        start: Arc<Mutex<Option<tokio::time::Instant>>>,
    }

    impl RecordingLines {
        fn record(&self, line: &'static str, state: bool) {
            let now = tokio::time::Instant::now();
            let start = *self.start.lock().unwrap().get_or_insert(now);
            self.log.lock().unwrap().push((now - start, line, state));
        }
        fn entries(&self) -> Vec<(Duration, &'static str, bool)> {
            self.log.lock().unwrap().clone()
        }
    }

    impl KeyLines for RecordingLines {
        fn set_key(&mut self, down: bool) -> std::io::Result<()> {
            self.record("key", down);
            Ok(())
        }
        fn set_ptt(&mut self, on: bool) -> std::io::Result<()> {
            self.record("ptt", on);
            Ok(())
        }
    }

    fn key_downs(entries: &[(Duration, &'static str, bool)]) -> Vec<(u128, u128)> {
        let mut out = Vec::new();
        let mut down_at = None;
        for &(t, line, state) in entries {
            if line != "key" {
                continue;
            }
            match (state, down_at) {
                (true, None) => down_at = Some(t),
                (false, Some(d)) => {
                    out.push((d.as_millis(), t.saturating_sub(d).as_millis()));
                    down_at = None;
                }
                _ => {}
            }
        }
        out
    }

    #[tokio::test(start_paused = true)]
    async fn line_keying_uses_paris_timing_and_ptt_lead_and_tail() {
        let lines = RecordingLines::default();
        let settings = CwKeyerSettings {
            kind: CwKeyerKind::SerialDtr,
            wpm: 20, // kropka = 60 ms
            line_ptt: true,
            ptt_lead_ms: 50,
            ptt_tail_ms: 150,
            ..CwKeyerSettings::default()
        };
        let handle = spawn_line_keyer(lines.clone(), settings);
        let mut ev = Events::new(&handle);
        assert_eq!(ev.next().await, CwEvent::Ready(String::new()));
        handle.send(CwCommand::Send("A E".to_string()));
        let events = ev.until(&CwEvent::Idle).await;
        assert_eq!(
            events,
            vec![
                CwEvent::Sent(1),
                CwEvent::Sent(1),
                CwEvent::Sent(1),
                CwEvent::Idle
            ]
        );

        let entries = lines.entries();
        assert_eq!(entries.first(), Some(&(Duration::ZERO, "ptt", true)));
        // A = kropka (60) + przerwa (60) + kreska (180); po 7 jednostkach odstępu E.
        assert_eq!(key_downs(&entries), vec![(50, 60), (170, 180), (770, 60)]);
        let ptt_off = entries
            .iter()
            .find(|(_, line, state)| *line == "ptt" && !*state)
            .unwrap();
        assert_eq!(
            ptt_off.0.as_millis(),
            830 + 150,
            "PTT tail after the last element"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn line_keying_abort_releases_the_key_at_once() {
        let lines = RecordingLines::default();
        let handle = spawn_line_keyer(
            lines.clone(),
            CwKeyerSettings {
                kind: CwKeyerKind::SerialRts,
                wpm: 12,
                ..CwKeyerSettings::default()
            },
        );
        let mut ev = Events::new(&handle);
        ev.next().await;
        handle.send(CwCommand::Send("TTTTTTTT".to_string()));
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.send(CwCommand::Abort);
        let events = ev.until(&CwEvent::Idle).await;
        assert!(events.len() <= 2, "{events:?}");
        let entries = lines.entries();
        let (t, line, state) = *entries.last().unwrap();
        assert_eq!((line, state), ("key", false));
        assert!(t.as_millis() <= 200, "released {t:?} after start");
    }

    #[tokio::test(start_paused = true)]
    async fn closing_the_line_keyer_always_releases_key_and_ptt() {
        let lines = RecordingLines::default();
        let handle = spawn_line_keyer(
            lines.clone(),
            CwKeyerSettings {
                line_ptt: true,
                ..CwKeyerSettings::default()
            },
        );
        let mut ev = Events::new(&handle);
        ev.next().await;
        handle.send(CwCommand::Send("TEST".to_string()));
        tokio::time::sleep(Duration::from_millis(120)).await;
        drop(ev);
        drop(handle);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let entries = lines.entries();
        let last_key = entries.iter().rev().find(|e| e.1 == "key").unwrap();
        let last_ptt = entries.iter().rev().find(|e| e.1 == "ptt").unwrap();
        assert!(!last_key.2 && !last_ptt.2, "{entries:?}");
    }

    async fn fake_rigctld(replies: Vec<&'static str>) -> (u16, Arc<Mutex<Vec<String>>>) {
        use tokio::io::AsyncBufReadExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        tokio::spawn(async move {
            for reply in replies {
                let (stream, _) = listener.accept().await.unwrap();
                let (reader, mut writer) = stream.into_split();
                let mut line = String::new();
                tokio::io::BufReader::new(reader)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                log.lock().unwrap().push(line);
                writer.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        (port, seen)
    }

    #[test]
    fn hamlib_text_keeps_only_morse_characters() {
        assert_eq!(hamlib_text("cq  de sp6ina <SK>\t#%"), "CQ DE SP6INA SK");
    }

    #[tokio::test]
    async fn hamlib_keyer_sets_speed_sends_morse_and_reports_rejections() {
        let (port, seen) = fake_rigctld(vec!["RPRT 0\n", "RPRT 0\n", "RPRT -11\n"]).await;
        let handle = spawn_hamlib("127.0.0.1".to_string(), port, 24);
        let mut ev = Events::new(&handle);
        assert!(matches!(ev.next().await, CwEvent::Ready(_)));
        handle.send(CwCommand::Send("cq test".to_string()));
        assert_eq!(ev.next().await, CwEvent::Sent(7));
        assert_eq!(ev.next().await, CwEvent::Idle);
        handle.send(CwCommand::Send("tu".to_string()));
        assert_eq!(ev.next().await, CwEvent::Error(CwKeyerError::Rejected(-11)));
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["L KEYSPD 24\n", "b CQ TEST\n", "b TU\n"]
        );
    }
}
