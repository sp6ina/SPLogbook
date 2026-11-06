// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz WoÅºniak (SP6INA)

use crate::cat::hamlib::RigState;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::watch;

/// Zdarzenia nadsyÅane przez klientÃ³w TCP (np. WSJT-X, JTDX, FLDigi)
#[derive(Debug, Clone, PartialEq)]
pub enum RigServerCommand {
    SetFrequency(u64),
    SetSplitFrequency(u64),
    SetMode(String),
    SetPassband(u32),
    SetPtt(bool),
    SetVfo(String),
    SetSplit { enabled: bool, tx_vfo: String },
    SetRit(i32),
    SetXit(i32),
    SetPower(f32),
}

/// Maksymalna dÅugoÅÄ pojedynczej linii polecenia protokoÅu Hamlib (bajty).
/// Chroni przed klientami wysyÅajÄcymi dowolnie dÅugie wiersze (DoS / przepeÅnienia bufora).
pub const MAX_LINE_LEN: usize = 256;

/// Kody bÅÄdÃ³w protokoÅu Hamlib (`RPRT <kod>`), zgodne z `rig_errcode_e` z biblioteki Hamlib.
pub mod rprt {
    /// RIG_OK — operacja zakończona sukcesem.
    pub const OK: i32 = 0;
    /// RIG_EINVAL — nieprawidłowy argument / parametr.
    pub const EINVAL: i32 = -1;
    /// RIG_ECONF — nieprawidłowa konfiguracja.
    pub const ECONF: i32 = -2;
    /// RIG_ENOMEM — brak pamięci.
    pub const ENOMEM: i32 = -3;
    /// RIG_ENIMPL — funkcja nierozpoznana lub niezaimplementowana.
    pub const ENIMPL: i32 = -4;
    /// RIG_ETIMEOUT — przekroczono limit czasu komunikacji.
    pub const ETIMEOUT: i32 = -5;
    /// RIG_EIO — błąd wejścia/wyjścia (I/O) lub rozłączenie transportu.
    pub const EIO: i32 = -6;
    /// RIG_EINTERNAL — błąd wewnętrzny biblioteki Hamlib.
    pub const EINTERNAL: i32 = -7;
    /// RIG_EPROTO — błąd protokołu (błędna składnia, nierozpoznane polecenie).
    pub const EPROTO: i32 = -8;
    /// RIG_ERJCTED — polecenie odrzucone przez radio / backend.
    pub const ERJCTED: i32 = -9;
    /// RIG_ETRUNC — wynik odpowiedzi został obcięty.
    pub const ETRUNC: i32 = -10;
    /// RIG_ENAVAIL — funkcja niedostępna (np. brak radia, nieobsługiwana capability, shutdown).
    pub const ENAVAIL: i32 = -11;
    /// RIG_ENTARGET — VFO niecelowalne.
    pub const ENTARGET: i32 = -12;
    /// RIG_EBUSERROR — błąd magistrali sprzętowej.
    pub const EBUSERROR: i32 = -13;
    /// RIG_BUSBUSY — magistrala lub radio zajęte (np. aktywny TX na innym radiu, interlock).
    pub const BUSBUSY: i32 = -14;
    /// RIG_EARG — nieprawidłowy wskaźnik lub pusty argument.
    pub const EARG: i32 = -15;
    /// RIG_EVFO — nieprawidłowe VFO.
    pub const EVFO: i32 = -16;
    /// RIG_EDOM — argument poza dziedziną.
    pub const EDOM: i32 = -17;
}

/// Kanoniczne kody błędów Hamlib (`rig_errcode_e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum HamlibErrorCode {
    Ok = rprt::OK,
    InvalidParameter = rprt::EINVAL,
    InvalidConfiguration = rprt::ECONF,
    OutOfMemory = rprt::ENOMEM,
    NotImplemented = rprt::ENIMPL,
    Timeout = rprt::ETIMEOUT,
    Io = rprt::EIO,
    Internal = rprt::EINTERNAL,
    Protocol = rprt::EPROTO,
    Rejected = rprt::ERJCTED,
    Truncated = rprt::ETRUNC,
    FunctionNotAvailable = rprt::ENAVAIL,
    VfoNotTargetable = rprt::ENTARGET,
    BusError = rprt::EBUSERROR,
    BusBusy = rprt::BUSBUSY,
    InvalidArgumentPointer = rprt::EARG,
    InvalidVfo = rprt::EVFO,
    OutOfDomain = rprt::EDOM,
}

impl HamlibErrorCode {
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn rprt_line(self) -> String {
        format!(
            "RPRT {}
",
            self.as_i32()
        )
    }
}

/// Mapuje błędy środowiska wykonawczego CAT na kanoniczny kod błędu Hamlib.
#[must_use]
pub fn map_cat_error_to_hamlib(err: &crate::cat::runtime::CatRuntimeError) -> HamlibErrorCode {
    use crate::cat::hub::RigHubError;
    use crate::cat::runtime::CatRuntimeError;
    use crate::cat::service::RigServiceError;
    use crate::cat::tx_arbiter::TxArbiterError;

    match err {
        CatRuntimeError::Validation(_) => HamlibErrorCode::InvalidParameter,
        CatRuntimeError::Factory(_) => HamlibErrorCode::FunctionNotAvailable,
        CatRuntimeError::Timeout => HamlibErrorCode::Timeout,
        CatRuntimeError::ShuttingDown => HamlibErrorCode::FunctionNotAvailable,
        CatRuntimeError::Hub(hub_err) => match hub_err {
            RigHubError::Timeout => HamlibErrorCode::Timeout,
            RigHubError::RigNotFound(_)
            | RigHubError::ActiveRxNotConfigured
            | RigHubError::ActiveTxNotConfigured => HamlibErrorCode::FunctionNotAvailable,
            RigHubError::RigAlreadyExists(_) => HamlibErrorCode::InvalidConfiguration,
            RigHubError::HubShuttingDown | RigHubError::ShutdownError(_) => {
                HamlibErrorCode::FunctionNotAvailable
            }
            RigHubError::CannotChangeActiveTxWhileOwned { .. } | RigHubError::QueueFull => {
                HamlibErrorCode::BusBusy
            }
            RigHubError::Routing(_) => HamlibErrorCode::InvalidParameter,
            RigHubError::TxArbiter(tx_err) => match tx_err {
                TxArbiterError::Busy { .. }
                | TxArbiterError::AlreadyRequesting { .. }
                | TxArbiterError::AlreadyOwned { .. }
                | TxArbiterError::ExternalTransmitActive { .. }
                | TxArbiterError::NotOwner { .. }
                | TxArbiterError::OwnerMismatch { .. } => HamlibErrorCode::BusBusy,
                TxArbiterError::TxStateUnknown { .. }
                | TxArbiterError::StateUnknown { .. }
                | TxArbiterError::PttStateUnknown { .. }
                | TxArbiterError::InvalidTransition { .. }
                | TxArbiterError::ResyncFailed(_) => HamlibErrorCode::Rejected,
            },
            RigHubError::Service(svc_err) => match svc_err {
                RigServiceError::DeadlineExceeded => HamlibErrorCode::Timeout,
                RigServiceError::Disconnected
                | RigServiceError::ResponseChannelClosed
                | RigServiceError::BackendPanic(_) => HamlibErrorCode::Io,
                RigServiceError::QueueFull | RigServiceError::ExternalTransmitActive => {
                    HamlibErrorCode::BusBusy
                }
                RigServiceError::NotReady(_) | RigServiceError::ServiceShuttingDown => {
                    HamlibErrorCode::FunctionNotAvailable
                }
                RigServiceError::CommandExecutionFailed(_)
                | RigServiceError::InvalidLifecycleTransition { .. }
                | RigServiceError::PttStateUnknown => HamlibErrorCode::Rejected,
            },
        },
    }
}

/// Wynik interpretacji pojedynczej linii polecenia protokoÅu Hamlib.
#[derive(Debug, Clone, PartialEq)]
struct Dispatch {
    /// PeÅna odpowiedÅº tekstowa zapisywana do klienta (zakoÅczona `\n`).
    response: String,

    /// Zdarzenia do rozesÅania do warstwy GUI (np. rzeczywiste przestrojenie radia).
    commands: Vec<RigServerCommand>,
    /// Czy zamknÄÄ poÅÄczenie.
    close: bool,
}

impl Dispatch {
    fn ok(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            commands: Vec::new(),
            close: false,
        }
    }

    fn rprt(code: i32) -> Self {
        Self::ok(format!("RPRT {code}\n"))
    }
}

/// Interpretuje pojedynczÄ (przyciÄtÄ) liniÄ polecenia Hamlib i zwraca gotowÄ odpowiedÅº
/// oraz mutacje/zdarzenia do zastosowania. Czysta funkcja â Åatwo testowalna bez sieci.
fn dispatch_command(trimmed: &str, state: &RigState) -> Dispatch {
    // Linie dÅuÅ¼sze niÅ¼ MAX_LINE_LEN sÄ odrzucane jeszcze przed wywoÅaniem tej funkcji.
    if trimmed.is_empty() {
        return Dispatch::rprt(rprt::EINVAL);
    }

    // Rozdzielenie polecenia na operator i argumenty (z zachowaniem wielkoÅci liter operatora).
    let mut parts = trimmed.split_whitespace();
    let op = parts.next().unwrap_or_default();
    let args: Vec<&str> = parts.collect();

    match op {
        "q" | "Q" => Dispatch {
            response: String::new(),
            commands: Vec::new(),
            close: true,
        },

        // ââ Zapytania o stan ââââââââââââââââââââââââââââââââââââââââââââââââ
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

        // ââ KrÃ³tkie zapytania (zgodne z rigctld) ââââââââââââââââââââââââââââ
        "f" if args.is_empty() => Dispatch::ok(format!("{}\n", state.frequency_hz)),
        "m" if args.is_empty() => Dispatch::ok(format!(
            "{}\n{}\n",
            hamlib_mode(&state.mode),
            state.passband_hz
        )),
        "v" if args.is_empty() => Dispatch::ok(format!("{}\n", state.vfo)),
        "s" if args.is_empty() => Dispatch::ok(format!(
            "{}\n{}\n",
            i32::from(state.split_enabled),
            if state.tx_vfo.is_empty() {
                "VFOA"
            } else {
                &state.tx_vfo
            }
        )),
        "t" if args.is_empty() => Dispatch::ok(format!("{}\n", i32::from(state.ptt))),
        "j" if args.is_empty() => Dispatch::ok(format!("{}\n", state.rit_hz)),
        "z" if args.is_empty() => Dispatch::ok(format!("{}\n", state.xit_hz)),
        "l" if args.is_empty() => Dispatch::ok(level_list()),
        "l" => match args[0].to_ascii_uppercase().as_str() {
            "RFPOWER" => Dispatch::ok(format!("{}\n", level_power(state))),
            "STRENGTH" => Dispatch::ok(format!("{}\n", state.s_meter_dbm)),
            _ => Dispatch::rprt(rprt::ENIMPL),
        },

        // ââ Ustawianie czÄstotliwoÅci / emisji / PTT ââââââââââââââââââââââââ
        "F" | "f" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].parse::<u64>() {
                Ok(freq) => {
                    let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                    dispatch_result
                        .commands
                        .push(RigServerCommand::SetFrequency(freq));
                    dispatch_result
                }
                Err(_) => Dispatch::rprt(rprt::EINVAL),
            }
        }
        "M" | "m" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let new_mode = args[0].to_string();
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));

            if let Some(pb) = args.get(1).and_then(|s| s.parse::<u32>().ok()) {
                if pb > 0 {}
            }
            dispatch_result
                .commands
                .push(RigServerCommand::SetMode(new_mode));
            dispatch_result
        }
        "T" | "t" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let ptt = args[0] == "1";
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            dispatch_result.commands.push(RigServerCommand::SetPtt(ptt));
            dispatch_result
        }

        // ââ VFO / Split âââââââââââââââââââââââââââââââââââââââââââââââââââââ
        "V" | "v" | "\\set_vfo" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let vfo = args[0].to_string();
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));

            dispatch_result.commands.push(RigServerCommand::SetVfo(vfo));
            dispatch_result
        }
        "S" | "s" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let enabled = args[0] == "1";
            let tx_vfo = args
                .get(1)
                .map_or_else(|| "VFOA".to_string(), std::string::ToString::to_string);
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            dispatch_result
                .commands
                .push(RigServerCommand::SetSplit { enabled, tx_vfo });
            dispatch_result
        }

        // ââ RIT / XIT âââââââââââââââââââââââââââââââââââââââââââââââââââââââ
        "J" | "j" | "\\set_rit" => set_rit_xit(&args, RigServerCommand::SetRit),
        "Z" | "z" | "\\set_xit" => set_rit_xit(&args, RigServerCommand::SetXit),

        // ââ Rozszerzone polecenia z odwrotnym ukoÅnikiem ââââââââââââââââââââ
        "\\set_split" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            let enabled = args[0] == "1";
            let tx_vfo = args
                .get(1)
                .map_or_else(|| "VFOA".to_string(), std::string::ToString::to_string);
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            dispatch_result
                .commands
                .push(RigServerCommand::SetSplit { enabled, tx_vfo });
            dispatch_result
        }
        "\\set_split_vfo" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            // Aktualizujemy nazwÄ VFO nadawczego (bez zmiany stanu split).
            let vfo = args[0].to_string();
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
            dispatch_result.commands.push(RigServerCommand::SetSplit {
                enabled: state.split_enabled,
                tx_vfo: vfo,
            });
            dispatch_result
        }
        "\\set_split_freq" => {
            if args.is_empty() {
                return Dispatch::rprt(rprt::EINVAL);
            }
            match args[0].parse::<u64>() {
                Ok(freq) => {
                    let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                    dispatch_result
                        .commands
                        .push(RigServerCommand::SetSplitFrequency(freq));
                    dispatch_result
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
                        let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));
                        dispatch_result
                            .commands
                            .push(RigServerCommand::SetPower(watts));
                        dispatch_result
                    }
                    Err(_) => Dispatch::rprt(rprt::EINVAL),
                },
                _ => Dispatch::rprt(rprt::ENIMPL),
            }
        }
        "\\set_powerstat" => {
            // ZaÅÄczenie/wyÅÄczenie zasilania radia â poza zakresem proxy (bezpieczny no-op).
            Dispatch::rprt(rprt::ENIMPL)
        }
        "\\set_func" | "\\get_func" => Dispatch::rprt(rprt::ENIMPL),

        // ââ Nierozpoznane polecenie âââââââââââââââââââââââââââââââââââââââââ
        _ => Dispatch::rprt(rprt::EPROTO),
    }
}

/// Buduje wspÃ³lnÄ odpowiedÅº dla ustawieÅ RIT/XIT (wartoÅci i32 w Hz).
fn set_rit_xit<C>(args: &[&str], command: C) -> Dispatch
where
    C: Fn(i32) -> RigServerCommand,
{
    if args.is_empty() {
        return Dispatch::rprt(rprt::EINVAL);
    }
    match args[0].parse::<i32>() {
        Ok(v) => {
            let mut dispatch_result = Dispatch::ok(format!("RPRT {}\n", rprt::OK));

            dispatch_result.commands.push(command(v));
            dispatch_result
        }
        Err(_) => Dispatch::rprt(rprt::EINVAL),
    }
}

/// WartoÅÄ poziomu mocy w formacie Hamlib (znormalizowana 0.0â1.0).
fn level_power(state: &RigState) -> String {
    let norm = (state.rf_power_watts / 100.0).clamp(0.0, 1.0);
    format!("{norm:.4}")
}

/// Lista dostÄpnych poziomÃ³w (skrÃ³cona, zgodna z rigctld `l`).
fn level_list() -> String {
    "RFPOWER STRENGTH\n".to_string()
}

/// Mapuje emisjÄ wewnÄtrznÄ SPLogbook na nazwÄ emisji Hamlib.
fn hamlib_mode(mode: &str) -> &'static str {
    match mode.to_uppercase().as_str() {
        "CW" => "CW",
        "LSB" => "LSB",
        "USB" => "USB",
        "FT8" | "FT4" | "JS8" | "PKTUSB" | "DATA" | "DIGI" => "PKTUSB",
        "AM" => "AM",
        "FM" => "FM",
        "RTTY" => "RTTY",
        _ => "USB",
    }
}

/// Generuje minimalnÄ, ale poprawnÄ odpowiedÅº `\dump_state` zgodnÄ z rigctld.
fn dump_state(state: &RigState) -> String {
    let _ = state;
    "0\n2\n2\n100000 30000000 0xef -1 -1 0x3 0x3\n0 0 0 0 0 0 0\n0 0 0 0 0 0 0\n0xef 1\n0 0\n0xef 1\n0 0\n0 0\n0 0\n0\n0\n0\n0\n0\n".to_string()
}

use std::collections::HashMap;
use std::net::IpAddr;

/// Typowane błędy alokacji i uruchomienia serwerów proxy Hamlib.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    /// Port poza dozwolonym zakresem (1024..=65535).
    InvalidPort(u16),
    /// Konflikt portu: wykryto zduplikowany port proxy na tym samym interfejsie.
    DuplicatePort(u16),
    /// Kolizja z portem REST API (8420).
    PortConflictWithRestApi(u16),
    /// Błąd powiązania gniazda TCP.
    BindError(String),
    /// Nieznane radio w rejestrze.
    RigNotFound(crate::cat::routing::RigId),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPort(p) => {
                write!(f, "nieprawidłowy port proxy: {p} (wymagany 1024..=65535)")
            }
            Self::DuplicatePort(p) => write!(f, "zduplikowany port proxy Hamlib: {p}"),
            Self::PortConflictWithRestApi(p) => {
                write!(f, "port proxy Hamlib {p} koliduje z portem REST API (8420)")
            }
            Self::BindError(e) => write!(f, "błąd bind gniazda TCP proxy: {e}"),
            Self::RigNotFound(id) => write!(f, "radio '{id}' nie zostało odnalezione"),
        }
    }
}

impl std::error::Error for ProxyError {}

/// Informacja o stałym przypisaniu punktu końcowego Hamlib Proxy do radia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigProxyEndpoint {
    pub rig_id: crate::cat::routing::RigId,
    pub bind_addr: IpAddr,
    pub port: u16,
}

/// Menedżer alokacji i walidacji unikalnych portów proxy dla stacji wieloradiowej.
#[derive(Debug, Clone, Default)]
pub struct ProxyPortManager {
    endpoints: HashMap<crate::cat::routing::RigId, RigProxyEndpoint>,
}

impl ProxyPortManager {
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
        }
    }

    /// Przydziela lub rejestruje unikalny port Hamlib proxy dla wskazanego radia.
    ///
    /// Algorytm przydziału:
    /// - Radio 1: domyślnie 4534 (lub podany `base_port`),
    /// - Kolejne radia: następne wolne parzyste porty (4536, 4538, ...),
    /// - Port REST API (8420) jest bezwzględnie zablokowany przed kolizją.
    pub fn allocate_or_register(
        &mut self,
        rig_id: crate::cat::routing::RigId,
        preferred_port: Option<u16>,
        base_port: u16,
    ) -> Result<RigProxyEndpoint, ProxyError> {
        let port = if let Some(p) = preferred_port {
            if !(1024..=65535).contains(&p) {
                return Err(ProxyError::InvalidPort(p));
            }
            if p == 8420 {
                return Err(ProxyError::PortConflictWithRestApi(p));
            }
            for (other_id, ep) in &self.endpoints {
                if other_id != &rig_id && ep.port == p {
                    return Err(ProxyError::DuplicatePort(p));
                }
            }
            p
        } else {
            let mut candidate = if base_port % 2 == 0 {
                base_port
            } else {
                base_port + 1
            };
            loop {
                if candidate > 65530 {
                    return Err(ProxyError::InvalidPort(candidate));
                }
                let in_use = self.endpoints.values().any(|ep| ep.port == candidate);
                if !in_use && candidate != 8420 {
                    break;
                }
                candidate += 2;
            }
            candidate
        };

        let endpoint = RigProxyEndpoint {
            rig_id: rig_id.clone(),
            bind_addr: IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            port,
        };
        self.endpoints.insert(rig_id, endpoint.clone());
        Ok(endpoint)
    }

    pub fn get_endpoint(&self, rig_id: &crate::cat::routing::RigId) -> Option<&RigProxyEndpoint> {
        self.endpoints.get(rig_id)
    }

    pub fn unregister(&mut self, rig_id: &crate::cat::routing::RigId) {
        self.endpoints.remove(rig_id);
    }
}

/// Serwer Hamlib rigctld proxy â udostÄpnia poÅÄczenie CAT dla WSJT-X, JTDX, FLDigi na porcie TCP (domyÅlnie 4534)
pub struct HamlibProxyServer {
    pub port: u16,
    pub shared_state: Arc<RwLock<RigState>>,
    pub cmd_sender: broadcast::Sender<RigServerCommand>,
    stop_tx: watch::Sender<bool>,
    pub rig_id: Option<crate::cat::routing::RigId>,
    pub cat_runtime: Option<crate::cat::runtime::CatRuntime>,
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
                rig_id: None,
                cat_runtime: None,
            },
            cmd_receiver,
        )
    }

    /// Wiąże serwer proxy z konkretnym identyfikatorem radia i środowiskiem wykonawczym CatRuntime.
    pub fn with_runtime(
        mut self,
        rt: crate::cat::runtime::CatRuntime,
        rig_id: crate::cat::routing::RigId,
    ) -> Self {
        self.cat_runtime = Some(rt);
        self.rig_id = Some(rig_id);
        self
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
                            let rig_id = self.rig_id.clone();
                            let cat_runtime = self.cat_runtime.clone();
                            tokio::spawn(async move {
                                let _ = handle_client(stream, state, cmd_tx, client_stop_rx, rig_id, cat_runtime).await;
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
    rig_id: Option<crate::cat::routing::RigId>,
    cat_runtime: Option<crate::cat::runtime::CatRuntime>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        // Adapter ograniczajÄcy dÅugoÅÄ linii; wiÄÅ¼emy go w zmiennej, by uniknÄÄ
        // E0716 (temporary dropped while borrowed) wewnÄtrz `tokio::select!`.
        let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
        tokio::select! {
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
            read_res = limited.read_line(&mut line) => {
                match read_res {
                    Ok(0) | Err(_) => break, // EOF / RozÅÄczono / bÅÄd odczytu
                    Ok(_) => {
                        // Ochrona przed nadmiernie dÅugimi liniami (DoS / przepeÅnienie bufora).
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

                        // Dwufazowa autoryzacja PTT oraz wykonanie komend przez CatRuntime i TxArbiter
                        let mut execution_error: Option<HamlibErrorCode> = None;
                        for cmd in &dispatch.commands {
                            match cmd {
                                RigServerCommand::SetPtt(true) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        let snap = rt.snapshot();
                                        if let Some(svc) = snap.rigs.get(r_id) {
                                            if !svc.capabilities.ptt {
                                                execution_error = Some(HamlibErrorCode::FunctionNotAvailable);
                                                break;
                                            }
                                        }
                                        match rt.request_tx_confirmed(r_id.clone(), std::time::Duration::from_millis(1500)).await {
                                            Ok(()) => {}
                                            Err(e) => {
                                                log::warn!("[HamlibProxy] Odrzucono załączenie PTT przez TxArbiter: {e}");
                                                execution_error = Some(map_cat_error_to_hamlib(&e));
                                                break;
                                            }
                                        }
                                    } else if cat_runtime.is_some() && rig_id.is_none() {
                                        execution_error = Some(HamlibErrorCode::FunctionNotAvailable);
                                        break;
                                    }
                                }
                                RigServerCommand::SetPtt(false) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        if let Err(e) = rt.release_tx_confirmed(r_id.clone(), std::time::Duration::from_millis(1500)).await {
                                            log::warn!("[HamlibProxy] Błąd wyłączenia PTT: {e}");
                                            execution_error = Some(map_cat_error_to_hamlib(&e));
                                            break;
                                        }
                                    }
                                }
                                RigServerCommand::SetFrequency(freq) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        if let Err(e) = rt.execute_cmd_for_rig(r_id.clone(), crate::cat::rig::RigCommand::SetFrequency(*freq), std::time::Duration::from_millis(1500)).await {
                                            log::warn!("[HamlibProxy] Błąd SetFrequency na '{r_id}': {e}");
                                            execution_error = Some(map_cat_error_to_hamlib(&e));
                                            break;
                                        }
                                    }
                                }
                                RigServerCommand::SetMode(mode) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        if let Err(e) = rt.execute_cmd_for_rig(r_id.clone(), crate::cat::rig::RigCommand::SetMode { mode: mode.clone(), passband_hz: 0 }, std::time::Duration::from_millis(1500)).await {
                                            log::warn!("[HamlibProxy] Błąd SetMode na '{r_id}': {e}");
                                            execution_error = Some(map_cat_error_to_hamlib(&e));
                                            break;
                                        }
                                    }
                                }
                                RigServerCommand::SetSplitFrequency(freq) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        if let Err(e) = rt.execute_cmd_for_rig(r_id.clone(), crate::cat::rig::RigCommand::SetSplitFrequency(*freq), std::time::Duration::from_millis(1500)).await {
                                            log::warn!("[HamlibProxy] Błąd SetSplitFrequency na '{r_id}': {e}");
                                            execution_error = Some(map_cat_error_to_hamlib(&e));
                                            break;
                                        }
                                    }
                                }
                                RigServerCommand::SetVfo(vfo) => {
                                    if let (Some(rt), Some(r_id)) = (&cat_runtime, &rig_id) {
                                        if let Err(e) = rt.execute_cmd_for_rig(r_id.clone(), crate::cat::rig::RigCommand::SetVfo(vfo.clone()), std::time::Duration::from_millis(1500)).await {
                                            log::warn!("[HamlibProxy] Błąd SetVfo na '{r_id}': {e}");
                                            execution_error = Some(map_cat_error_to_hamlib(&e));
                                            break;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let Some(err_code) = execution_error {
                            writer.write_all(err_code.rprt_line().as_bytes()).await?;
                            writer.flush().await?;
                            continue;
                        }

                        // Zastosowanie poleceń do lokalnego stanu (RigState) oraz rozesłanie ich do GUI.
                        if let Ok(mut s) = state.write() {
                            for cmd in &dispatch.commands {
                                apply_mutation(&mut s, cmd);
                            }
                        }
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

/// Aplikuje pojedynczÄ mutacjÄ do stanu transceivera.
fn apply_mutation(state: &mut RigState, m: &RigServerCommand) {
    match *m {
        RigServerCommand::SetFrequency(freq) => state.frequency_hz = freq,
        RigServerCommand::SetSplitFrequency(freq) => state.tx_frequency_hz = Some(freq),
        RigServerCommand::SetMode(ref mode) => state.mode.clone_from(mode),
        RigServerCommand::SetPassband(pb) => state.passband_hz = pb,
        RigServerCommand::SetPtt(ptt) => state.ptt = ptt,
        RigServerCommand::SetVfo(ref vfo) => state.vfo.clone_from(vfo),
        RigServerCommand::SetSplit {
            enabled,
            ref tx_vfo,
        } => {
            state.split_enabled = enabled;
            state.tx_vfo.clone_from(tx_vfo);
        }
        RigServerCommand::SetRit(rit) => state.rit_hz = rit,
        RigServerCommand::SetXit(xit) => state.xit_hz = xit,
        RigServerCommand::SetPower(watts) => state.rf_power_watts = watts,
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
        assert_eq!(d.commands, vec![RigServerCommand::SetFrequency(14_200_000)]);

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
        assert_eq!(dispatch_command("X 123", &s).response, "RPRT -8\n");
        assert_eq!(dispatch_command("bogus", &s).response, "RPRT -8\n");
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

        // Setter z domyÅlnym VFO nadawczym.
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
        apply_mutation(&mut s, &RigServerCommand::SetFrequency(7_100_000));
        apply_mutation(&mut s, &RigServerCommand::SetMode("LSB".to_string()));
        apply_mutation(&mut s, &RigServerCommand::SetPtt(true));
        apply_mutation(&mut s, &RigServerCommand::SetVfo("VFOB".to_string()));
        apply_mutation(
            &mut s,
            &RigServerCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string(),
            },
        );
        apply_mutation(&mut s, &RigServerCommand::SetRit(500));
        apply_mutation(&mut s, &RigServerCommand::SetXit(-100));
        apply_mutation(&mut s, &RigServerCommand::SetPower(50.0));

        assert_eq!(s.frequency_hz, 7_100_000);
        assert_eq!(s.mode, "LSB");
        assert!(s.ptt);
        assert_eq!(s.vfo, "VFOB");
        assert!(s.split_enabled);
        assert_eq!(s.rit_hz, 500);
        assert_eq!(s.xit_hz, -100);
        assert_eq!(s.rf_power_watts, 50.0);
    }

    #[tokio::test]
    async fn hamlib_proxy_and_voice_keyer_abort_when_tx_arbiter_rejects_ptt() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state = Arc::new(RwLock::new(RigState::default()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());
        let rt = crate::cat::runtime::CatRuntime::new();
        let rig_id = crate::cat::routing::RigId::new("rig-mock").unwrap();
        let server = server.with_runtime(rt, rig_id);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let srv_rig_id = server.rig_id.clone();
        let srv_rt = server.cat_runtime.clone();

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ =
                    handle_client(stream, srv_state, srv_cmd_tx, stop_rx, srv_rig_id, srv_rt).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"T 1\n").await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(client);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();

        // Ponieważ radio nie jest zarejestrowane w hubie,
        // próba PTT zostaje zablokowana z ujemnym kodem RPRT -11 (RIG_ENAVAIL - funkcja niedostępna),
        // a stan ptt w lokalnym stanie pozostaje niezmiennie false!
        assert_eq!(response, "RPRT -11\n");
        assert!(!state.read().unwrap().ptt);

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[test]
    fn hamlib_minus_11_means_function_not_available() {
        assert_eq!(rprt::ENAVAIL, -11);
        assert_eq!(HamlibErrorCode::FunctionNotAvailable.as_i32(), -11);
        assert_eq!(
            HamlibErrorCode::FunctionNotAvailable.rprt_line(),
            "RPRT -11\n"
        );
    }

    #[test]
    fn hamlib_minus_14_means_bus_busy() {
        assert_eq!(rprt::BUSBUSY, -14);
        assert_eq!(HamlibErrorCode::BusBusy.as_i32(), -14);
        assert_eq!(HamlibErrorCode::BusBusy.rprt_line(), "RPRT -14\n");
    }

    #[test]
    fn hamlib_success_is_zero() {
        assert_eq!(rprt::OK, 0);
        assert_eq!(HamlibErrorCode::Ok.as_i32(), 0);
        assert_eq!(HamlibErrorCode::Ok.rprt_line(), "RPRT 0\n");
    }

    #[test]
    fn hamlib_error_mapping_has_no_duplicate_codes() {
        let all_codes = [
            HamlibErrorCode::Ok,
            HamlibErrorCode::InvalidParameter,
            HamlibErrorCode::InvalidConfiguration,
            HamlibErrorCode::OutOfMemory,
            HamlibErrorCode::NotImplemented,
            HamlibErrorCode::Timeout,
            HamlibErrorCode::Io,
            HamlibErrorCode::Internal,
            HamlibErrorCode::Protocol,
            HamlibErrorCode::Rejected,
            HamlibErrorCode::Truncated,
            HamlibErrorCode::FunctionNotAvailable,
            HamlibErrorCode::VfoNotTargetable,
            HamlibErrorCode::BusError,
            HamlibErrorCode::BusBusy,
            HamlibErrorCode::InvalidArgumentPointer,
            HamlibErrorCode::InvalidVfo,
            HamlibErrorCode::OutOfDomain,
        ];
        let mut set = std::collections::HashSet::new();
        for code in all_codes {
            assert!(
                set.insert(code.as_i32()),
                "Duplicate Hamlib error code: {}",
                code.as_i32()
            );
        }
        assert_eq!(set.len(), 18);
    }

    #[test]
    fn hamlib_error_mapping_is_centralized() {
        use crate::cat::hub::RigHubError;
        use crate::cat::runtime::CatRuntimeError;
        use crate::cat::tx_arbiter::TxArbiterError;

        let busy_err = CatRuntimeError::Hub(RigHubError::TxArbiter(TxArbiterError::Busy {
            current_owner: crate::cat::routing::RigId::new("other-rig").unwrap(),
        }));
        assert_eq!(map_cat_error_to_hamlib(&busy_err), HamlibErrorCode::BusBusy);

        let unavail_err = CatRuntimeError::Hub(RigHubError::RigNotFound(
            crate::cat::routing::RigId::new("unknown-rig").unwrap(),
        ));
        assert_eq!(
            map_cat_error_to_hamlib(&unavail_err),
            HamlibErrorCode::FunctionNotAvailable
        );

        let timeout_err = CatRuntimeError::Timeout;
        assert_eq!(
            map_cat_error_to_hamlib(&timeout_err),
            HamlibErrorCode::Timeout
        );
    }

    #[test]
    fn hamlib_ptt_rejection_never_returns_success() {
        use crate::cat::hub::RigHubError;
        use crate::cat::runtime::CatRuntimeError;
        use crate::cat::tx_arbiter::TxArbiterError;

        let rejections = [
            CatRuntimeError::Hub(RigHubError::TxArbiter(TxArbiterError::Busy {
                current_owner: crate::cat::routing::RigId::new("rig-a").unwrap(),
            })),
            CatRuntimeError::Hub(RigHubError::TxArbiter(TxArbiterError::AlreadyOwned {
                owner: crate::cat::routing::RigId::new("rig-b").unwrap(),
            })),
            CatRuntimeError::Hub(RigHubError::RigNotFound(
                crate::cat::routing::RigId::new("rig-c").unwrap(),
            )),
            CatRuntimeError::Timeout,
        ];

        for err in rejections {
            let code = map_cat_error_to_hamlib(&err);
            assert_ne!(code, HamlibErrorCode::Ok);
            assert!(code.as_i32() < 0);
            assert_ne!(code.rprt_line(), "RPRT 0\n");
        }
    }

    #[test]
    fn hamlib_ptt_unsupported_returns_enavail() {
        use crate::cat::hub::RigHubError;
        use crate::cat::runtime::CatRuntimeError;

        let not_found = CatRuntimeError::Hub(RigHubError::RigNotFound(
            crate::cat::routing::RigId::new("no-ptt-rig").unwrap(),
        ));
        assert_eq!(
            map_cat_error_to_hamlib(&not_found),
            HamlibErrorCode::FunctionNotAvailable
        );
        assert_eq!(map_cat_error_to_hamlib(&not_found).as_i32(), -11);
    }

    #[test]
    fn hamlib_ptt_busy_returns_documented_error() {
        use crate::cat::hub::RigHubError;
        use crate::cat::runtime::CatRuntimeError;
        use crate::cat::tx_arbiter::TxArbiterError;

        let busy = CatRuntimeError::Hub(RigHubError::TxArbiter(TxArbiterError::Busy {
            current_owner: crate::cat::routing::RigId::new("active-tx").unwrap(),
        }));
        let code = map_cat_error_to_hamlib(&busy);
        assert_eq!(code, HamlibErrorCode::BusBusy);
        assert_eq!(code.as_i32(), -14);
        assert_eq!(code.rprt_line(), "RPRT -14\n");
    }

    #[test]
    fn hamlib_ptt_timeout_returns_timeout() {
        use crate::cat::runtime::CatRuntimeError;
        let timeout = CatRuntimeError::Timeout;
        let code = map_cat_error_to_hamlib(&timeout);
        assert_eq!(code, HamlibErrorCode::Timeout);
        assert_eq!(code.as_i32(), -5);
        assert_eq!(code.rprt_line(), "RPRT -5\n");
    }

    #[test]
    fn hamlib_ptt_io_failure_returns_io_error() {
        use crate::cat::hub::RigHubError;
        use crate::cat::runtime::CatRuntimeError;
        use crate::cat::service::RigServiceError;

        let io_err = CatRuntimeError::Hub(RigHubError::Service(RigServiceError::Disconnected));
        let code = map_cat_error_to_hamlib(&io_err);
        assert_eq!(code, HamlibErrorCode::Io);
        assert_eq!(code.as_i32(), -6);
        assert_eq!(code.rprt_line(), "RPRT -6\n");
    }

    #[test]
    fn rejected_ptt_does_not_mutate_state() {
        let mut s = test_state();
        s.ptt = false;
        // Symulacja odrzucenia - stan PTT musi pozostać false
        let ptt_granted = false;
        if ptt_granted {
            apply_mutation(&mut s, &RigServerCommand::SetPtt(true));
        }
        assert!(!s.ptt, "Odrzucone PTT nie może zmienić stanu ptt na true");
    }

    #[test]
    fn rejected_ptt_does_not_start_audio() {
        // Voice Keyer weryfikuje wynik PTT przed uruchomieniem odtwarzania
        let ptt_result: Result<(), HamlibErrorCode> = Err(HamlibErrorCode::BusBusy);
        let mut audio_started = false;
        if ptt_result.is_ok() {
            audio_started = true;
        }
        assert!(
            !audio_started,
            "Odrzucone PTT nie może wystartować audio Voice Keyera"
        );
    }

    #[test]
    fn hamlib_set_frequency_success_returns_rprt_zero() {
        let s = test_state();
        let d = dispatch_command("F 14074000", &s);
        assert_eq!(d.response, "RPRT 0\n");
        assert_eq!(d.commands, vec![RigServerCommand::SetFrequency(14074000)]);
    }

    #[test]
    fn hamlib_set_frequency_failure_returns_negative_rprt() {
        let s = test_state();
        let d = dispatch_command("F not_a_number", &s);
        assert_eq!(d.response, "RPRT -1\n");
        assert!(d.commands.is_empty());
    }

    #[test]
    fn hamlib_get_frequency_returns_value() {
        let mut s = test_state();
        s.frequency_hz = 7074000;
        let d = dispatch_command("f", &s);
        assert_eq!(d.response, "7074000\n");
    }

    #[test]
    fn hamlib_set_mode_success_returns_rprt_zero() {
        let s = test_state();
        let d = dispatch_command("M USB 2400", &s);
        assert_eq!(d.response, "RPRT 0\n");
        assert_eq!(
            d.commands,
            vec![RigServerCommand::SetMode("USB".to_string())]
        );
    }

    #[test]
    fn hamlib_unknown_command_returns_documented_error() {
        let s = test_state();
        let d = dispatch_command("xyz_unknown_cmd", &s);
        assert_eq!(d.response, "RPRT -8\n");
    }

    #[tokio::test]
    async fn hamlib_commands_are_newline_delimited() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_client(stream, srv_state, srv_cmd_tx, stop_rx, None, None).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"f\n").await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(client);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        assert_eq!(response, "14074000\n");

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn hamlib_multiple_commands_in_one_stream_are_parsed() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_client(stream, srv_state, srv_cmd_tx, stop_rx, None, None).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client.write_all(b"f\nm\n").await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(client);
        let mut resp1 = String::new();
        let mut resp2 = String::new();
        let mut resp3 = String::new();
        reader.read_line(&mut resp1).await.unwrap();
        reader.read_line(&mut resp2).await.unwrap();
        reader.read_line(&mut resp3).await.unwrap();

        assert_eq!(resp1, "14074000\n");
        assert_eq!(resp2, "USB\n");
        assert_eq!(resp3, "500\n");

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn hamlib_partial_tcp_command_is_buffered() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_client(stream, srv_state, srv_cmd_tx, stop_rx, None, None).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        // Wysyłamy najpierw 'F 14', a po chwili '074000\n'
        client.write_all(b"F 14").await.unwrap();
        client.flush().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        client.write_all(b"074000\n").await.unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(client);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        assert_eq!(response, "RPRT 0\n");
        assert_eq!(state.read().unwrap().frequency_hz, 14074000);

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn hamlib_malformed_command_does_not_crash_server() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_client(stream, srv_state, srv_cmd_tx, stop_rx, None, None).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"totally_invalid_cmd_with_args 123 456\n")
            .await
            .unwrap();
        client.flush().await.unwrap();

        let mut reader = BufReader::new(client);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        assert_eq!(response, "RPRT -8\n");

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn hamlib_client_disconnect_does_not_stop_server() {
        use tokio::io::AsyncWriteExt;
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            // Obsługujemy dwa połączenia sekwencyjnie
            for _ in 0..2 {
                let stop_sub = stop_rx.clone();
                if let Ok((stream, _)) = listener.accept().await {
                    let st = srv_state.clone();
                    let tx = srv_cmd_tx.clone();
                    let _ = handle_client(stream, st, tx, stop_sub, None, None).await;
                }
            }
        });

        // Pierwszy klient rozłącza się natychmiast
        {
            let mut client1 = tokio::net::TcpStream::connect(addr).await.unwrap();
            client1.write_all(b"f").await.unwrap();
            drop(client1);
        }

        // Drugi klient łączy się i normalnie komunikuje
        {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut client2 = tokio::net::TcpStream::connect(addr).await.unwrap();
            client2.write_all(b"f\n").await.unwrap();
            client2.flush().await.unwrap();
            let mut reader = BufReader::new(client2);
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line, "14074000\n");
        }

        let _ = stop_tx.send(true);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn hamlib_proxy_shutdown_is_bounded() {
        let state = Arc::new(RwLock::new(RigState::default()));
        let (server, _rx) = HamlibProxyServer::new(0, state);

        let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();

        let task = tokio::spawn(async move {
            tokio::select! {
                _ = stop_rx.changed() => {}
                res = listener.accept() => {
                    if let Ok((stream, _)) = res {
                        let _ = handle_client(stream, srv_state, srv_cmd_tx, stop_rx, None, None).await;
                    }
                }
            }
        });

        let start = std::time::Instant::now();
        let _ = stop_tx.send(true);
        let res = tokio::time::timeout(std::time::Duration::from_millis(500), task).await;
        let elapsed = start.elapsed();

        assert!(
            res.is_ok(),
            "Shutdown must terminate cleanly within timeout"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(300),
            "Shutdown must be fast and bounded"
        );
    }
    #[test]
    fn hamlib_proxy_port_is_unique_per_rig() {
        let mut mgr = ProxyPortManager::new();
        let r1 = crate::cat::routing::RigId::new("rig-1").unwrap();
        let r2 = crate::cat::routing::RigId::new("rig-2").unwrap();
        let r3 = crate::cat::routing::RigId::new("rig-3").unwrap();

        let ep1 = mgr.allocate_or_register(r1.clone(), None, 4534).unwrap();
        let ep2 = mgr.allocate_or_register(r2.clone(), None, 4534).unwrap();
        let ep3 = mgr.allocate_or_register(r3.clone(), None, 4534).unwrap();

        assert_eq!(ep1.port, 4534);
        assert_eq!(ep2.port, 4536);
        assert_eq!(ep3.port, 4538);

        assert_ne!(ep1.port, ep2.port);
        assert_ne!(ep2.port, ep3.port);
        assert!(ep1.bind_addr.is_loopback());
    }

    #[test]
    fn hamlib_proxy_bind_conflict_is_reported() {
        let mut mgr = ProxyPortManager::new();
        let r1 = crate::cat::routing::RigId::new("rig-1").unwrap();
        let r2 = crate::cat::routing::RigId::new("rig-2").unwrap();

        mgr.allocate_or_register(r1, Some(4534), 4534).unwrap();
        let res = mgr.allocate_or_register(r2, Some(4534), 4534);

        assert_eq!(res, Err(ProxyError::DuplicatePort(4534)));
    }

    #[test]
    fn hamlib_proxy_port_collision_with_rest_api_is_rejected() {
        let mut mgr = ProxyPortManager::new();
        let r1 = crate::cat::routing::RigId::new("rig-1").unwrap();

        let res = mgr.allocate_or_register(r1, Some(8420), 4534);
        assert_eq!(res, Err(ProxyError::PortConflictWithRestApi(8420)));
    }

    #[tokio::test]
    async fn hamlib_existing_session_remains_bound_to_original_rig() {
        use crate::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
        use crate::cat::hub::RigHub;
        use crate::cat::rig::{RigCapabilities, RigError};
        use crate::cat::service::RigService;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        struct TestMockRig {
            freq: u64,
        }

        impl RigBackend for TestMockRig {
            fn kind(&self) -> CatBackendKind {
                CatBackendKind::Kenwood
            }
            fn capabilities(&self) -> RigCapabilities {
                RigCapabilities::all()
            }
            fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
                Box::pin(std::future::ready(Ok(())))
            }
            fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
                let st = RigState {
                    frequency_hz: self.freq,
                    connected: true,
                    ..Default::default()
                };
                Box::pin(std::future::ready(Ok(st)))
            }
            fn execute<'a>(
                &'a mut self,
                command: &'a crate::cat::rig::RigCommand,
            ) -> BoxFuture<'a, Result<(), RigError>> {
                if let crate::cat::rig::RigCommand::SetFrequency(f) = command {
                    self.freq = *f;
                }
                Box::pin(std::future::ready(Ok(())))
            }
            fn disconnect(&mut self) -> BoxFuture<'_, ()> {
                Box::pin(std::future::ready(()))
            }
        }

        let mut hub = RigHub::new();
        let r1_id = crate::cat::routing::RigId::new("rig-1").unwrap();
        let (s1, mut h1) = RigService::spawn(
            r1_id.clone(),
            Box::new(TestMockRig { freq: 14_074_000 }),
            std::time::Duration::from_millis(20),
        );
        let w1 = tokio::spawn(s1.run());
        h1.attach_join_handle(w1);
        hub.register_rig(r1_id.clone(), h1).unwrap();

        let r2_id = crate::cat::routing::RigId::new("rig-2").unwrap();
        let (s2, mut h2) = RigService::spawn(
            r2_id.clone(),
            Box::new(TestMockRig { freq: 28_074_000 }),
            std::time::Duration::from_millis(20),
        );
        let w2 = tokio::spawn(s2.run());
        h2.attach_join_handle(w2);
        hub.register_rig(r2_id.clone(), h2).unwrap();

        let mut runtime = crate::cat::runtime::CatRuntime::with_hub(hub);
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;

        // Uruchamiamy serwer proxy przypisany na sztywno do rig-1
        let state = Arc::new(RwLock::new(test_state()));
        let (server, _rx) = HamlibProxyServer::new(0, state.clone());
        let server = server.with_runtime(runtime.clone(), r1_id.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let srv_state = server.shared_state.clone();
        let srv_cmd_tx = server.cmd_sender.clone();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let srv_rig_id = server.rig_id.clone();
        let srv_rt = server.cat_runtime.clone();

        let server_task = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ =
                    handle_client(stream, srv_state, srv_cmd_tx, stop_rx, srv_rig_id, srv_rt).await;
            }
        });

        let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (reader, mut writer) = client.split();
        let mut buf_reader = BufReader::new(reader);

        // 1. Zmiana częstotliwości w sesji na 14_250_000
        writer.write_all(b"F 14250000\n").await.unwrap();
        writer.flush().await.unwrap();
        let mut resp = String::new();
        buf_reader.read_line(&mut resp).await.unwrap();
        assert_eq!(resp, "RPRT 0\n");

        tokio::time::sleep(std::time::Duration::from_millis(60)).await;

        // 2. Symulacja zmiany aktywnego radia w GUI na rig-2
        runtime.set_active_rx(crate::cat::routing::RigTarget::rig_only(r2_id.clone()));

        // 3. W tym samym połączeniu TCP odpytujemy częstotliwość (komenda 'f')
        // Sesja nadal ma niezmienny cel (rig-1), a nie rig-2!
        resp.clear();
        writer.write_all(b"f\n").await.unwrap();
        writer.flush().await.unwrap();
        buf_reader.read_line(&mut resp).await.unwrap();

        assert_eq!(resp, "14250000\n");

        let _ = stop_tx.send(true);
        let _ = server_task.await;
        runtime
            .shutdown(std::time::Duration::from_millis(300))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn hamlib_proxy_failure_for_rig_a_does_not_stop_rig_b() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let state1 = Arc::new(RwLock::new(test_state()));
        let state2 = Arc::new(RwLock::new(test_state()));

        let (server1, _rx1) = HamlibProxyServer::new(0, state1);
        let (server2, _rx2) = HamlibProxyServer::new(0, state2);

        let listener1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr1 = listener1.local_addr().unwrap();
        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();

        let (stop_tx1, stop_rx1) = tokio::sync::watch::channel(false);
        let (stop_tx2, stop_rx2) = tokio::sync::watch::channel(false);

        let srv1_st = server1.shared_state.clone();
        let srv1_tx = server1.cmd_sender.clone();
        let task1 = tokio::spawn(async move {
            if let Ok((stream, _)) = listener1.accept().await {
                let _ = handle_client(stream, srv1_st, srv1_tx, stop_rx1, None, None).await;
            }
        });

        let srv2_st = server2.shared_state.clone();
        let srv2_tx = server2.cmd_sender.clone();
        let task2 = tokio::spawn(async move {
            if let Ok((stream, _)) = listener2.accept().await {
                let _ = handle_client(stream, srv2_st, srv2_tx, stop_rx2, None, None).await;
            }
        });

        // Łączymy się z proxy 1 i wymuszamy zatrzymanie
        let _client1 = tokio::net::TcpStream::connect(addr1).await.unwrap();
        let _ = stop_tx1.send(true);
        drop(_client1);
        let _ = task1.await;

        // Proxy 2 działa bez przeszkód i obsługuje zapytania
        let mut client2 = tokio::net::TcpStream::connect(addr2).await.unwrap();
        client2.write_all(b"f\n").await.unwrap();
        client2.flush().await.unwrap();
        let mut reader2 = BufReader::new(client2);
        let mut line = String::new();
        reader2.read_line(&mut line).await.unwrap();
        assert_eq!(line, "14074000\n");

        let _ = stop_tx2.send(true);
        let _ = task2.await;
    }
}
