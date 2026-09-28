# 📻 SPLogbook

<p align="center">
  <strong>Next-Generation Amateur Radio Logging & Station Control Suite</strong><br>
  <em>Engineered in Rust with Immediate-Mode GUI for Speed, Precision, and Reliability</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust_2021-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/GUI-egui_%2F_eframe-blueviolet.svg" alt="egui">
  <img src="https://img.shields.io/badge/License-GPLv3-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/Platform-Windows_%7C_GNU%2FLinux-blue.svg" alt="Platform">
  <img src="https://img.shields.io/badge/Version-1.1-emerald.svg" alt="Version">
  <a href="https://github.com/sp6ina/SPLogbook/actions"><img src="https://github.com/sp6ina/SPLogbook/actions/workflows/build-and-release.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/Tests-231%2F231_Passed-brightgreen.svg" alt="Tests">
  <img src="https://img.shields.io/badge/i18n-7_Languages-cyan.svg" alt="i18n">
  <a href="https://buycoffee.to/sp6ina"><img src="https://img.shields.io/badge/☕_Buy_Me_a_Coffee-buycoffee.to%2Fsp6ina-FFDD00?style=flat&logoColor=black" alt="Buy Me a Coffee"></a>
</p>

<img width="1917" height="991" alt="image" src="https://github.com/user-attachments/assets/465c5eda-739d-4191-ad17-c6cc89a39347" />

---

## 🌟 Executive Summary

**SPLogbook** is an advanced, high-performance amateur radio logging software and station automation console engineered from the ground up in **Rust**. Built with an immediate-mode user interface powered by `egui`/`eframe`, SPLogbook delivers instant sub-millisecond responsiveness, zero-garbage-collection pauses, complete memory safety, and cross-platform native performance across both **Windows 10/11 (64-bit)** and **GNU/Linux (x86_64, X11 & Wayland)** environments.

From deep ionospheric modeling (VOACAP-lite HF propagation), automated antenna rotator steering (`rotctld`), and real-time DX Cluster intelligence with acoustic All-Time New One (ATNO) alarms, to bi-directional digital modes bridging (WSJT-X, JS8Call, FLDigi), orbital satellite Doppler tracking, multi-award tracking, contest logging with live scoring, and professional A4 Avery QSL label vector PDF generation, SPLogbook provides the modern radio amateur with a unified, state-of-the-art operating environment. A dockable `egui_dock` workspace with floating multi-monitor windows, a sandboxed **Rhai** plugin system, a central event bus with live WebSocket streaming, a multi-backend CAT abstraction layer, and encrypted peer-to-peer log synchronization make it equally suited to the casual DXer, the serious contester, and the multi-station operator.

---

## 📑 Table of Contents

1. [Key Features Overview](#-key-features-overview)
2. [Deep Architectural & Functional Breakdown](#-deep-architectural--functional-breakdown)
   - [Core Logging Engine & Database Architecture](#1-core-logging-engine--database-architecture)
   - [VOACAP-lite HF Propagation Modeling](#2-voacap-lite-hf-propagation-modeling)
   - [Antenna Rotator Integration (rotctld)](#3-antenna-rotator-integration-rotctld)
   - [DX Cluster Intelligence & Acoustic Alerts](#4-dx-cluster-intelligence--acoustic-alerts)
   - [Specialty Ham Radio Clubs Integration](#5-specialty-ham-radio-clubs-integration)
   - [QSL Card Designer & Avery Label Sheet PDF Exporter](#6-qsl-card-designer--avery-label-sheet-pdf-exporter)
   - [Transceiver CAT & Hardware Supervision](#7-transceiver-cat--hardware-supervision)
   - [Orbital Satellite Tracking & Doppler Compensation](#8-orbital-satellite-tracking--doppler-compensation)
   - [Digital Modes Bridging (WSJT-X, JS8Call, FLDigi)](#9-digital-modes-bridging-wsjt-x-js8call-fldigi)
   - [Contest Engine & Cabrillo Exporter](#10-contest-engine--cabrillo-exporter)
   - [Award Programs Tracking Matrix](#11-award-programs-tracking-matrix)
   - [Cloud Services & Online QSL Synchronization](#12-cloud-services--online-qsl-synchronization)
   - [Interactive World Map & Real-Time Grey Line](#13-interactive-world-map--real-time-grey-line)
   - [Visual Analytics & Station Statistics](#14-visual-analytics--station-statistics)
   - [Embedded Local REST API Server](#15-embedded-local-rest-api-server)
   - [Internationalization (i18n)](#16-internationalization-i18n)
   - [Dockable Workspace & Operator Layout Profiles](#17-dockable-workspace--operator-layout-profiles)
   - [User Plugin System (Rhai) + Plugin Marketplace](#18-user-plugin-system-rhai--plugin-marketplace)
   - [CAT Abstraction Layer (Multi-Backend)](#19-cat-abstraction-layer-multi-backend)
   - [Central Event Bus & Live WebSocket](#20-central-event-bus--live-websocket)
   - [Callbook Aggregation & Offline Cache](#21-callbook-aggregation--offline-cache)
   - [Operator Assistant & Voice Keyer](#22-operator-assistant--voice-keyer)
   - [Encrypted P2P Log Synchronization](#23-encrypted-p2p-log-synchronization)
   - [N1MM Logger+ Compatibility](#24-n1mm-logger-compatibility)
3. [Keyboard Shortcuts Reference](#-keyboard-shortcuts-reference)
4. [REST API Documentation](#-rest-api-documentation)
5. [System Requirements](#-system-requirements)
6. [Installation & Quick Start](#-installation--quick-start)
7. [Building from Source](#-building-from-source)
8. [Codebase Architecture & Directory Structure](#-codebase-architecture--directory-structure)
9. [Configuration & Data Safety](#-configuration--data-safety)
10. [Contributing & Bug Reports](#-contributing--bug-reports)
11. [Support & Donations](#-support--donations)
12. [License & Credits](#-license--credits)

---

## 🚀 Key Features Overview

| Feature Area | Capabilities |
|---|---|
| **Database & Engine** | SQLite with WAL mode, 10 specialized indexes, auto-VACUUM, persistent window geometry, rolling 10-revision backup, fail-safe mutex poison recovery. |
| **HF Propagation** | VOACAP-lite ionospheric engine: Great-Circle midpoint, solar zenith, $foF2$, MUF/LUF/FOT, D-layer absorption, reliability index ($0	ext{--}100\%$), S-meter estimation, dynamic band opening matrix. |
| **Antenna Steering** | Hamlib `rotctld` TCP client (port 4533) with one-click short-path beam rotation in QSO Entry and World Map. |
| **DX Cluster** | Built-in Telnet client, 2 kHz / 30-spot sliding deduplication, visual highlights (⭐ Magenta for ATNO, ✨ Emerald for new band/mode), acoustic fanfare alerts, click-to-tune CAT integration. |
| **Ham Clubs Directory** | Real-time recognition and color-coded badge display for SP-OTC, SPCWC, SKCC, CWOPS, FOC, and HSC with member number recognition. |
| **QSL Vector PDF** | Full A4 vector PDF label sheet exporter for Avery 3x8, 3x7, 2x8, 2x7 formats, plus standalone QSL card visual designer. |
| **Station Profiles** | Multi-profile workstation management (Home QTH, Field /P, SOTA/POTA, Contest) with instant 1-click preset switching. |
| **Transceiver CAT & Sharing** | Unified **CAT abstraction layer** (`CatBackend` trait + `CatBackendKind`) over multiple rig backends: Hamlib `rigctld`, **FLRig (XML-RPC)**, **TCI (SDR)**, **Icom CI-V (serial)**, and **SO2R**; VFO A/B, split, RIT/XIT, mode, PTT, power, WinKeyer. Selectable **Bundled (Hamlib 4.7.2)** or **System-installed** source, plus integrated **Hamlib CAT TCP Proxy Server** (port 4534) for multi-app rig sharing. |
| **Digital Modes** | WSJT-X / JTDX bi-directional UDP bridge (port 2237), JS8Call TCP JSON API integration, FLDigi XML-RPC bridge. |
| **Satellites** | SGP4/SDP4 Keplerian orbital propagation from TLE, real-time Doppler shift frequency correction via CAT, antenna elevation/azimuth steering. |
| **SDR Waterfall & Spectrum** | Real-time FFT spectrum/waterfall panel capturing the radio's audio output (or any audio input) via `cpal`; selectable FFT sizes (512–4096), gain/floor/color-scale controls, input-device selection, and full docking support (dockable tile or floating multi-monitor window). |
| **Contests & Dupe Engine** | Rules for SP DX, CQ WW, ARRL DX, CQ WPX, IARU HF; structured **exchange parser** (`599 001 EU-115` → validated fields), live **rate meter (QSO/h)** + rate history, **multiplier matrix** per band, real-time inline `[DUPE!]` warning, N1MM-style keyboard-first auto-refocus, Duplicate Manager with batch cleaning, and **Cabrillo 3.0 / ADX (XML ADIF)** export. |
| **Awards Tracking** | DXCC (Mixed, Band, Mode, Challenge, ATNO), WAZ, WAS, SP DX Award, WAE, WWFF, RDA, PGA (2477 Polish municipalities), IOTA, SOTA, POTA. |
| **Cloud Sync** | LoTW (TQSL), eQSL.cc, Club Log, QRZ.com, HamQTH, HRDLog, Cloudlog, PSK Reporter, WSPR monitor, NOAA space weather, with a shared **upload scheduler** (offline queue, exponential-backoff retry, per-service rate limiting). |
| **Mapping** | Equirectangular world map with real-time day/night terminator (Grey Line), zoom/pan, confirmed/unconfirmed QSO markers, live spot pins. |
| **REST API & WebSocket** | Embedded asynchronous Axum HTTP server on port 8080 with JSON endpoints plus a live **WebSocket** stream (`/api/v1/ws`) broadcasting application events in real time for external integration and station automation. |
| **Logbook Table** | Virtualized, resizable-column data grid with multi-row selection, bulk delete, per-column sorting, live filtering, and saveable column presets. |
| **CSV Export** | Configurable logbook CSV exporter with the full `QsoRecord` field catalog, delimiter selection (comma/semicolon/tab), optional header row, and correct quoting/escaping of special characters. |
| **Dockable Workspace** | Full `egui_dock` docking system: panels can be detached into **floating native windows** (move to a second monitor), re-arranged into tabs/columns, and saved/restored via **operator layout profiles** with built-in presets. |
| **User Plugins (Rhai)** | Sandboxed embedded **Rhai** scripting engine: users write `.rhai` plugins with safe getters/actions and lifecycle hooks (`on_startup`, `on_qso_logged`, `on_dx_spot`, `on_rig_state`, …) — no filesystem/network access by default. |
| **Plugin Marketplace** | One-click-install add-on catalog (POTA/SOTA helpers, CW macros, contest assistant, rotor assistant, award tracker, DX spot alerts, and more) with SHA256 verification, offline fallback, and update/uninstall support. |
| **Event Bus** | Central `tokio::sync::broadcast` event bus decoupling modules; the same JSON events drive UI toasts, the WebSocket stream, and Rhai plugins. |
| **Operator Assistant & Voice Keyer** | Always-on decision bar recommending "what to do now" from propagation/CAT/DX/award goals, plus an SSB **voice keyer** (WAV playback, F1–F8 slots, CQ loop). |
| **Encrypted P2P Sync** | Direct peer-to-peer log synchronization over LAN/VPN encrypted with **XChaCha20-Poly1305** (Argon2id key derivation) — no cloud required. |
| **N1MM Compatibility** | Emits **N1MM Logger+ UDP broadcast** XML frames so GridTracker, overlay tools, and the N1MM ecosystem integrate out of the box. |
| **Callbook Aggregation** | Configurable callbook **source priority** (QRZ.com, HamQTH, offline cache) with automatic result merging and a persistent offline **cache** (TTL-based). |
| **Theming & Accessibility** | Central `theme.rs` palette with presets — **Operator Dark**, **Daylight**, **High-Contrast / colorblind-safe (Okabe-Ito)** — user font scaling and family selection, and a status legend window. |
| **Command Palette** | Searchable command launcher (`Ctrl+Shift+P`) for instant keyboard-first navigation to any function. |
| **Built-in Help** | In-app user manual (Help → Instrukcja obsługi), contextual "?" hints in complex panels, full changelog window, and GitHub update checker. |
| **Languages** | 6 complete native translations: English, Polish, German, French, Spanish, Russian (100% verified test coverage). |

---

## 🔍 Deep Architectural & Functional Breakdown

### 1. Core Logging Engine & Database Architecture
- **SQLite with WAL Mode:** The database backend operates in `WAL` (Write-Ahead Logging) mode with `PRAGMA synchronous=NORMAL;`, `PRAGMA cache_size=10000;`, and `PRAGMA temp_store=MEMORY;`. This architecture ensures concurrent non-blocking reads and writes, protecting against corruptions even during sudden power losses.
- **10 Performance-Optimized Indexes:** Rapid querying across multi-hundred-thousand QSO logs:
  - `idx_qso_callsign` (Callsign lookup)
  - `idx_qso_date` (Chronological ordering)
  - `idx_qso_band` & `idx_qso_mode` (Filtering)
  - `idx_qso_journal` (Multi-journal separation)
  - `idx_qso_dxcc` (Award calculation)
  - `idx_qso_lotw` & `idx_qso_eqsl` (Confirmation tracking)
  - `idx_qso_cqz` (CQ zone statistics)
  - `idx_qso_composite` (`callsign, band, mode` composite index for instant duplicate checking)
- **Automatic Health & Housekeeping:**
  - `vacuum_if_needed`: Automatically triggers `VACUUM;` every 1,000 logged QSOs to maintain minimal database fragmentation.
  - **Fail-Safe Mutexes:** Mutex locks (`log_db`, `awards_engine`, `scp_engine`) utilize poison-recovery patterns (`unwrap_or_else(|p| p.into_inner())`), preventing entire application crashes if an asynchronous worker encounters an error.
  - **Rolling Backups:** Automatic timestamped SQLite backup created on shutdown into `%APPDATA%/SPLogbook/backups/`, with automated pruning keeping the 10 most recent backups.
  - **Upload Retry Queue:** An embedded `upload_queue` table tracks failed cloud uploads (LoTW, eQSL, Club Log, QRZ, Cloudlog) with retry counters and backoff logging.
- **Persistent Window Geometry:** All 8 major dockable panels (`VFO`, `QSO Entry`, `Logbook Table`, `DX Cluster`, `BandMap`, `Solar Weather`, `Satellites`, `World Map`) persistently record their coordinates and dimensions across sessions in `station_config.json`.

---

### 2. VOACAP-lite HF Propagation Modeling
SPLogbook features a self-contained ionospheric HF propagation engine (`src/core/propagation.rs`) that models radio wave refraction through the ionosphere without requiring external Python or web dependencies:
- **Great-Circle Trigonometry:** Computes the shortest distance and bearing between the home station and the remote DX Maidenhead locator using spherical trigonometry.
- **Path Midpoint & Solar Zenith:** Pinpoints the exact geographic coordinates of the path midpoint and determines the solar zenith angle ($\chi$) to establish whether the midpoint is in daylight, darkness, or traversing the grey line.
- **Critical Frequencies & Limits:**
  - Calculates the ordinary ray critical frequency $foF2$ dynamically based on the Solar Flux Index (SFI) and sun angle:
    $$	ext{foF2} = 3.5 + 4.5 \cdot \sqrt{rac{	ext{SFI}}{100}} \cdot \max(0.1, \cos\chi)$$
  - Derives the Maximum Usable Frequency (**MUF**), Frequency of Optimum Traffic (**FOT** = $0.85 	imes 	ext{MUF}$), and Lowest Usable Frequency (**LUF**).
- **D-Layer Absorption:** Models daytime signal attenuation based on the geomagnetic $K$-index and solar elevation.
- **Propagation Mode Identification:**
  - **Groundwave:** For short paths ($< 120	ext{ km}$).
  - **Ionospheric $F_2$ Layer:** For long-distance single/multi-hop propagation.
  - **Sporadic-E ($E_s$):** Estimated during elevated summer solar conditions on 10m/6m.
- **Live User Feedback:**
  - Formularz QSO renders a real-time badge: `📡 Propagacja: XX% REL · SX (F2) · MUF XX.X MHz` with detailed hover tooltips.
  - Solar Panel provides a dynamic HF band opening matrix (160m to 10m) classifying current conditions as *Closed*, *Marginal*, *Good*, or *Excellent*.

---

### 3. Antenna Rotator Integration (`rotctld`)
- **Hamlib Rotator Protocol Client:** Built-in network client connecting to Hamlib's `rotctld` daemon on TCP port `4533`.
- **One-Click Beam Steering:**
  - **QSO Entry Panel:** Displays the calculated great-circle bearing and features an interactive **`🔄 Obróć ({azimuth}°)`** button that commands the rotor to swing to the target heading.
  - **World Map Panel:** A dedicated rotation control appears next to selected DX coordinates, allowing instantaneous antenna pointing directly from the map.
- **Dual-Path Capability:** Computes both Short Path (SP) and Long Path (LP) bearings ($180^\circ$ inversion).

---

### 4. DX Cluster Intelligence & Acoustic Alerts
- **High-Resilience Telnet Client:** Multi-threaded asynchronous Telnet client handling automated reconnects, terminal keep-alive, ANSI escape code filtering, and non-blocking teardown upon application close.
- **Sliding Deduplication Buffer:** Automatically filters out redundant cluster spots within a 30-spot / $2	ext{ kHz}$ window, preventing cluster spam from obscuring rare DX.
- **Visual Entity Highlighting:**
  - ⭐ **Magenta / Fuchsia:** **ATNO (All-Time New One)** — An entity never worked on any band or mode in the entire station history.
  - ✨ **Emerald Green:** New band or new mode for a previously confirmed DXCC country.
- **Acoustic Synthesizer:** Multi-threaded sound engine powered by `rodio` that plays a triumphant musical fanfare (440 Hz + 660 Hz dual-tone sequence) when an ATNO spot arrives.
- **Transceiver Click-to-Tune:** Clicking any spot in the cluster list or band map commands Hamlib CAT to immediately tune the transceiver VFO to the spot frequency.
- **Reverse Beacon Network (RBN):** Dedicated filter to toggle between human spotters and automated CW/FT8 skimmer spots.

---

### 5. Specialty Ham Radio Clubs Integration
- **Automated Directory Lookup:** As you type a callsign in the QSO entry field, SPLogbook checks an embedded database of prestigious international and national amateur radio clubs:
  - **SP-OTC** (SP Old Timers Club)
  - **SPCWC** (SP CW Club)
  - **SKCC** (Straight Key Century Club)
  - **CWOPS** (CW Operators' Club)
  - **FOC** (First Class CW Operators' Club)
  - **HSC** (High Speed Club)
- **Visual Badging:** Displays styled badges (e.g. `🎖 SP-OTC #248`, `🎖 SKCC #18942`) indicating club membership and member numbers, essential for club award endorsements and specialty on-air operating.

---

### 6. QSL Card Designer & Avery Label Sheet PDF Exporter
- **Vector PDF Generation Engine:** High-precision PDF rendering powered by `printpdf`.
- **Avery Standard Label Sheets (A4):** Built-in geometry templates for industry-standard self-adhesive printable sticker sheets:
  - **3 × 8** (24 labels per A4 sheet, $70.0 	imes 37.0	ext{ mm}$)
  - **3 × 7** (21 labels per A4 sheet, $70.0 	imes 42.4	ext{ mm}$)
  - **2 × 8** (16 labels per A4 sheet, $105.0 	imes 37.0	ext{ mm}$)
  - **2 × 7** (14 labels per A4 sheet, $105.0 	imes 42.4	ext{ mm}$)
- **Automated QSO Population:** Automatically formats and paginates selected QSOs with Callsign, Date, Time UTC, Band, Mode, RST Sent/Received, and QSL confirmation text onto sticker grids with exact printer margins.
- **Print Preview & Printer Calibration:** Live A4 sheet preview with adjustable left/top margins (in mm) so labels align precisely with physical sticker fields before export.
- **Batch Card-Selection Rules:** Queue cards by QSL-sent status (unconfirmed only), band, mode, date range, and a configurable count limit.
- **Pre-Export Validation:** The designer validates a non-empty print queue and station callsign before generating the PDF.
- **Interactive QSL Card Designer:** Visual editor for previewing custom QSL layouts, positioning elements, and preparing cards for print.

---

### 7. Transceiver CAT & Hardware Supervision
- **Hamlib `rigctld` Client & Lifecycle Supervisor:**
  - **Bundled vs System Hamlib Source Selection:** Operators can choose between the pre-packaged **Bundled Hamlib 4.7.2** (included out-of-the-box with both Windows `.zip` and Linux `.tar.gz` releases) or a **System-Installed** Hamlib binary (`/usr/bin/rigctld` or system `PATH`).
  - **Live Auto-Detection & Path Verification:** The settings window dynamically verifies the presence and exact path of the selected binary with visual status badges (`✔ Detected` or `⚠ Not Found`), and offers an optional custom binary path field.
  - **Self-Contained Linux Portability:** Linux runtime automatically manages dynamic library paths (`LD_LIBRARY_PATH` and RPATH `$ORIGIN/hamlib/lib`), guaranteeing that bundled shared libraries (`libhamlib.so`) load seamlessly without requiring external packages or system root privileges.
  - **Smooth Lifecycle Management:** Background supervisor launches `rigctld` with concealed console flags (`CREATE_NO_WINDOW` on Windows), checks PID liveness non-blockingly, and performs graceful termination on application exit.
  - Supports bidirectional polling of VFO frequency, mode, passband, and split status.
- **PTT & On-Air Control:** On-screen PTT toggle transmitting Hamlib `T 1` / `T 0` commands with visual TX/RX indication.
- **CW Keyer & Macro Terminal:**
  - 12 fully customizable CW macros mapped to function keys `F1`–`F12` with speed control (WPM) and live transmit buffer.
  - Hardware support for K1EL WinKeyer protocol over serial interfaces.
  - Expert Electronics TCI protocol client support.

---

### 8. Orbital Satellite Tracking & Doppler Compensation
- **SGP4/SDP4 Keplerian Propagator:** High-precision orbital tracker utilizing NORAD Two-Line Element (TLE) datasets with automated online updating.
- **Real-Time Orbit Calculations:** Instantaneous computation of Satellite Azimuth, Elevation, Range, Slant Rate, Footprint, and Sub-Satellite Point.
- **Automated Doppler CAT Control:** Compensates for the Doppler effect on transponder uplinks and downlinks by computing real-time frequency shifts:
  $$\Delta f = - f_0 rac{v_{	ext{slant}}}{c}$$
  and adjusting transceiver frequencies dynamically via CAT.
- **Antenna Elevation/Azimuth Rotator Steering:** Sends real-time azimuth and elevation tracking coordinates to dual-axis satellite rotators via `rotctld`.

---

### 9. Digital Modes Bridging (WSJT-X, JS8Call, FLDigi)
- **WSJT-X & JTDX (UDP Port 2237):**
  - Decodes binary UDP frames: Heartbeat, Status, Decode, Clear, QSO Logged, Close, and WSPR.
  - Automatically logs completed FT8/FT4 QSOs into SPLogbook with precise signal reports.
  - Queries the SPLogbook database in real-time to highlight whether a calling station is an unworked country, unworked band, or unworked mode.
- **JS8Call (TCP Port 2237):**
  - Connects to JS8Call's JSON API socket.
  - Ingests `RX.ACTIVITY`, `RX.DIRECTED`, and `RIG.FREQ` packets to log keyboard-to-keyboard contacts seamlessly.
- **FLDigi (XML-RPC):**
  - Direct integration for PSK31, RTTY, Olivia, and CW decoding.

---

### 10. Contest Engine & Cabrillo Exporter
- **Built-in Contest Rules & Multiplier Tracking:**
  - **SP DX Contest:** 3 points per QSO with non-SP stations; multipliers per Polish voivodeship / foreign country.
  - **CQ World Wide DX (CQ WW):** 1/2/3 points depending on station continent; CQ Zone and DXCC country multipliers.
  - **ARRL International DX:** 3 points per QSO; multipliers per US State and Canadian Province.
  - **CQ WPX Contest:** Points based on band and continent; prefix multipliers.
  - **VHF/UHF Polish National Contests:** 1 point per kilometer of great-circle distance; gridsquare multipliers.
- **Structured Exchange Parser:** Each contest rule declares an ordered list of exchange fields (RST, serial, CQ/ITU zone, US/VE state, grid, power, IOTA, name, QTH, etc.); the parser (`src/core/exchange.rs`) converts a typed string such as `599 001 EU-115` into validated, structured fields ready to store in the QSO record.
- **Live Scoring, Rate Meter & Multiplier Matrix:** Displays current score, multiplier breakdown, band-by-band QSO counts, a rolling operating rate (QSO/h) with rate history, and a **per-band multiplier matrix** highlighting needed multipliers.
- **Dupe Checking:** Immediate visual warning (red highlight and bold "DUPE!" tag) when entering an already-worked station on the current band/mode.
- **Cabrillo 3.0 & ADX Export:** Generates fully compliant Cabrillo log files formatted according to official contest sponsor specifications, plus **ADX (XML ADIF 3.1.5)** export.

---

### 11. Award Programs Tracking Matrix
SPLogbook features an integrated awards matrix (`src/core/awards.rs`) tracking confirmed and worked status across multiple international award programs:
- **DXCC:** Mixed, Band (160m–10m), Mode (CW, Phone, Digital), and DXCC Challenge.
- **WAZ (Worked All Zones):** Tracking across all 40 CQ Zones.
- **WAS (Worked All States):** Tracking across all 50 US States.
- **SP DX Award:** Polish districts (SP1 through SP9, SO, SN).
- **WAE (Worked All Europe):** Tracking across all 50+ European DXCC entities.
- **WWFF (World Wide Flora & Fauna):** Protected natural site references.
- **RDA (Russian Districts Award):** Regional district tracking.
- **PGA (Polska Gmina Award):** Comprehensive database of all 2,477 Polish municipalities.
- **IOTA (Islands On The Air):** Island reference tracking.
- **SOTA & POTA:** Summits and Parks on the air tracking with specialized SOTA CSV exports.

---

### 12. Cloud Services & Online QSL Synchronization
- **ARRL LoTW (Logbook of the World):** Direct TQSL command-line invocation for digital certificate signing, automated ADIF export, and inbound `.adi` download and reconciliation.
- **eQSL.cc:** Real-time upload of logged QSOs and inbound verification synchronization.
- **Club Log:** Instantaneous real-time QSO upload via API and Online QSL Request System (OQRS) tracking.
- **Callbook XML Lookups:** Automated XML lookup via **QRZ.com** (XML subscription) and **HamQTH** (free XML API) retrieving operator name, QTH address, Maidenhead grid, country, and biography photo, aggregated by a configurable **source priority** and backed by a persistent **offline cache**.
- **HRDLog & Cloudlog:** Automated cloud log synchronization with an embedded SQLite retry queue ensuring reliable delivery during internet drops.
- **Shared Upload Scheduler:** A deterministic scheduler (`src/cloud/scheduler.rs`) centralizes the offline queue, exponential-backoff retry, and per-service rate limiting for Club Log, QRZ.com, and eQSL.cc.
- **PSK Reporter & WSPR Monitor:** Submits live reception reports to `pskreporter.info` and queries `wspr.live` for real-time propagation probes.
- **Space Weather (NOAA SWPC):** Fetches real-time Solar Flux Index (SFI), Sunspot Number (SSN), geomagnetic $A$-index, $K$-index, X-ray flux, and solar wind velocity.

---

### 13. Interactive World Map & Real-Time Grey Line
- **High-Precision Equirectangular Projection:** Accurately renders continental outlines, country borders, and geographic latitude/longitude grids.
- **Dynamic Grey Line Terminator:** Computes the solar sub-solar point in real-time from the Earth's axial tilt and current UTC timestamp, drawing the day/night boundary.
- **Interactive Navigation:** Smooth mouse-wheel zooming ($1.0	imes$ to $5.0	imes$) and drag-to-pan exploration.
- **QSO & Cluster Spot Pins:**
  - Color-coded QSO pins: Green for LoTW/eQSL confirmed, Blue for standard logged QSOs.
  - Live DX cluster spot markers displaying callsign, frequency, band, and time on hover.
  - Short-path and long-path great-circle propagation tracks drawn between home station and remote stations.
  - Interactive beam heading steering directly from the map.

---

### 14. Visual Analytics & Station Statistics
SPLogbook includes a visual analytics dashboard providing comprehensive insights into your amateur radio activity:
- **QSO per Month:** 24-month historical trend chart.
- **Band Distribution:** Percentage breakdown of activity across all amateur bands ($160	ext{m}$ to $70	ext{cm}$).
- **Mode Breakdown:** Distribution across CW, SSB, FT8, FT4, RTTY, and FM.
- **Hourly UTC Activity:** 24-hour histogram identifying peak operating hours.
- **Top 10 Countries:** Ranking of most frequently contacted DXCC entities.
- **QSL Confirmation Ratios:** Comparative analysis between paper QSL, LoTW, and eQSL confirmations.
- **Interactive Drill-Down:** Every chart is clickable — click a month, band, mode, hour, country, or QSL-status segment to instantly open the logbook filtered to that subset, then return with one click.

---

### 15. Embedded Local REST API Server
SPLogbook features a built-in, lightweight asynchronous HTTP server powered by **Axum** running on port `8080`. This enables external software, custom scripts, web dashboards, or home automation systems to interact with the logbook:

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/v1/status` | Returns application health, version, station callsign, total QSOs, and uptime. |
| `GET` | `/api/v1/qsos` | Returns a paginated JSON list of logged QSOs (`limit`, `offset`). |
| `GET` | `/api/v1/qsos/{id}` | Returns complete details for a specific QSO record. |
| `POST` | `/api/v1/qsos` | Logs a new QSO record via JSON payload with full schema validation. |
| `GET` | `/api/v1/stats` | Returns aggregated station statistics (by band, by mode, QSL counts). |
| `GET` | `/api/v1/cluster/spots` | Returns active DX cluster spots in real-time. |
| `WS` | `/api/v1/ws` | WebSocket streaming live application events (`QsoLogged`, `DxSpot`, `RigState`, `ClusterStatus`, `CloudSync`, `Toast`) as JSON. |

---

### 16. Internationalization (i18n)
SPLogbook provides full multi-language support across all menus, toolbars, settings dialogs, and error notifications. Languages can be switched on the fly without restarting the application:
- 🇬🇧 **English** (EN)
- 🇵🇱 **Polski** (PL)
- 🇩🇪 **Deutsch** (DE)
- 🇫🇷 **Français** (FR)
- 🇪🇸 **Español** (ES)
- 🇷🇺 **Русский** (RU)

---

### 17. Dockable Workspace & Operator Layout Profiles
SPLogbook replaces the fixed panel grid with a fully **dockable workspace** powered by `egui_dock`:
- **Floating Native Windows:** Any panel (`VFO`, `QSO Entry`, `Logbook Table`, `DX Cluster`, `BandMap`, `World Map`, etc.) can be **undocked** from the main window into its own floating window — drag it to a second monitor for a multi-screen contest or field setup.
- **Flexible Arrangement:** Panels snap into tabs, side-by-side columns, and resizable split panes; the layout is serialized and restored across sessions.
- **Operator Layout Profiles:** Save, name, rename, and instantly recall complete workspace layouts (panel visibility, position, size, docking, and floating-window geometry) from a dedicated **Workspace** menu or the profile manager. Each profile can also bind a **color theme**, optional CAT/CW profile names, and an enabled-plugin list (`workspace_profiles.rs`).
- **Built-in Presets:** Seven one-click layouts — **Normalny DX**, **Kontest**, **Cyfrowe (FT8/WSJT-X)**, **POTA**, **SOTA**, **Satelity**, and **EME (Księżyc)** — each preconfigured for its operating scenario (e.g. daylight theme for field ops, high-contrast for contesting).
- **Shareable `.spws` Files:** Export any workspace to a versioned `.spws` JSON file and import it on another installation — ideal for sharing field, contest, or club setups.

---

### 18. User Plugin System (Rhai) + Plugin Marketplace
SPLogbook embeds the **Rhai** scripting language as a safe extension mechanism, letting operators automate and personalize the station without touching Rust:
- **Sandboxed by default:** scripts have no filesystem, network, or process access; engine limits (max string size, max expression depth) guard against runaway scripts.
- **Safe API (getters):** `qso_count()`, `my_call()`, `rig_freq_mhz()`, `rig_mode()`, `rig_connected()`, `rotor_azimuth()`, `rotor_elevation()`, `dxcc_worked()`, `dxcc_confirmed()`, `waz_worked()`, `was_worked()`, `wac_worked()`, `iota_worked()`, `pota_worked()`, `sota_worked()`, `pga_worked()`, and `qso_field(name)` (reads `callsign`, `band`, `mode`, `freq_mhz`, `name`, `qth`, `gridsquare`, `country`, `dxcc`, `sota_ref`, `pota_ref`, `pga_ref`, `iota`, `state`, `rst_sent`, `rst_rcvd`, `comment`).
- **Safe API (actions):** `log(msg)`, `notify(msg)`, `send_cw(text)`, `send_voice(text)`, `rotate(azimuth_deg[, elevation_deg])`, `spot(dx_call, freq_khz, comment)`, `set_qso_field(name, value)`, `play_sound(name)` (`new_dxcc`, `duplicate`, `new_iota`, `qso_saved`, `band_opened`), `pota_lookup(reference)`, and `sota_lookup(reference)`.
- **Lifecycle hooks:** `on_startup()`, `on_qso_logged(call_sign, band, mode, freq_mhz, is_atno)`, `on_band_opened(band)`, `on_workspace_changed(name)`, `on_dx_spot(spotter, dx_call, freq_khz, band, comment, is_ft8)`, `on_rig_state(freq_mhz, mode, connected)`, `on_pota_info(reference, name, active)`, and `on_sota_info(reference, name, points)` fire automatically on matching events.
- **Command bridge:** plugin actions are queued (`PluginCommand`) and executed by the app each frame, so sandboxed scripts can safely drive the CW keyer, voice keyer, rotor, local spots, form fields, sounds, and POTA/SOTA lookups without touching hardware directly (`plugins/bridge.rs`).
- **Hot reload:** `.rhai` files are loaded from the plugins directory; malformed scripts are reported (never crash) and can be enabled/disabled at runtime (`plugin_manager.rs`).
- **Plugin Marketplace:** a one-click-install catalog (`plugins/marketplace.rs`) with a built-in offline fallback of add-ons — **POTA Helper**, **SOTA Helper**, **CW Macros**, **Contest Assistant**, **Rotor Assistant**, **Award Tracker**, **DX Spot Alerts (ATNO)**, **Propagation Watchdog**, **Band Activity Logger**, **QSL Reminder**, **Voice Keyer Trigger**, and **FT8/WSJT-X Bridge**. Remote catalogs are fetched from GitHub Releases, installs are SHA256-verified, and sidecar manifests enable status detection (installed / update available) with one-click install, update, and uninstall (`gui/marketplace.rs`).

---

### 19. CAT Abstraction Layer (Multi-Backend)
A unified `CatBackend` trait and `CatBackendKind` enum (Hamlib, FLRig, TCI, Icom CI-V, SO2R) abstract away the hardware protocol so the rest of the application talks to **one `RigState`**, regardless of radio:
- **Hamlib `rigctld`** — the default TCP backend.
- **FLRig (XML-RPC)** — cross-platform rig control via FLDigi's companion server.
- **TCI** — Transceiver Control Interface for SDR platforms (SunSDR, ExpertSDR, Thetis).
- **Icom CI-V** — direct serial control of Icom transceivers.
- **SO2R** — single-operator two-radio logic with TX lockout.
- A `CatController` dispatcher selects the active implementation at runtime; unimplemented operations return descriptive errors so new backends can be added incrementally.

---

### 20. Central Event Bus & Live WebSocket
A central **event bus** (`src/core/events.rs`, built on `tokio::sync::broadcast`) publishes typed domain events — `QsoLogged`, `DxSpot`, `RigState`, `ClusterStatus`, `CloudSync`, `Toast` — consumed by multiple subscribers without coupling:
- **UI:** toasts, status bar, and notifications.
- **WebSocket:** `GET /api/v1/ws` streams the same JSON-serialized events live to external dashboards and automation.
- **Plugins:** Rhai hooks are driven by the same lifecycle events.

---

### 21. Callbook Aggregation & Offline Cache
Callbook lookups merge results from multiple sources according to a user-configurable **priority order** (`QRZ.com` → `HamQTH` → offline cache), so the richest available data wins:
- **Source priority** is reorderable in the Online Synchronization settings.
- **Offline cache:** successful online lookups are persisted to a local SQLite `CallbookCache` table with a configurable TTL, enabling instant lookups without internet and reduced API quota usage.
- **Local callbook** (bundled `databases/`) remains the zero-latency first line of resolution.

---

### 22. Operator Assistant & Voice Keyer
- **Operator Assistant:** a pure, unit-tested `recommend()` function produces an always-available decision bar that answers "what should I do now?" using live propagation forecasts, current band status, CAT state, DX cluster activity, and award goals.
- **SSB Voice Keyer:** plays pre-recorded WAV announcements (CQ calls, replies) through `rodio`, mapped to F1–F8 slots with loop and stop controls for hands-free contest operation.

---

### 23. Encrypted P2P Log Synchronization
For operators who want multi-station or field/backup synchronization without a cloud service, SPLogbook provides a **direct peer-to-peer sync** (`src/sync/p2p.rs`) over TCP:
- **Encrypted transport:** each frame is sealed with **XChaCha20-Poly1305**, with the symmetric key derived from a shared passphrase via **Argon2id**.
- **Frame integrity:** length-prefixed frames with a per-message nonce and 64 MB size cap protect against truncation and memory-exhaustion.
- Works over LAN or VPN, no third-party server involved.

---

### 24. N1MM Logger+ Compatibility
SPLogbook emits **N1MM Logger+ UDP broadcast** XML frames (loopback port `12060`), making it a drop-in data source for the wide ecosystem of N1MM companion tools — GridTracker, overlay utilities, and contest peripheral software — with no additional plugins.

---

## ⌨️ Keyboard Shortcuts Reference

| Shortcut | Context | Action |
|---|---|---|
| `Ctrl + Z` | Global | Undo last QSO operation |
| `Ctrl + Y` | Global | Redo last undone operation |
| `Ctrl + Shift + S` | Global | Open Statistics & Analytics Dashboard |
| `F1` – `F12` | CW Keyer | Transmit pre-programmed CW macros |
| `Esc` | QSO Entry | Clear all input fields in QSO Entry panel |
| `Return` / `Enter` | QSO Entry | Save entered QSO to active journal |
| `Space` | QSO Entry | Advance cursor to next field (Callsign $	o$ RST $	o$ Comments) |

---

## 📡 REST API Documentation

### Example 1: Querying Station Status
```bash
curl -X GET http://127.0.0.1:8080/api/v1/status
```
**Response:**
```json
{
  "status": "online",
  "version": "1.0.0",
  "callsign": "SP6INA",
  "total_qsos": 14250,
  "uptime_seconds": 3600
}
```

### Example 2: Ingesting a New QSO
```bash
curl -X POST http://127.0.0.1:8080/api/v1/qsos \
  -H "Content-Type: application/json" \
  -d '{
    "callsign": "W1AW",
    "qso_date": "2026-09-21",
    "time_on": "18:30:00",
    "band": "20m",
    "mode": "CW",
    "rst_sent": "599",
    "rst_rcvd": "599",
    "country": "United States"
  }'
```

---

### 💻 System Requirements

- **Operating Systems:**
  - **Windows:** Windows 10 (64-bit) or Windows 11 (64-bit).
  - **GNU/Linux:** Any modern 64-bit distribution (Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch Linux, openSUSE). Full native support for both **X11** and **Wayland** display servers.
- **Architecture:** x86_64.
- **Processor:** Any modern dual-core CPU ($\ge 1.6\text{ GHz}$).
- **Memory (RAM):** 2 GB minimum (4 GB recommended).
- **Disk Space:** ~50 MB for the binary, configuration, and reference databases.

---

## 🚀 Installation & Quick Start

Pre-compiled release packages are available directly from the [Releases](https://github.com/sp6ina/SPLogbook/releases) page:

### 🪟 Windows (x64)
1. Download **`SPLogbook-Windows-x64.zip`** from the latest release.
2. Extract the archive to any desired directory (e.g. `C:\Radio\SPLogbook\`).
3. Double-click **`SPLogbook.exe`** to launch.

### 🐧 GNU/Linux (x86_64)
1. Download **`SPLogbook-Linux-x86_64.tar.gz`** from the latest release.
2. Extract the archive and enter the directory:
   ```bash
   tar -xvf SPLogbook-Linux-x86_64.tar.gz
   cd SPLogbook-Linux-x86_64
   ```
3. Grant access to your serial ports (for CAT & Winkeyer interfaces):
   ```bash
   sudo usermod -aG dialout $USER   # Ubuntu / Debian / Mint
   # or
   sudo usermod -aG uucp $USER      # Arch Linux / Manjaro
   ```
4. Run the portable launcher:
   ```bash
   ./run.sh
   ```
   *(Optional)* Copy `assets/splogbook.desktop` to `~/.local/share/applications/` to integrate with your desktop application launcher.

---

## 🛠️ Building from Source

Ensure you have the Rust toolchain (version $\ge 1.80$) installed.

### On Windows
```powershell
# 1. Clone the repository
git clone https://github.com/sp6ina/SPLogbook.git
cd SPLogbook

# 2. Run automated unit tests (205 tests)
cargo test

# 3. Build optimized release binary with Link-Time Optimization (LTO)
cargo build --release
```

### On GNU/Linux (Ubuntu / Debian / Mint)
```bash
# 1. Install development dependencies
sudo apt update && sudo apt install -y \
  pkg-config libasound2-dev libx11-dev libxcursor-dev libxrandr-dev \
  libxi-dev libxkbcommon-dev libxkbcommon-x11-dev libgl1-mesa-dev \
  libfontconfig1-dev libwayland-dev libgtk-3-dev

# 2. Clone and build
git clone https://github.com/sp6ina/SPLogbook.git
cd SPLogbook
cargo test
cargo build --release
```

The compiled standalone binary will be located at:
- Windows: `target/release/SPLogbook.exe`
- Linux: `target/release/SPLogbook`

---

## 📁 Codebase Architecture & Directory Structure

```
SPLogbook/
├── .github/
│   └── workflows/
│       └── build-and-release.yml # Multi-platform CI/CD (Ubuntu + Windows)
├── Cargo.toml               # Project manifest, dependencies, release profile (LTO, strip)
├── build.rs                 # Windows resource compilation (application icon, metadata)
├── Bin/
│   └── Windows/
│       └── SPLogbook.exe    # Deployed 64-bit standalone release binary
├── databases/               # Reference databases (CTY.DAT, callbooks, PGA, clubs)
├── assets/                  # Graphical icons, desktop shortcut, and portable launcher
│   ├── icon.ico / icon.png  # Application branding icons
│   ├── splogbook.desktop    # Freedesktop Linux application launcher
│   └── run.sh               # Linux portable environment launcher
└── src/
    ├── main.rs              # Application entry point, XDG/AppData path resolution, crash guard
    ├── lib.rs               # Library root exposing core modules
    ├── api/                 # Local REST API server (Axum, port 8080)
    │   ├── mod.rs
    │   └── server.rs
    ├── cat/                 # Hardware control & rig supervision
    │   ├── backend.rs       # CAT abstraction layer (CatBackend trait + CatBackendKind)
    │   ├── flrig.rs         # FLRig XML-RPC client
    │   ├── hamlib.rs        # Asynchronous TCP client for rigctld
    │   ├── icom_ci_v.rs     # Icom CI-V serial protocol client
    │   ├── rig_models.rs    # Database of transceiver models
    │   ├── rotor.rs         # Hamlib rotctld client (TCP port 4533)
    │   ├── server.rs        # Hamlib TCP proxy server (CAT sharing for WSJT-X/FLDigi)
    │   ├── so2r.rs          # Single-operator two-radio (SO2R) logic
    │   ├── supervisor.rs    # Non-blocking rigctld subprocess supervisor
    │   ├── tci.rs           # Expert Electronics TCI protocol client
    │   └── winkeyer.rs      # K1EL WinKeyer serial keyer protocol
    ├── cloud/               # Online services & cloud synchronization
    │   ├── cloudlog.rs      # Cloudlog REST API sync
    │   ├── clublog.rs       # Club Log real-time upload & OQRS
    │   ├── eqsl.rs          # eQSL.cc submission & confirmation
    │   ├── hamqth.rs        # HamQTH XML callbook lookup
    │   ├── hrdlog.rs        # HRDLog.net synchronization
    │   ├── lotw.rs          # ARRL LoTW TQSL integration
    │   ├── psk_reporter.rs  # PSK Reporter HTTP XML spotting
    │   ├── qrz.rs           # QRZ.com XML Logbook subscription client
    │   ├── scheduler.rs     # Shared upload scheduler (offline queue, retry, rate-limit)
    │   ├── solar.rs         # NOAA SWPC space weather data parser
    │   ├── updater.rs       # Automatic GitHub releases updater
    │   └── wspr.rs          # wspr.live REST API monitor
    ├── cluster/             # Spotting networks
    │   ├── lan_sync.rs      # Multi-operator LAN synchronization
    │   └── telnet.rs        # Multi-threaded Telnet client for DX Cluster
    ├── core/                # Core business logic & database
    │   ├── adif.rs          # ADIF 3.1.4 parser & exporter (+ ADX XML export)
    │   ├── astronomy.rs     # Solar & lunar ephemeris, solar zenith calculation
    │   ├── awards.rs        # Awards tracking engine (DXCC, WAZ, WAS, PGA, etc.)
    │   ├── backup.rs        # Database rolling backup manager (10 revisions)
    │   ├── bandplan.rs      # IARU Region 1/2/3 band plans
    │   ├── callbook.rs      # Callbook aggregation (source priority + offline cache)
    │   ├── callsign_correction.rs # Callsign typo correction & suggestions
    │   ├── clubs.rs         # Specialty ham radio clubs directory (SP-OTC, SKCC, etc.)
    │   ├── contest_rules.rs # Contest scoring engine & Cabrillo exporter
    │   ├── contest_stats.rs # Contest rate meter & multiplier matrix
    │   ├── credentials.rs   # OS credential store (keyring) integration
    │   ├── csv_export.rs    # Configurable logbook CSV export formatter
    │   ├── database.rs      # SQLite WAL backend, 10 indexes, upload queue
    │   ├── database_stats.rs# Aggregated statistics queries
    │   ├── events.rs        # Central event bus (tokio::sync::broadcast)
    │   ├── exchange.rs      # Contest exchange parser & validation
    │   ├── geo.rs           # Maidenhead grid converter, spherical trigonometry
    │   ├── http.rs          # Shared reqwest HTTP client (timeouts/retry)
    │   ├── i18n.rs          # Translation dictionary & lookup (6 languages)
    │   ├── i18n_tr.rs       # Translation strings data (include_str!)
    │   ├── pga.rs           # Polska Gmina Award logic
    │   ├── pga_data.rs      # PGA gmina database (2477 gminas, data)
    │   ├── prefix.rs        # ITU prefix allocations & DXCC country mapping
    │   ├── propagation.rs   # VOACAP-lite HF ionospheric propagation modeling
    │   ├── propagation_history.rs # Propagation forecast history
    │   ├── qsl_print.rs     # QSL card geometry & Avery label calculations
    │   ├── qso.rs           # QSO record data structures & validation
    │   ├── satellite.rs     # SGP4/SDP4 satellite orbital propagation
    │   ├── scp.rs           # Super Check Partial database lookup
    │   ├── service_db.rs    # Equipment ledger & station inventory
    │   ├── sota_export.rs   # SOTA CSV export formatter
    │   └── station.rs       # Station profile & workspace/dock profiles
    ├── digital/             # Digital mode bridges
    │   ├── fldigi.rs        # FLDigi XML-RPC client
    │   ├── js8call.rs       # JS8Call TCP JSON API integration
    │   ├── n1mm.rs          # N1MM Logger+ UDP broadcast emitter
    │   └── wsjtx.rs         # WSJT-X / JTDX binary UDP frame decoder
    ├── gui/                 # Immediate-mode egui user interface
    │   ├── advanced_filter.rs# Multi-criteria logbook search & filter
    │   ├── app.rs           # Main application state, event loops, hotkeys
    │   ├── app_export.rs    # Log export (Cabrillo, ADX, PDF, GPX)
    │   ├── app_layout.rs    # egui_dock layout, docking & panel management
    │   ├── astronomy_dialog.rs # Celestial ephemeris modal
    │   ├── awards_matrix.rs # Interactive award matrix viewer
    │   ├── bandmap.rs       # Visual graphical band map with fading spots
    │   ├── cat_settings.rs  # CAT & rotator configuration dialog
    │   ├── changelog.rs     # Built-in changelog & update-check windows
    │   ├── command_palette.rs # Searchable command launcher (Ctrl+Shift+P)
    │   ├── cluster_panel.rs # DX Cluster spot table with ATNO highlighting
    │   ├── contest.rs       # Contest operating window with live score
    │   ├── csv_export_dialog.rs # Configurable CSV export dialog
    │   ├── cw_macros.rs     # CW macro configuration modal
    │   ├── cw_terminal.rs   # CW keyer terminal window
    │   ├── find_duplicates.rs# Smart duplicate QSO detection and batch cleanup
    │   ├── icons.rs         # Centralized icon set
    │   ├── iota_browser.rs  # IOTA directory browser
    │   ├── journal_manager.rs # Multi-journal profile manager
    │   ├── logbook_table.rs # Paginated QSO log table with sorting
    │   ├── menu/            # Reorganized top menu (File/Edit/View/Operation/References/Tools/Settings/Help)
    │   ├── mini_hud.rs      # Compact desktop VFO HUD
    │   ├── online_sync.rs   # Cloud synchronization manager dialog
    │   ├── operator_assistant.rs # Live "what to do now" recommendation bar
    │   ├── photo_viewer.rs  # QSL and station photo viewer
    │   ├── plugin_manager.rs# Rhai user plugin manager window
    │   ├── marketplace.rs   # Plugin marketplace (one-click install catalog)
    │   ├── prefix_manager.rs# Country & prefix lookup browser
    │   ├── qsl_designer.rs  # QSL designer & Avery A4 PDF label exporter
    │   ├── qsl_manager.rs   # Paper & electronic QSL manager
    │   ├── qso_entry.rs     # Primary QSO logging panel with propagation badge & dupe check
    │   ├── satellites.rs    # Satellite tracker & Doppler CAT compensator
    │   ├── send_spot.rs     # Cluster spotting dialog
    │   ├── solar_panel.rs   # Space weather & HF band opening conditions table
    │   ├── sota_dialog.rs   # SOTA reference lookup dialog
    │   ├── states_browser.rs# US states & WAS browser
    │   ├── station_ledger.rs# Station maintenance logbook
    │   ├── user_manual.rs   # In-app user manual with contextual help links
    │   ├── station_profiles.rs# Workstation multi-profiles (Home, /P, SOTA, Contest)
    │   ├── statistics.rs    # Visual analytics & charts dashboard (click-to-drill-down)
    │   ├── theme.rs         # Central theme & palette system
    │   ├── vfo_panel.rs     # Primary transceiver VFO & PTT panel
    │   ├── voice_keyer.rs   # SSB voice keyer window (WAV, F1–F8)
    │   ├── waterfall_panel.rs # SDR waterfall & FFT spectrum panel (dockable tile)
    │   ├── welcome_wizard.rs# First-time station setup wizard
    │   ├── wol_dialog.rs    # Wake-on-LAN remote rig trigger
    │   ├── workspace_profiles.rs # Operator layout (workspace) profile manager
    │   ├── world_map.rs     # Interactive world map with Grey Line & rotator
    │   └── wspr_panel.rs    # Real-time WSPR monitor
    ├── plugins/             # User plugin system (embedded Rhai) + marketplace
    │   ├── mod.rs           # PluginEngine: sandboxed .rhai scripts, getters/actions & hooks
    │   ├── bridge.rs        # PluginCommand queue + PluginSnapshot state bridge
    │   ├── lookups.rs       # POTA/SOTA REST lookups for plugin actions
    │   └── marketplace.rs   # Plugin catalog, fetch, SHA256-verified install/uninstall
    ├── sync/                # Encrypted peer-to-peer synchronization
    │   └── p2p.rs           # XChaCha20-Poly1305 P2P log sync over TCP
    ├── network/             # Network utilities
    │   └── wol.rs           # Wake-on-LAN magic packet sender
    └── media/               # Audio alerts & assets
        ├── audio_recorder.rs# On-air QSO audio recording engine
        ├── sounds.rs        # Synthesized audio alerts (rodio)
        └── voice_keyer.rs   # SSB voice keyer audio playback engine
```

---

## 🔒 Configuration & Data Safety

- **Storage Location:** All user databases, journal files, and configuration files are stored safely in:
  ```
  %APPDATA%\SPLogbook\
  ```
- **Portable Mode:** If a `station_config.json` file is present in the application's local directory, SPLogbook automatically switches to **Portable Mode**, storing all databases in the program folder (ideal for USB sticks and contest field operations).
- **Credentials:** Service passwords, API keys, LAN sync secret and the local REST API token
  are stored in the operating system's credential store (Windows Credential Manager,
  macOS Keychain or a Linux Secret Service provider), not in `station_config.json`.
  Existing plaintext credentials are moved into the store on first launch; startup
  stops with an error if migration or access fails, without overwriting the original
  configuration. On Linux, a running, unlocked Secret Service is required. Copying
  a configuration or portable folder to another computer does **not** copy its
  credentials: clear the `secret_store_id` value in the copied JSON file, then enter
  the credentials again on the destination computer. The same step is needed to
  restore an older protected config whose keychain entry no longer exists. Older
  plaintext copies or backups of `station_config.json` remain sensitive until removed.
- **Crash Prevention:** Mutexes protect shared data structures against thread poisoning; background tasks communicate over thread-safe mpsc channels, ensuring that long-running Telnet or CAT operations never block the 60 FPS graphical interface.

---

## 🤝 Contributing & Bug Reports

Contributions, issue reports, and suggestions are warmly welcome!
1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/amazing-feature`).
3. Verify that all automated tests pass (`cargo test`).
4. Ensure clean code formatting (`cargo clippy`).
5. Commit your changes (`git commit -m 'Add amazing feature'`).
6. Push to the branch (`git push origin feature/amazing-feature`).
7. Open a Pull Request.

---

## ☕ Support & Donations

**SPLogbook** is a 100% free, libre, and open-source amateur radio workstation console built with deep passion for the global amateur radio community. The software contains no advertisements, locked tiers, telemetry tracking, or subscriptions. It is actively maintained and evolved solely through dedication, on-air field testing, and hundreds of engineering hours invested into high-performance Rust systems programming, real-time ionospheric modeling, and seamless hardware CAT automation.

If **SPLogbook** enhances your daily logging experience, DX chasing, contest operations, SOTA/POTA field activations, or QSL printing, and you would like to appreciate the work or help support ongoing development, hardware testing, and project infrastructure — **feel free to buy me a coffee!** ☕

Every contribution is a huge motivation to continue adding new features, optimizing performance, and delivering the highest quality software to radio amateurs around the world.

<p align="center">
  <a href="https://buycoffee.to/sp6ina" target="_blank">
    <img src="https://img.shields.io/badge/☕_Buy_a_Coffee_for_SP6INA-buycoffee.to%2Fsp6ina-FFDD00?style=for-the-badge&logoColor=black" alt="Buy a Coffee for SP6INA on buycoffee.to">
  </a>
  <br><br>
  👉 <strong><a href="https://buycoffee.to/sp6ina">https://buycoffee.to/sp6ina</a></strong> 👈
</p>

*Thank you for your generous support, and see you on the air! Vy 73 de SP6INA* 🎙️📻

---

## 📄 License & Credits

SPLogbook is licensed under the **GNU General Public License v3.0 (GPLv3)**. See the [LICENSE](LICENSE) file for details.

### Author & Maintainer
**Mariusz Woźniak (SP6INA)**  
- E-mail: [contact@splogbook.org](mailto:contact@splogbook.org)  
- QRZ Profile: [SP6INA on QRZ.com](https://www.qrz.com/db/SP6INA)  
- GitHub: [@sp6ina](https://github.com/sp6ina)  
- ☕ Support & Donate: [buycoffee.to/sp6ina](https://buycoffee.to/sp6ina)  

*Vy 73 & Good DX!* 📻
