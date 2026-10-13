// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny moduł obsługi radiotelefonów Icom (CI-V).

pub mod backend;
pub mod codec;
pub mod models;

pub use backend::IcomCiVBackend;
pub use codec::{CiV, IcomCiVCodec, IcomCiVFrame};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    use crate::cat::backend::RigBackend;
    use crate::cat::native::icom::models::ICOM_IC7300;
    use crate::cat::native::transport::SerialConfig;
    use crate::cat::rig::{RigCommand, RigError};

    fn test_config() -> SerialConfig {
        SerialConfig {
            port_name: "COM_TEST".to_string(),
            baud_rate: 115_200,
            data_bits: crate::cat::native::transport::SerialDataBits::Eight,
            stop_bits: crate::cat::native::transport::SerialStopBits::One,
            parity: crate::cat::native::transport::SerialParity::None,
            flow_control: crate::cat::native::transport::SerialFlowControl::None,
            connect_timeout: Duration::from_millis(300),
            read_timeout: Duration::from_millis(300),
            write_timeout: Duration::from_millis(300),
            max_buffer_size: 1024,
        }
    }

    #[tokio::test]
    async fn icom_backend_poll_reads_state() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        tokio::spawn(async move {
            let mut buf = [0u8; 128];

            // 1. Oczekiwanie na odczyt częstotliwości (0x03)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD]);
            // Odpowiedź: 14.074.000 Hz
            radio_io
                .write_all(&[
                    0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD,
                ])
                .await
                .unwrap();

            // 2. Oczekiwanie na odczyt trybu (0x04)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x04, 0xFD]);
            // Odpowiedź: USB, filtr 1
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x04, 0x01, 0x01, 0xFD])
                .await
                .unwrap();

            // 3. Oczekiwanie na odczyt S-Metra (0x15 0x02)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x15, 0x02, 0xFD]);
            // Odpowiedź: S9 (raw 0120)
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x15, 0x02, 0x20, 0x01, 0xFD])
                .await
                .unwrap();

            // 4. Oczekiwanie na odczyt RF power (0x14 0x0A)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x14, 0x0A, 0xFD]);
            // Odpowiedź: 50%
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x14, 0x0A, 0x28, 0x01, 0xFD])
                .await
                .unwrap();

            // 5. Oczekiwanie na odczyt PTT (0x1C 0x00)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x1C, 0x00, 0xFD]);
            // Odpowiedź: RX (0x00)
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x1C, 0x00, 0x00, 0xFD])
                .await
                .unwrap();
        });

        backend.connect().await.expect("connect");
        let state = backend.poll().await.expect("poll");

        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");
        assert_eq!(state.s_meter_unit, "S9");
        assert!(!state.ptt);
    }

    #[tokio::test]
    async fn icom_backend_execute_common_commands() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        tokio::spawn(async move {
            let mut buf = [0u8; 128];

            // 1. SetFrequency(7_074_000)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(
                &buf[..n],
                &[
                    0xFE, 0xFE, 0x94, 0xE0, 0x05, 0x00, 0x40, 0x07, 0x07, 0x00, 0xFD
                ]
            );
            // ACK
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD])
                .await
                .unwrap();

            // 2. SetMode("CW")
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x06, 0x03, 0x01, 0xFD]);
            // ACK
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD])
                .await
                .unwrap();

            // 3. SetPtt(true)
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x1C, 0x00, 0x01, 0xFD]);
            // ACK
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD])
                .await
                .unwrap();
        });

        backend.connect().await.unwrap();
        backend
            .execute(&RigCommand::SetFrequency(7_074_000))
            .await
            .unwrap();
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();
    }

    #[tokio::test]
    async fn icom_backend_unsupported_command() {
        let (client_io, _radio_io) = duplex(512);
        let mut profile = ICOM_IC7300.clone();
        profile.capabilities.power = false; // Wyłącz obsługę mocy w profilu

        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), profile, Some(0x94));
        backend.connect().await.unwrap();

        let res = backend.execute(&RigCommand::SetPower(50.0)).await;
        assert!(matches!(
            res,
            Err(RigError::Unsupported {
                command: "SetPower",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn icom_backend_timeout_is_typed() {
        let (client_io, _radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        backend.connect().await.unwrap();
        // Radio nie odpowiada -> spodziewany RigError::Timeout
        let res = backend.poll().await;
        assert!(matches!(res, Err(RigError::Timeout { .. })));
    }

    #[tokio::test]
    async fn icom_backend_disconnect_is_idempotent() {
        let (client_io, _radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        backend.connect().await.unwrap();
        backend.disconnect().await;
        backend.disconnect().await; // Drugi disconnect bez paniki
    }

    #[tokio::test]
    async fn icom_probe_is_read_only() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Probe wysyła tylko READ_FREQUENCY i READ_MODE
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD]);
            radio_io
                .write_all(&[
                    0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x00, 0x14, 0x07, 0x00, 0xFD,
                ])
                .await
                .unwrap();

            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], &[0xFE, 0xFE, 0x94, 0xE0, 0x04, 0xFD]);
            radio_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x04, 0x00, 0x01, 0xFD])
                .await
                .unwrap();
        });

        let state = backend.probe().await.expect("probe should succeed");
        assert_eq!(state.frequency_hz, 7_140_000);
        assert_eq!(state.mode, "LSB");
    }

    #[test]
    fn icom_profile_capabilities_are_consistent() {
        assert!(ICOM_IC7300.capabilities.frequency);
        assert!(ICOM_IC7300.capabilities.mode);
        assert!(ICOM_IC7300.capabilities.ptt);
        assert_eq!(ICOM_IC7300.default_ci_v_address, Some(0x94));
    }

    #[tokio::test]
    async fn icom_partial_reads_preserve_data() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = radio_io.read(&mut buf).await.unwrap();

            // Wysyłamy odpowiedź w dwóch oddzielnych kawałkach I/O
            radio_io.write_all(&[0xFE, 0xFE, 0xE0]).await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
            radio_io
                .write_all(&[0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD])
                .await
                .unwrap();
        });

        backend.connect().await.unwrap();
        backend
            .poll_frequency_for_test()
            .await
            .expect("poll_frequency with fragmentation");
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);
    }

    #[tokio::test]
    async fn icom_no_ptt_during_probe() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend =
            IcomCiVBackend::with_stream(client_io, test_config(), ICOM_IC7300.clone(), Some(0x94));

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            while let Ok(n) = radio_io.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                // Sprawdź, czy komenda nie jest komendą załączenia PTT (0x1C 0x00 0x01)
                assert!(
                    !(&buf[..n].windows(3).any(|w| w == [0x1C, 0x00, 0x01])),
                    "PTT activated during probe!"
                );
                if &buf[..n] == &[0xFE, 0xFE, 0x94, 0xE0, 0x03, 0xFD] {
                    radio_io
                        .write_all(&[
                            0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD,
                        ])
                        .await
                        .unwrap();
                } else if &buf[..n] == &[0xFE, 0xFE, 0x94, 0xE0, 0x04, 0xFD] {
                    radio_io
                        .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x04, 0x01, 0x01, 0xFD])
                        .await
                        .unwrap();
                }
            }
        });

        let state = backend.probe().await.expect("probe succeeds");
        assert!(!state.ptt, "state.ptt must be false during probe");
    }
}
