<h1 align="center">
  📻 SPLogbook
</h1>

<p align="center">
  <img src="https://img.shields.io/badge/License-GPLv3-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/Language-Rust-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Platform-Windows-blueviolet.svg" alt="Platform">
  <img src="https://img.shields.io/badge/Version-1.0.0-green.svg" alt="Version">
</p>

A modern, advanced logging software for amateur radio operators.
*Nowoczesny, zaawansowany program logujący dla krótkofalowców.*

---

## 📸 Screenshot

<img width="1911" height="988" alt="image" src="https://github.com/user-attachments/assets/a21c3e9d-b4c0-4ac7-8fcb-8e699dff1ec8" />


## ✨ Features

- **📝 QSO Logging:** Multi-journal support, ADIF import/export, PDF export, GPX export.
- **📻 Radio Integration:** CAT/Hamlib support, VFO control, PTT control, JS8Call, WSJT-X, and FLDigi integration.
- **🌐 DX Cluster:** Built-in Telnet client, RBN filter, auto-tune, and DX alerts.
- **🏆 Awards & Diplomas:** Tracking for DXCC, WAS, WAZ, SP DX, WAE, WWFF, RDA, IOTA, SOTA, POTA, and PGA.
- **🗺️ Maps:** Interactive world map with grey line, zoom, and QSO markers.
- **📊 Statistics & Charts:** Detailed metrics (QSO per month, bands, modes, hourly, countries, QSL tracking).
- **🏁 Contest Engine:** Support for SP DX, CQWW, ARRL DX, WPX, and VHF/UHF contests with live scoring and Cabrillo export.
- **☁️ Online Integration:** Synchronization with LoTW, eQSL, Club Log, QRZ.com, HRDLog, CloudLog, PSK Reporter, and WSPR.
- **🔌 REST API:** Built-in HTTP server on port 8080 with JSON endpoints for external integrations.
- **🌍 Multi-language:** Polish (PL), English (EN), Russian (RU), German (DE), French (FR), and Spanish (ES) support.
- **🔔 Alerts:** Sound alerts for new DXCC/IOTA and duplicate QSOs.

## 💻 Requirements

- **Operating System:** Windows 10/11 (64-bit)
- **Dependencies:** None! SPLogbook is statically linked and requires no additional runtime libraries.

## 🚀 Installation

You can either download a pre-built binary or compile it from source:

1. Navigate to the **[Releases](#)** page.
2. Download the latest `SPLogbook.exe`.
3. Run the executable (no installation required).

## 🛠️ Building from source

To build SPLogbook from source, ensure you have the Rust toolchain and MinGW-w64 installed:

```powershell
# Install Rust: https://rustup.rs
# Install MinGW-w64
cargo build --release
```

The compiled binary will be located in `target/release/SPLogbook.exe`.

## ⚙️ Configuration

SPLogbook stores its configuration files and local databases safely in your Windows AppData directory or directly in the program directory if configured as portable. The primary configuration is stored in a JSON format.

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+Shift+S` | Open Statistics window |
| `F1 - F12` | CW Macros |

## 📡 REST API Reference

The application exposes a local REST API on `http://127.0.0.1:8080/`.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/qso` | GET | Retrieve a list of recent QSOs |
| `/api/qso` | POST | Log a new QSO via JSON payload |
| `/api/radio/frequency` | GET | Get current VFO frequency |
| `/api/status` | GET | Check application status and version |

*(Note: API access must be enabled in the settings)*

## 📄 License

This project is licensed under the **GNU General Public License v3.0** (GPLv3). See the [LICENSE](LICENSE) file for details.

## ✍️ Author

**Mariusz Woźniak (SP6INA)**

## 🤝 Contributing

Contributions are welcome! If you'd like to improve SPLogbook, please follow these steps:
1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Ensure your code compiles and passes `cargo check` and `cargo clippy`.
4. Submit a Pull Request.
