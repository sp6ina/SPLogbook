// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Ramkowanie strumienia TCP dla protokołu synchronizacji (`src/sync/frame.rs`).

use super::error::SyncError;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maksymalny dopuszczalny rozmiar pojedynczej ramki: 8 MiB.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Koduje długość ramki do 4 bajtów big-endian.
pub fn encode_length(len: usize) -> [u8; 4] {
    (len as u32).to_be_bytes()
}

/// Dekoduje 4-bajtową długość ramki i weryfikuje limit MAX_FRAME_BYTES.
pub fn decode_length(bytes: &[u8; 4]) -> Result<usize, SyncError> {
    let len = u32::from_be_bytes(*bytes) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(SyncError::FrameTooLarge {
            size: len,
            limit: MAX_FRAME_BYTES,
        });
    }
    Ok(len)
}

/// Odczytuje kompletną ramkę ze strumienia asynchronicznego.
/// Nie alokuje pamięci powyżej limitu MAX_FRAME_BYTES przy odebraniu nadmiarowej deklaracji długości.
pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>, SyncError>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(SyncError::ConnectionClosed);
        }
        Err(e) => return Err(SyncError::Io(e.to_string())),
    }

    let len = decode_length(&len_buf)?;
    if len == 0 {
        return Ok(Vec::new());
    }

    let mut data = vec![0u8; len];
    match reader.read_exact(&mut data).await {
        Ok(_) => Ok(data),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(SyncError::FrameTruncated),
        Err(e) => Err(SyncError::Io(e.to_string())),
    }
}

/// Zapisuje kompletną ramkę [4 B długość][dane] do strumienia asynchronicznego i opróżnia bufor.
pub async fn write_frame<W>(writer: &mut W, data: &[u8]) -> Result<(), SyncError>
where
    W: AsyncWrite + Unpin,
{
    if data.len() > MAX_FRAME_BYTES {
        return Err(SyncError::FrameTooLarge {
            size: data.len(),
            limit: MAX_FRAME_BYTES,
        });
    }

    let len_buf = encode_length(data.len());
    writer
        .write_all(&len_buf)
        .await
        .map_err(|e| SyncError::Io(e.to_string()))?;
    writer
        .write_all(data)
        .await
        .map_err(|e| SyncError::Io(e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| SyncError::Io(e.to_string()))?;
    Ok(())
}
