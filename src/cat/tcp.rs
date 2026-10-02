// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wspólny transport TCP dla demonów Hamlib (rigctld/rotctld).

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Maksymalna długość pojedynczej linii odpowiedzi demona.
pub(super) const MAX_LINE_LEN: usize = 256;
/// Maksymalny czas oczekiwania na operacje sieciowe z demonem.
const TCP_TIMEOUT: Duration = Duration::from_millis(1500);
/// Maksymalny czas oczekiwania na odpowiedź, która jest następnie odrzucana.
const IGNORED_REPLY_TIMEOUT: Duration = Duration::from_millis(500);

fn timed_out(msg: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::TimedOut, msg)
}

pub(super) async fn connect_timeout(addr: &str, daemon: &str) -> Result<TcpStream, std::io::Error> {
    match tokio::time::timeout(TCP_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(res) => res,
        Err(_) => Err(timed_out(format!(
            "Przekroczono czas oczekiwania na połączenie z {daemon}"
        ))),
    }
}

pub(super) async fn write_cmd<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    cmd: &[u8],
    daemon: &str,
) -> Result<(), std::io::Error> {
    match tokio::time::timeout(TCP_TIMEOUT, writer.write_all(cmd)).await {
        Ok(res) => res,
        Err(_) => Err(timed_out(format!("Przekroczono czas zapisu do {daemon}"))),
    }
}

pub(super) async fn read_line_timeout<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    line: &mut String,
    daemon: &str,
) -> Result<usize, std::io::Error> {
    line.clear();
    let mut limited = reader.take((MAX_LINE_LEN + 1) as u64);
    match tokio::time::timeout(TCP_TIMEOUT, limited.read_line(line)).await {
        Ok(res) => res,
        Err(_) => Err(timed_out(format!("Przekroczono czas odczytu z {daemon}"))),
    }
}

/// Łączy się z demonem, wysyła komendę i odrzuca (best-effort) pierwszą linię odpowiedzi.
pub(super) async fn send_cmd_ignore_reply(
    addr: &str,
    cmd: &str,
    daemon: &str,
) -> Result<(), std::io::Error> {
    let stream = connect_timeout(addr, daemon).await?;
    let (reader, mut writer) = stream.into_split();
    write_cmd(&mut writer, cmd.as_bytes(), daemon).await?;
    let mut buf_reader = BufReader::new(reader);
    let mut resp = String::new();
    let _ = tokio::time::timeout(
        IGNORED_REPLY_TIMEOUT,
        (&mut buf_reader)
            .take((MAX_LINE_LEN + 1) as u64)
            .read_line(&mut resp),
    )
    .await;
    Ok(())
}
