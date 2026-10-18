// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Model sesji synchronizacji z pojedynczym writera i asynchronicznym odczytem (`src/sync/session.rs`).

use super::crypto::{KEY_LEN, SESSION_ID_LEN, build_aad, decrypt_payload, encrypt_payload};
use super::error::SyncError;
use super::frame::{read_frame, write_frame};
use super::protocol::Message;
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub const QUEUE_CAPACITY: usize = 256;

/// Wątkowo-bezpieczny uchwyt do wysyłania wiadomości przez pętlę writera sesji.
#[derive(Clone, Debug)]
pub struct SessionSender {
    tx: mpsc::Sender<Message>,
}

impl SessionSender {
    pub fn from_channel(tx: mpsc::Sender<Message>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, msg: Message) -> Result<(), SyncError> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| SyncError::ConnectionClosed)
    }

    pub fn try_send(&self, msg: Message) -> Result<(), SyncError> {
        match self.tx.try_send(msg) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => Err(SyncError::QueueFull),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(SyncError::ConnectionClosed),
        }
    }
}

/// Pojedyncza uwierzytelniona i szyfrowana sesja synchronizacji.
pub struct SyncSession {
    station: String,
    sender: SessionSender,
    reader: OwnedReadHalf,
    key: [u8; KEY_LEN],
    session_id: [u8; SESSION_ID_LEN],
    recv_direction: u8,
    recv_counter: u64,
    writer_handle: Option<JoinHandle<()>>,
}

impl SyncSession {
    pub fn new(
        stream: TcpStream,
        key: [u8; KEY_LEN],
        session_id: [u8; SESSION_ID_LEN],
        send_direction: u8,
        recv_direction: u8,
        station: String,
    ) -> Self {
        let (reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::channel::<Message>(QUEUE_CAPACITY);

        let writer_handle =
            tokio::spawn(run_writer_loop(writer, rx, key, session_id, send_direction));

        Self {
            station,
            sender: SessionSender { tx },
            reader,
            key,
            session_id,
            recv_direction,
            recv_counter: 0,
            writer_handle: Some(writer_handle),
        }
    }

    pub fn sender(&self) -> SessionSender {
        self.sender.clone()
    }

    pub fn station(&self) -> &str {
        &self.station
    }

    pub async fn send(&self, msg: Message) -> Result<(), SyncError> {
        self.sender.send(msg).await
    }

    pub fn try_send(&self, msg: Message) -> Result<(), SyncError> {
        self.sender.try_send(msg)
    }

    /// Odbiera i odszyfrowuje kolejną wiadomość ze strumienia TCP.
    /// Weryfikuje monotoniczny licznik i AAD chroniąc przed replay i reorder.
    pub async fn recv(&mut self) -> Result<Message, SyncError> {
        let frame = read_frame(&mut self.reader).await?;
        let aad = build_aad(&self.session_id, self.recv_direction, self.recv_counter);
        let plaintext = decrypt_payload(&self.key, &aad, &frame)?;
        self.recv_counter = self.recv_counter.wrapping_add(1);
        Message::from_json(&plaintext)
    }

    /// Bezpiecznie zamyka sesję, wysyłając Message::Bye i zatrzymując zadanie zapisu.
    pub async fn shutdown(&mut self) {
        let _ = self.sender.send(Message::Bye).await;
        if let Some(handle) = self.writer_handle.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_millis(500), handle).await;
        }
    }
}

impl Drop for SyncSession {
    fn drop(&mut self) {
        if let Some(handle) = self.writer_handle.take() {
            handle.abort();
        }
    }
}

async fn run_writer_loop(
    mut writer: OwnedWriteHalf,
    mut rx: mpsc::Receiver<Message>,
    key: [u8; KEY_LEN],
    session_id: [u8; SESSION_ID_LEN],
    send_direction: u8,
) {
    let mut counter: u64 = 0;

    while let Some(msg) = rx.recv().await {
        let is_bye = matches!(msg, Message::Bye);

        let Ok(plaintext) = msg.to_json() else {
            break;
        };

        let aad = build_aad(&session_id, send_direction, counter);
        let Ok(encrypted) = encrypt_payload(&key, &aad, &plaintext) else {
            break;
        };

        counter = counter.wrapping_add(1);

        if write_frame(&mut writer, &encrypted).await.is_err() {
            break;
        }

        if is_bye {
            break;
        }
    }
}
