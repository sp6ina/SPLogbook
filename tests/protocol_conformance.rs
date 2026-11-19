// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz WoĹşniak (SP6INA)
//! Protocol conformance â€” SOFTWARE-ONLY verification against official documentation.
//!
//! Every assertion in this file is grounded in an official manufacturer / official project
//! reference document (see `docs/HARDWARE_VALIDATION_MATRIX.md` and `ADR-72`–`ADR-74`).
//!
//! The tests call the **real** production codecs and clients. No hardware, no serial port, no
//! real `rigctld`, no real transceiver and no RF is involved. Loopback TCP listeners are
//! test-owned devices, not real daemons.

use std::time::Duration;

use splogbook::cat::backend::RigBackend;
use splogbook::cat::flrig::{FlrigClient, XmlRpcRequest};
use splogbook::cat::hamlib::HamlibClient;
use splogbook::cat::native::elecraft::ElecraftCodec;
use splogbook::cat::native::elecraft::codec::ElecraftCat;
use splogbook::cat::native::flexradio::protocol::FlexCommand;
use splogbook::cat::native::icom::codec::{IcomCiVCodec, IcomCiVFrame, command, subcommand};
use splogbook::cat::native::kenwood::codec::KenwoodCat;
use splogbook::cat::native::kenwood::models::KenwoodPttContract;
use splogbook::cat::native::yaesu::codec::YaesuCat;
use splogbook::cat::tci::TciProtocol;
use tokio::io::AsyncReadExt;

#[path = "support/protocol_double.rs"]
mod protocol_double;

#[path = "support/flrig_wire.rs"]
mod flrig_wire;

use flrig_wire::{FlrigReply, spawn_flrig_wire};
use protocol_double::{DocumentedPttProtocol, DoubleFaults, ProtocolDouble};

/// The documented protocol table must cover every production PTT-capable backend exactly once,
/// and every entry must carry an official source.
#[test]
fn documented_ptt_table_covers_every_ptt_backend_with_an_official_source() {
    let table = [
        DocumentedPttProtocol::ICOM_CI_V,
        DocumentedPttProtocol::YAESU_CAT,
        DocumentedPttProtocol::KENWOOD_CAT,
        DocumentedPttProtocol::ELECRAFT_CAT,
        DocumentedPttProtocol::FLEX_SMARTSDR,
        DocumentedPttProtocol::TCI,
        DocumentedPttProtocol::HAMLIB_RIGCTLD,
        DocumentedPttProtocol::FLRIG,
    ];
    assert_eq!(table.len(), 8, "every PTT-capable backend is represented");
    for entry in table {
        assert!(!entry.source.trim().is_empty(), "official source required");
        assert!(!entry.ptt_on.trim().is_empty());
        assert!(!entry.ptt_off.trim().is_empty());
        assert_ne!(entry.ptt_on, entry.ptt_off, "ON and OFF must differ");
        assert_eq!(entry.frame_for(true), entry.ptt_on);
        assert_eq!(entry.frame_for(false), entry.ptt_off);
    }
}

/// Icom CI-V `1C 00`: official IC-7610 guide row `1C 00 *1 00 or 01 Transceiver's status
/// (00=RX, 01=TX)`. The real codec must produce exactly that frame and parse the answer.
#[test]
fn icom_civ_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::ICOM_CI_V;
    let on = IcomCiVFrame::new(
        0x94,
        0xE0,
        command::SET_PTT,
        Some(subcommand::PTT_CTRL),
        vec![0x01],
    )
    .encode();
    let off = IcomCiVFrame::new(
        0x94,
        0xE0,
        command::SET_PTT,
        Some(subcommand::PTT_CTRL),
        vec![0x00],
    )
    .encode();
    assert_eq!(hex(&on), documented.ptt_on);
    assert_eq!(hex(&off), documented.ptt_off);
    assert_eq!(hex(&on), "FE FE 94 E0 1C 00 01 FD");
    assert_eq!(hex(&off), "FE FE 94 E0 1C 00 00 FD");

    // Documented answer frame: the transceiver reports its own status (01 = TX).
    let codec = IcomCiVCodec::new(0xE0, 0x94);
    let mut buffer = vec![0xFE, 0xFE, 0xE0, 0x94, 0x1C, 0x00, 0x01, 0xFD];
    let frame = codec
        .extract_frame(&mut buffer)
        .expect("documented frame parses")
        .expect("complete frame");
    assert_eq!(frame.command, command::SET_PTT);
    assert_eq!(frame.subcommand, Some(subcommand::PTT_CTRL));
    assert_eq!(frame.data, vec![0x01], "01 = TX");
}

/// Yaesu CAT: official CAT Operation Reference (FT-891 1909-C, FT-991A 1711-D/1808-D,
/// FT-710 2306-C, FTDX10 2308-F): Set `TX1;` (P1=1), Set `TX0;` (P1=0), Read `TX;`,
/// Answer `TX P1;` â€” P1=2 means "RADIO TX ON, CAT TX OFF".
#[test]
fn yaesu_cat_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::YAESU_CAT;
    assert_eq!(YaesuCat::set_ptt(true), documented.ptt_on);
    assert_eq!(YaesuCat::set_ptt(false), documented.ptt_off);
    assert_eq!(YaesuCat::read_ptt(), documented.ptt_read.unwrap());
    assert_eq!(YaesuCat::set_ptt(true), "TX1;");
    assert_eq!(YaesuCat::set_ptt(false), "TX0;");

    assert_eq!(YaesuCat::parse_ptt_resp("TX1;"), Some(true));
    assert_eq!(YaesuCat::parse_ptt_resp("TX0;"), Some(false));
    assert_eq!(
        YaesuCat::parse_ptt_resp("TX2;"),
        Some(true),
        "documented Answer P1=2: RADIO TX ON, CAT TX OFF"
    );
    assert_eq!(YaesuCat::parse_ptt_resp("TX9;"), None);
}

/// Kenwood PC CONTROL COMMAND Reference (B5A-0316-20/01 TS-590S/SG, B5A-2944-00/01 TS-890S,
/// B5A-3170-00 TS-990S): `TX;` keys the transmitter (P1 omitted = SEND/PTT) and `RX;` returns to
/// receive. Every supported model carries its own documented contract.
#[test]
fn kenwood_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::KENWOOD_CAT;
    assert_eq!(KenwoodCat::set_ptt_cmd(true), documented.ptt_on);
    assert_eq!(KenwoodCat::set_ptt_cmd(false), documented.ptt_off);
    assert_eq!(KenwoodCat::set_ptt_cmd(true), "TX;");
    assert_eq!(KenwoodCat::set_ptt_cmd(false), "RX;");
    assert_eq!(KenwoodCat::parse_ptt_resp("RX;"), Some(false));
    // Documented Answer values: TX0 = SEND, TX1 = DATA SEND, TX2 = TX Tune — all are transmit.
    assert_eq!(KenwoodCat::parse_ptt_resp("TX0;"), Some(true));
    assert_eq!(KenwoodCat::parse_ptt_resp("TX1;"), Some(true));
    assert_eq!(KenwoodCat::parse_ptt_resp("TX2;"), Some(true));

    // Per-model documented contracts (no brand-wide assumption).
    let models = [
        ("TS-590S", true),
        ("TS-590SG", true),
        ("TS-890S", false),
        ("TS-990S", false),
    ];
    for (model_id, documents_if) in models {
        let contract = KenwoodPttContract::for_model(model_id)
            .unwrap_or_else(|| panic!("{model_id} has no documented PTT contract"));
        assert_eq!(contract.ptt_on, KenwoodCat::set_ptt_cmd(true));
        assert_eq!(contract.ptt_off, KenwoodCat::set_ptt_cmd(false));
        assert_eq!(contract.documents_status_read(), documents_if, "{model_id}");
        assert!(contract.document.contains("PC CONTROL COMMAND"));
    }
    assert!(KenwoodPttContract::for_model("TS-2000").is_none());
}

/// CAT-018 (conformant after the fix): the official references define `TX P1;` with `P1 = 0` as
/// SEND (transmit). The production parser now matches that documented Answer model; a bare `TX;`
/// is **not** a documented Answer and yields no PTT state.
#[test]
fn kenwood_tx0_answer_is_the_documented_send_answer() {
    assert_eq!(
        KenwoodCat::parse_ptt_resp("TX0;"),
        Some(true),
        "documented P1=0 = SEND = transmit"
    );
    assert_eq!(KenwoodCat::parse_ptt_resp("TX;"), None);
    assert_eq!(
        KenwoodCat::set_ptt_cmd(false),
        "RX;",
        "the release path uses the documented RX; command"
    );
}

/// Elecraft: K3S/K3/KX3/KX2 Programmer's Reference Rev. G5 (2019-02-20) and K4 Rev. D12:
/// Set `TX;` / `RX;`, TQ query answers `TQ0;` (receive) or `TQ1;` (transmit).
#[test]
fn elecraft_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::ELECRAFT_CAT;
    assert_eq!(ElecraftCat::set_ptt(true), documented.ptt_on);
    assert_eq!(ElecraftCat::set_ptt(false), documented.ptt_off);
    assert_eq!(ElecraftCat::set_ptt(true), "TX;");
    assert_eq!(ElecraftCat::set_ptt(false), "RX;");
    let codec = ElecraftCodec::default();
    assert_eq!(codec.encode_command("TQ"), b"TQ;".to_vec());
    assert_eq!(codec.encode_command(documented.ptt_on), b"TX;".to_vec());
    assert_eq!(codec.encode_command(documented.ptt_off), b"RX;".to_vec());
}

/// TCI Protocol 2.1 (Expert Electronics, Sep. 2026): `TRX:arg1,arg2;` with `arg2 = true/false`
/// and a documented `TRX:arg1,arg2;` reply.
#[test]
fn tci_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::TCI;
    assert_eq!(TciProtocol::cmd_set_trx(0, true), documented.ptt_on);
    assert_eq!(TciProtocol::cmd_set_trx(0, false), documented.ptt_off);
    assert_eq!(TciProtocol::cmd_set_trx(0, true), "trx:0,true;");
    assert_eq!(TciProtocol::cmd_set_trx(0, false), "trx:0,false;");
}

/// FlexRadio SmartSDR TCP/IP API (`TCPIP xmit`, rev 177f09f, 2024-07-01): `xmit 1` / `xmit 0`,
/// answered with `R<seq>|0|`.
#[test]
fn flexradio_documented_ptt_frames_match_the_production_codec() {
    let documented = DocumentedPttProtocol::FLEX_SMARTSDR;
    assert_eq!(documented.ptt_on, "xmit 1");
    assert_eq!(documented.ptt_off, "xmit 0");
    assert_eq!(
        FlexCommand::new(19, documented.ptt_on).encode(),
        "C19|xmit 1\n"
    );
    assert_eq!(
        FlexCommand::new(20, documented.ptt_off).encode(),
        "C20|xmit 0\n"
    );
}

/// Hamlib `rigctld` 4.7.2: `T 1` / `T 0` are the documented set commands. The real production
/// client must put exactly those bytes on the wire (test-owned loopback listener).
#[tokio::test]
async fn hamlib_documented_ptt_frames_are_sent_by_the_production_client() {
    for (ptt, expected) in [(true, "T 1\n"), (false, "T 0\n")] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test-owned loopback listener");
        let port = listener.local_addr().expect("local addr").port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = vec![0u8; 64];
            let read = socket.read(&mut buffer).await.expect("read");
            String::from_utf8_lossy(&buffer[..read]).to_string()
        });
        HamlibClient::set_ptt("127.0.0.1", port, ptt)
            .await
            .expect("production client writes the command");
        let sent = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("listener completes")
            .expect("join");
        assert_eq!(sent, expected, "documented rigctld set-PTT frame");
    }
}

/// FLRig 2.0.00 `xml_server.cxx`: `rig.set_ptt` is documented with the **integer** parameter
/// type (`n:i`, "set PTT 1/0"), and `rig.get_ptt` returns `<int>`. CAT-019 (conformant after the
/// fix): the production client must put exactly the documented `<int>` payload on the wire.
#[tokio::test]
async fn flrig_documented_set_ptt_parameter_is_an_integer() {
    let documented_on = XmlRpcRequest::new("rig.set_ptt").param_int(1).to_xml();
    let documented_off = XmlRpcRequest::new("rig.set_ptt").param_int(0).to_xml();
    assert!(documented_on.contains("<int>1</int>"));
    assert!(documented_off.contains("<int>0</int>"));
    assert!(!documented_on.contains("<boolean>"));

    // The real production client against a test-only loopback XML-RPC daemon.
    let wire = spawn_flrig_wire(FlrigReply::Nil).await;
    let client = FlrigClient::new("127.0.0.1", wire.port());
    for (ptt, expected) in [
        (true, documented_on.clone()),
        (false, documented_off.clone()),
    ] {
        let _ = client.set_ptt(ptt).await;
        assert_eq!(
            wire.last_body(),
            expected,
            "the production client must send the documented integer payload"
        );
    }
    assert!(
        !wire.last_body().contains("<boolean>"),
        "no boolean parameter is ever sent"
    );
    wire.shutdown().await;
}

/// Every documented double implements the documented ONâ†’OFF order and never keys PTT on a
/// preflight failure.
#[tokio::test]
async fn doubles_reflect_documented_silence_and_error_behaviour() {
    // Silent ON: the documented command timeout is a controlled error with no confirmed ON.
    let mut silent = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            silent_on: true,
            ..Default::default()
        },
    );
    let counters = silent.counters();
    let outcome = silent
        .execute(&splogbook::cat::rig::RigCommand::SetPtt(true))
        .await;
    assert!(outcome.is_err(), "documented timeout is an error");
    assert!(!counters.confirmed_on());

    // Documented error code line: a rejected command is an error.
    let mut rejected = ProtocolDouble::with_faults(
        DocumentedPttProtocol::HAMLIB_RIGCTLD,
        DoubleFaults {
            reject_on: true,
            ..Default::default()
        },
    );
    let outcome = rejected
        .execute(&splogbook::cat::rig::RigCommand::SetPtt(true))
        .await;
    assert!(outcome.is_err());
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
