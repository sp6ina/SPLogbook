// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::hamlib::RigState;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::watch;

/// Zdarzenia nadsyłane przez klientów TCP (np. WSJT-X, JTDX, FLDigi)
#[derive(Debug, Clone, PartialEq)]
pub enum RigServerCommand {
    SetFrequency(u64),
    SetSplitFrequency(u64),
    SetMode(String),
    SetPtt(bool),
    SetVfo(String),
    SetSplit { enabled: bool, tx_vfo: String },
    SetRit(i32),
    SetXit(i32),
    SetPower(f32),
}

/// Maksymalna długość pojedynczej linii polecenia protokołu Hamlib (bajty).
/// Chroni przed klientami wysyłającymi dowolnie długie wiersze (DoS / przepełnienia bufora).
pub const MAX_LINE_LEN: usize = 256;

/// Kody błędów protokołu Hamlib (`RPRT <kod>`), zgodne z `rig_errcode_e` z biblioteki Hamlib.
pub mod rprt {
    /// RIG_OK — operacja zakończona sukcesem.
    pub const OK: i32 = 0;
    /// RIG_EINVAL — nieprawidłowy argument.
    pub const EINVAL: i32 = -1;
    /// RIG_ENIMPL — funkcja rozpoznana, ale niezaimplementowana.
    pub const ENIMPL: i32 = -4;
    /// RIG_EPROTO — błąd protokołu (nierozpoznane polecenie).
    pub const EPROTO: i32 = -9;
}

/// Pojedyncza mutacja stanu transceivera, wyznaczona przez polecenie.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum StateMutation {
    SetFrequency(u64),
    SetSplitFrequency(u64),
    SetMode(String),
    SetPtt(bool),
    SetVfo(String),
    SetSplit { enabled: bool, tx_vfo: String },
    SetRit(i32),
    SetXit(i32),
    SetPower(f32),
}

/// Wynik interpretacji pojedynczej linii polecenia protokołu Hamlib.
#[derive(Debug, Clone, PartialEq)]
struct Dispatch {
    /// Pełna odpowiedź tekstowa zapisywana do klienta (zakończona `\n`).
    response: String,
    /// Mutacje do zastosowania na współdzielonym stanie `RigState`.
    mutations: Vec<StateMutation>,
    /// Zdarzenia do rozesłania do warstwy GUI (np. rzeczywiste przestrojenie radia).
    commands: Vec<RigServerCommand>,
    /// Czy zamknąć połączenie.
    close: bool,
}

impl Dispatch {
    fn ok(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            mutations: Vec::new(),
            commands: Vec::new(),
            close: false,
        }
    }

    fn rprt(code: i32) -> Self {
        Self::ok(format!("RPRT {code}\n"))
    }
}

/// Interpretuje pojedynczą (przyciętą) linię polecenia Hamlib i zwraca gotową odpowiedź
/// oraz mutacje/zdarzenia do zastosowania. Czysta funkcja — łatwo testowalna bez sieci.
fn dispatch_command(trimmed: &str, state: &RigState) -> Dispatch {
    // Linie dłuższe niż MAX_LINE_LEN są odrzucane jeszcze przed wywołaniem tej funkcji.
    if trimmed.is_empty() {
        return Dispatch::rprt(rprt::EINVAL);
    }

    // Rozdzielenie polecenia na operator i argumenty (z zachowaniem wielkości liter operatora).
    let mut parts = trimmed.split_whitespace();
    let op = parts.next().unwrap_or_default();
    let args: Vec<&str> = parts.collect();

    match op {
        "q" | "Q" => Dispatch {
            response: String::new(),
            mutations: Vec::new(),
            commands: Vec::new(),
            close: true,
        },

        // ── Zapytania o stan ────────────────────────────────────────────────
        "\\dump_state" => Dispatch::ok(dump_state(state)),
        "\\chk_vfo" => Dispatch::ok("CHKVFO 0\n".to_string()),
        "\\get_powerstat" => Dispatch::ok(format!("{}\n", i32::from(state.connected))),
        "\\get_vfo" => Dispatch::ok(format!("{}\n", state.vfo)),
        "\\get_split" => Dispatch::ok(format!("{}\n", i32::from(state.split_enabled))),
        "\\get_split_vfo" => Dispatch::ok(format!("{}\n", state.tx_vfo)),
        "\\get_split_freq" => Dispatch::ok(format!(
            "{}\n",
            state.tx_frequency_hz.unwrap_or(state.frequency_hz)
        )),
        "\\get_rit" => Dispatch::ok(format!("{}\n", state.rit_hz)),
        "\\get_xit" => Dispatch::ok(format!("{}\n", state.xit_hz)),
        "\\get_info" => Dispatch::ok("Rig command set (SPLogbook Hamlib proxy) 1.0\n".to_string()),
        "\\get_level" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].to_ascii_uppercase().as_str() {
                "RFPOWER" => Dispatch::ok(format!("{}\n", level_power(state))),
                "STRENGTH" => Dispatch::ok(format!("{}\n", state.s_meter_dbm)),
                _ => Dispatch::rprt(rprt::ENIMPL),
            }
        }

        // ── Krótkie zapytania (zgodne z rigctld) ────────────────────────────
        "f" if args.is_empty() => Dispatch::ok(format!("{}\n", state.frequency_hz)),
        "m" if args.is_empty() => Dispatch::ok(format!(
            "{}\n{}\n",
            hamlib_mode(&state.mode),
            state.passband_hz
        )),
        "v" if args.is_empty() => Dispatch::ok(format!("{}\n", state.vfo)),
        "s" if args.is_empty() => {
            Dispatch::ok(format!("{}\nVFOA\n", i32::from(state.split_enabled)))
        }
        "t" if args.is_empty() => Dispatch::ok(format!("{}\n", i32::from(state.ptt))),
        "j" if args.is_empty() => Dispatch::ok(format!("{}\n", state.rit_hz)),
        "z" if args.is_empty() => Dispatch::ok(format!("{}\n", state.xit_hz)),
        "l" if args.is_empty() => Dispatch::ok(level_list()),
        "l" => match args[0].to_ascii_uppercase().as_str() {
            "RFPOWER" => Dispatch::ok(format!("{}\n", level_power(state))),
            "STRENGTH" => Dispatch::ok(format!("{}\n", state.s_meter_dbm)),
            _ => Dispatch::rprt(rprt::ENIMPL),
        },

        // ── Ustawianie częstotliwości / emisji / PTT ────────────────────────
        "F" | "f" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].parse::<u64>() {
                Ok(freq) => {
                    let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                    d.mutations.push(StateMutation::SetFrequency(freq));
                    d.commands.push(RigServerCommand::SetFrequency(freq));
                    d
                }
                Err(_) => Dispatch::rprt(rprt::EINVAL),
            }
        }
        "M" | "m" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let new_mode = args[0].to_string();
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetMode(new_mode.clone()));
            d.commands.push(RigServerCommand::SetMode(new_mode));
            d
        }
        "T" | "t" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let ptt = args[0] == "1";
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetPtt(ptt));
            d.commands.push(RigServerCommand::SetPtt(ptt));
            d
        }

        // ── VFO / Split ─────────────────────────────────────────────────────
        "V" | "v" | "\\set_vfo" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let vfo = args[0].to_string();
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetVfo(vfo.clone()));
            d.commands.push(RigServerCommand::SetVfo(vfo));
            d
        }
        "S" | "s" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let enabled = args[0] == "1";
            let tx_vfo = args
                .get(1)
                .map_or_else(|| "VFOA".to_string(), std::string::ToString::to_string);
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetSplit {
                enabled,
                tx_vfo: tx_vfo.clone(),
            });
            d.commands
                .push(RigServerCommand::SetSplit { enabled, tx_vfo });
            d
        }

        // ── RIT / XIT ───────────────────────────────────────────────────────
        "J" | "j" | "\\set_rit" => {
            set_rit_xit(&args, StateMutation::SetRit, RigServerCommand::SetRit)
        }
        "Z" | "z" | "\\set_xit" => {
            set_rit_xit(&args, StateMutation::SetXit, RigServerCommand::SetXit)
        }

        // ── Rozszerzone polecenia z odwrotnym ukośnikiem ────────────────────
        "\\set_split" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let enabled = args[0] == "1";
            let tx_vfo = args
                .get(1)
                .map_or_else(|| "VFOA".to_string(), std::string::ToString::to_string);
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetSplit {
                enabled,
                tx_vfo: tx_vfo.clone(),
            });
            d.commands
                .push(RigServerCommand::SetSplit { enabled, tx_vfo });
            d
        }
        "\\set_split_vfo" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            // Aktualizujemy nazwę VFO nadawczego (bez zmiany stanu split).
            let vfo = args[0].to_string();
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(StateMutation::SetSplit {
                enabled: state.split_enabled,
                tx_vfo: vfo.clone(),
            });
            d.commands.push(RigServerCommand::SetSplit {
                enabled: state.split_enabled,
                tx_vfo: vfo,
            });
            d
        }
        "\\set_split_freq" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].parse::<u64>() {
                Ok(freq) => {
                    let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                    d.mutations.push(StateMutation::SetSplitFrequency(freq));
                    d.commands.push(RigServerCommand::SetSplitFrequency(freq));
                    d
                }
                Err(_) => Dispatch::rprt(rprt::EINVAL),
            }
        }
        "\\set_level" => {
            if args.len() < 2 {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].to_ascii_uppercase().as_str() {
                "RFPOWER" => match args[1].parse::<f32>() {
                    Ok(norm) => {
                        let watts = norm.clamp(0.0, 1.0) * 100.0;
                        let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                        d.mutations.push(StateMutation::SetPower(watts));
                        d.commands.push(RigServerCommand::SetPower(watts));
                        d
                    }
                    Err(_) => Dispatch::rprt(rprt::EINVAL),
                },
                _ => Dispatch::rprt(rprt::ENIMPL),
            }
        }
        "\\set_powerstat" => {
            // Załączenie/wyłączenie zasilania radia — poza zakresem proxy (bezpieczny no-op).
            Dispatch::rprt(rprt::ENIMPL)
        }
        "\\set_func" | "\\get_func" => Dispatch::rprt(rprt::ENIMPL),

        // ── Nierozpoznane polecenie ─────────────────────────────────────────
        _ => Dispatch::rprt(rprt::EPROTO),
    }
}

/// Buduje wspólną odpowiedź dla ustawień RIT/XIT (wartości i32 w Hz).
fn set_rit_xit<M, C>(args: &[&str], mutation: M, command: C) -> Dispatch
where
    M: Fn(i32) -> StateMutation,
    C: Fn(i32) -> RigServerCommand,
{
    if args.is_empty() {
        return Dispatch::rprt(rprt::EINVAL);
    }
    match args[0].parse::<i32>() {
        Ok(v) => {
            let mut d = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            d.mutations.push(mutation(v));
            d.commands.push(command(v));
            d
        }
        Err(_) => Dispatch::rprt(rprt::EINVAL),
    }
}

/// Wartość poziomu mocy w formacie Hamlib (znormalizowana 0.0–1.0).
fn level_power(state: &RigState) -> String {
    let norm = (state.rf_power_watts / 100.0).clamp(0.0, 1.0);
    format!("{norm:.4}")
}

/// Lista dostępnych poziomów (skrócona, zgodna z rigctld `l`).
fn level_list() -> String {
    "RFPOWER STRENGTH\n".to_string()
}

/// Mapuje emisję wewnętrzną SPLogbook na nazwę emisji Hamlib.
fn hamlib_mode(mode: &str) -> &'static str {
    match mode.to_uppercase().as_str() {
        "CW" => "CW",
        "LSB" => "LSB",
        "USB" | "FT8" | "FT4" | "JS8" => "PKTUSB",
        "AM" => "AM",
        "FM" => "FM",
        "RTTY" => "RTTY",
        _ => "USB",
    }
}

/// Generuje minimalną, ale poprawną odpowiedź `\dump_state` zgodną z rigctld.
fn dump_state(state: &RigState) -> String {
    let _ = state;
    "0\n2\n2\n100000 30000000 0xef -1 -1 0x3 0x3\n0 0 0 0 0 0 0\n0 0 0 0 0 0 0\n0xef 1\n0 0\n0xef 1\n0 0\n0 0\n0 0\n0\n0\n0\n0\n0\n".to_string()
}

/// Serwer Hamlib rigctld proxy — udostępnia połączenie CAT dla WSJT-X, JTDX, FLDigi na porcie TCP (domyślnie 4534)
pub struct HamlibProxyServer {
    pub port: u16,
    pub shared_state: Arc<RwLock<RigState>>,
    pub cmd_sender: broadcast::Sender<RigServerCommand>,
    stop_tx: watch::Sender<bool>,
}

impl HamlibProxyServer {
    pub fn new(
        port: u16,
        shared_state: Arc<RwLock<RigState>>,
    ) -> (Self, broadcast::Receiver<RigServerCommand>) {
        let (cmd_sender, cmd_receiver) = broadcast::channel(64);
        let (stop_tx, _stop_rx) = watch::channel(false);
        (
            Self {
                port,
                shared_state,
                cmd_sender,
                stop_tx,
            },
            cmd_receiver,
        )
    }

    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let mut stop_rx = self.stop_tx.subscribe();

        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, _peer)) => {
                            let state = self.shared_state.clone();
                            let cmd_tx = self.cmd_sender.clone();
                            let client_stop_rx = self.stop_tx.subscribe();
                            tokio::spawn(async move {
                                let _ = handle_client(stream, state, cmd_tx, client_stop_rx).await;
                            });
                        }
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

async fn handle_client(
    mut stream: tokio::net::TcpStream,
    state: Arc<RwLock<RigState>>,
    cmd_tx: broadcast::Sender<RigServerCommand>,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        // Adapter ograniczający długość linii; wiążemy go w zmiennej, by uniknąć
        // E0716 (temporary dropped while borrowed) wewnątrz `tokio::select!`.
        let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
        tokio::select! {
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
            read_res = limited.read_line(&mut line) => {
                match read_res {
                    Ok(0) | Err(_) => break, // EOF / Rozłączono / błąd odczytu
                    Ok(_) => {
                        // Ochrona przed nadmiernie długimi liniami (DoS / przepełnienie bufora).
                        if line.len() > MAX_LINE_LEN {
                            writer.write_all(format!("RPRT {}\n", rprt::EPROTO).as_bytes()).await?;
                            writer.flush().await?;
                            break;
                        }

                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Interpretacja polecenia (czysta, testowalna funkcja).
                        let snapshot = state.read().map(|s| s.clone()).unwrap_or_default();
                        let dispatch = dispatch_command(trimmed, &snapshot);

                        // Zastosowanie mutacji stanu pod pojedynczą blokadą zapisu.
                        if !dispatch.mutations.is_empty() {
                            if let Ok(mut s) = state.write() {
                                for m in &dispatch.mutations {
                                    apply_mutation(&mut s, m);
                                }
                            }
                        }

                        // Rozesłanie zdarzeń do warstwy GUI (rzeczywiste przestrojenie radia).
                        for cmd in &dispatch.commands {
                            let _ = cmd_tx.send(cmd.clone());
                        }

                        // Zapisanie odpowiedzi.
                        if !dispatch.response.is_empty() {
                            writer.write_all(dispatch.response.as_bytes()).await?;
                            writer.flush().await?;
                        }

                        if dispatch.close {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Aplikuje pojedynczą mutację do stanu transceivera.
fn apply_mutation(state: &mut RigState, m: &StateMutation) {
    match *m {
        StateMutation::SetFrequency(freq) => state.frequency_hz = freq,
        StateMutation::SetSplitFrequency(freq) => state.tx_frequency_hz = Some(freq),
        StateMutation::SetMode(ref mode) => state.mode.clone_from(mode),
        StateMutation::SetPtt(ptt) => state.ptt = ptt,
        StateMutation::SetVfo(ref vfo) => state.vfo.clone_from(vfo),
        StateMutation::SetSplit {
            enabled,
            ref tx_vfo,
        } => {
            state.split_enabled = enabled;
            state.tx_vfo.clone_from(tx_vfo);
        }
        StateMutation::SetRit(rit) => state.rit_hz = rit,
        StateMutation::SetXit(xit) => state.xit_hz = xit,
        StateMutation::SetPower(watts) => state.rf_power_watts = watts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> RigState {
        RigState {
            frequency_hz: 14_074_000,
            mode: "USB".to_string(),
            ..RigState::default()
        }
    }

    #[test]
    fn getters_return_current_state() {
        let s = test_state();
        assert_eq!(dispatch_command("f", &s).response, "14074000\n");
        assert_eq!(dispatch_command("v", &s).response, "VFOA\n");
        assert_eq!(dispatch_command("t", &s).response, "0\n");
        assert_eq!(dispatch_command("j", &s).response, "0\n");
        assert_eq!(dispatch_command("z", &s).response, "0\n");
        assert_eq!(dispatch_command("\\get_rit", &s).response, "0\n");
        assert_eq!(dispatch_command("\\get_xit", &s).response, "0\n");
        assert_eq!(dispatch_command("\\get_vfo", &s).response, "VFOA\n");
        assert_eq!(dispatch_command("\\get_split", &s).response, "0\n");
    }

    #[test]
    fn setters_emit_ok_and_mutate() {
        let s = test_state();

        let d = dispatch_command("F 14200000", &s);
        assert_eq!(d.response, "RPRT 0\n");
        assert_eq!(d.commands, vec![RigServerCommand::SetFrequency(14_200_000)]);
        assert_eq!(d.mutations, vec![StateMutation::SetFrequency(14_200_000)]);

        let d = dispatch_command("M CW", &s);
        assert_eq!(
            d.commands,
            vec![RigServerCommand::SetMode("CW".to_string())]
        );

        let d = dispatch_command("T 1", &s);
        assert_eq!(d.commands, vec![RigServerCommand::SetPtt(true)]);

        let d = dispatch_command("V VFOB", &s);
        assert_eq!(
            d.commands,
            vec![RigServerCommand::SetVfo("VFOB".to_string())]
        );

        let d = dispatch_command("S 1 VFOB", &s);
        assert_eq!(
            d.commands,
            vec![RigServerCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string()
            }]
        );

        let d = dispatch_command("J 250", &s);
        assert_eq!(d.commands, vec![RigServerCommand::SetRit(250)]);

        let d = dispatch_command("Z -300", &s);
        assert_eq!(d.commands, vec![RigServerCommand::SetXit(-300)]);
    }

    #[test]
    fn invalid_arguments_return_einval() {
        let s = test_state();
        assert_eq!(dispatch_command("F", &s).response, "RPRT -1\n");
        assert_eq!(dispatch_command("F abc", &s).response, "RPRT -1\n");
        assert_eq!(dispatch_command("J", &s).response, "RPRT -1\n");
        assert_eq!(dispatch_command("J notanum", &s).response, "RPRT -1\n");
        assert_eq!(dispatch_command("M", &s).response, "RPRT -1\n");
        assert_eq!(dispatch_command("S", &s).response, "RPRT -1\n");
        assert_eq!(
            dispatch_command("\\set_level RFPOWER", &s).response,
            "RPRT -1\n"
        );
    }

    #[test]
    fn unrecognized_command_returns_eproto() {
        let s = test_state();
        assert_eq!(dispatch_command("X 123", &s).response, "RPRT -9\n");
        assert_eq!(dispatch_command("bogus", &s).response, "RPRT -9\n");
    }

    #[test]
    fn empty_command_returns_einval() {
        let s = test_state();
        assert_eq!(dispatch_command("", &s).response, "RPRT -1\n");
    }

    #[test]
    fn quit_closes_connection() {
        let s = test_state();
        assert!(dispatch_command("q", &s).close);
        assert!(dispatch_command("Q", &s).close);
    }

    #[test]
    fn split_getter_and_setter() {
        let s = test_state();
        assert_eq!(dispatch_command("s", &s).response, "0\nVFOA\n");

        let mut split_state = test_state();
        split_state.split_enabled = true;
        assert_eq!(dispatch_command("s", &split_state).response, "1\nVFOA\n");

        // Setter z domyślnym VFO nadawczym.
        let d = dispatch_command("S 1", &s);
        assert_eq!(
            d.commands,
            vec![RigServerCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOA".to_string()
            }]
        );
    }

    #[test]
    fn apply_mutation_updates_state() {
        let mut s = test_state();
        apply_mutation(&mut s, &StateMutation::SetFrequency(7_100_000));
        apply_mutation(&mut s, &StateMutation::SetMode("LSB".to_string()));
        apply_mutation(&mut s, &StateMutation::SetPtt(true));
        apply_mutation(&mut s, &StateMutation::SetVfo("VFOB".to_string()));
        apply_mutation(
            &mut s,
            &StateMutation::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string(),
            },
        );
        apply_mutation(&mut s, &StateMutation::SetRit(500));
        apply_mutation(&mut s, &StateMutation::SetXit(-100));
        apply_mutation(&mut s, &StateMutation::SetPower(50.0));

        assert_eq!(s.frequency_hz, 7_100_000);
        assert_eq!(s.mode, "LSB");
        assert!(s.ptt);
        assert_eq!(s.vfo, "VFOB");
        assert!(s.split_enabled);
        assert_eq!(s.rit_hz, 500);
        assert_eq!(s.xit_hz, -100);
        assert_eq!(s.rf_power_watts, 50.0);
    }
}
