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
  <img src="https://img.shields.io/badge/Version-1.0.2-emerald.svg" alt="Version">
  <a href="https://github.com/sp6ina/SPLogbook/actions"><img src="https://github.com/sp6ina/SPLogbook/actions/workflows/build-and-release.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/Tests-54%2F54_Passed-brightgreen.svg" alt="Tests">
  <img src="https://img.shields.io/badge/i18n-6_Languages-cyan.svg" alt="i18n">
  <a href="https://buycoffee.to/sp6ina"><img src="https://img.shields.io/badge/☕_Buy_Me_a_Coffee-buycoffee.to%2Fsp6ina-FFDD00?style=flat&logoColor=black" alt="Buy Me a Coffee"></a>
</p>

<img width="1917" height="991" alt="image" src="https://github.com/user-attachments/assets/465c5eda-739d-4191-ad17-c6cc89a39347" />

---

## 🌟 Executive Summary

**SPLogbook** is an advanced, high-performance amateur radio logging software and station automation console engineered from the ground up in **Rust**. Built with an immediate-mode user interface powered by `egui`/`eframe`, SPLogbook delivers instant sub-millisecond responsiveness, zero-garbage-collection pauses, complete memory safety, and cross-platform native performance across both **Windows 10/11 (64-bit)** and **GNU/Linux (x86_64, X11 & Wayland)** environments.

From deep ionospheric modeling (VOACAP-lite HF propagation), automated antenna rotator steering (`rotctld`), and real-time DX Cluster intelligence with acoustic All-Time New One (ATNO) alarms, to bi-directional digital modes bridging (WSJT-X, JS8Call, FLDigi), orbital satellite Doppler tracking, multi-award tracking, contest logging with live scoring, and professional A4 Avery QSL label vector PDF generation, SPLogbook provides the modern radio amateur with a unified, state-of-the-art operating environment.

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
| **Transceiver CAT & Sharing** | Non-blocking Hamlib `rigctld` supervisor, VFO A/B, split, mode, PTT, WinKeyer, TCI protocol, plus integrated **Hamlib CAT TCP Proxy Server** (port 4534) for simultaneous multi-app rig sharing (WSJT-X, JTDX, FLDigi). |
| **Digital Modes** | WSJT-X / JTDX bi-directional UDP bridge (port 2237), JS8Call TCP JSON API integration, FLDigi XML-RPC bridge. |
| **Satellites** | SGP4/SDP4 Keplerian orbital propagation from TLE, real-time Doppler shift frequency correction via CAT, antenna elevation/azimuth steering. |
| **Contests & Dupe Engine** | Rules for SP DX, CQ WW, ARRL DX, CQ WPX; real-time inline `[DUPE!]` warning, N1MM-style keyboard-first auto-refocus, dedicated Duplicate Manager with smart batch cleaning, and Cabrillo 3.0 export. |
| **Awards Tracking** | DXCC (Mixed, Band, Mode, Challenge, ATNO), WAZ, WAS, SP DX Award, WAE, WWFF, RDA, PGA (2477 Polish municipalities), IOTA, SOTA, POTA. |
| **Cloud Sync** | LoTW (TQSL), eQSL.cc, Club Log, QRZ.com, HamQTH, HRDLog, Cloudlog, PSK Reporter, WSPR monitor, NOAA space weather. |
| **Mapping** | Equirectangular world map with real-time day/night terminator (Grey Line), zoom/pan, confirmed/unconfirmed QSO markers, live spot pins. |
| **REST API** | Embedded asynchronous Axum HTTP server on port 8080 providing JSON endpoints for external integration, remote monitoring, and station automation. |
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
- **Interactive QSL Card Designer:** Visual editor for previewing custom QSL layouts, positioning elements, and preparing cards for print.

---

### 7. Transceiver CAT & Hardware Supervision
- **Hamlib `rigctld` Client & Supervisor:**
  - Manages the `rigctld.exe` background process, automatically launching it on startup and terminating it smoothly.
  - Employs non-blocking polling (`try_wait` with 50ms intervals) to eliminate application freezes during shutdown.
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
- **Live Scoring & Rate Meter:** Displays current score, multiplier breakdown, band-by-band QSO counts, and rolling operating rate (QSO/h).
- **Dupe Checking:** Immediate visual warning (red highlight and bold "DUPE!" tag) when entering an already-worked station on the current band/mode.
- **Cabrillo 3.0 Export:** Generates fully compliant Cabrillo log files formatted according to official contest sponsor specifications.

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
- **Callbook XML Lookups:** Automated XML lookup via **QRZ.com** (XML subscription) and **HamQTH** (free XML API) retrieving operator name, QTH address, Maidenhead grid, country, and biography photo.
- **HRDLog & Cloudlog:** Automated cloud log synchronization with an embedded SQLite retry queue ensuring reliable delivery during internet drops.
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

# 2. Run automated unit tests (51 tests)
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
    │   ├── hamlib.rs        # Asynchronous TCP client for rigctld
    │   ├── rig_models.rs    # Database of transceiver models
    │   ├── rotor.rs         # Hamlib rotctld client (TCP port 4533)
    │   ├── server.rs        # Hamlib TCP proxy server (CAT sharing for WSJT-X/FLDigi)
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
    │   ├── solar.rs         # NOAA SWPC space weather data parser
    │   ├── updater.rs       # Automatic GitHub releases updater
    │   └── wspr.rs          # wspr.live REST API monitor
    ├── cluster/             # Spotting networks
    │   ├── lan_sync.rs      # Multi-operator LAN synchronization
    │   └── telnet.rs        # Multi-threaded Telnet client for DX Cluster
    ├── core/                # Core business logic & database
    │   ├── adif.rs          # ADIF 3.1.4 parser & exporter
    │   ├── astronomy.rs     # Solar & lunar ephemeris, solar zenith calculation
    │   ├── awards.rs        # Awards tracking engine (DXCC, WAZ, WAS, PGA, etc.)
    │   ├── backup.rs        # Database rolling backup manager (10 revisions)
    │   ├── bandplan.rs      # IARU Region 1/2/3 band plans
    │   ├── callbook.rs      # Callsign & prefix resolution
    │   ├── clubs.rs         # Specialty ham radio clubs directory (SP-OTC, SKCC, etc.)
    │   ├── contest_rules.rs # Contest scoring engine & Cabrillo exporter
    │   ├── database.rs      # SQLite WAL backend, 10 indexes, upload queue
    │   ├── geo.rs           # Maidenhead grid converter, spherical trigonometry
    │   ├── i18n.rs          # Translation dictionary (6 languages, 300+ keys)
    │   ├── pga.rs           # Polska Gmina Award database (2477 gminas)
    │   ├── prefix.rs        # ITU prefix allocations & DXCC country mapping
    │   ├── propagation.rs   # VOACAP-lite HF ionospheric propagation modeling
    │   ├── qsl_print.rs     # QSL card geometry & Avery label calculations
    │   ├── qso.rs           # QSO record data structures & validation
    │   ├── scp.rs           # Super Check Partial database lookup
    │   ├── service_db.rs    # Equipment ledger & station inventory
    │   ├── sota_export.rs   # SOTA CSV export formatter
    │   └── station.rs       # Station profile & persistent panel coordinates
    ├── digital/             # Digital mode bridges
    │   ├── fldigi.rs        # FLDigi XML-RPC client
    │   ├── js8call.rs       # JS8Call TCP JSON API integration
    │   └── wsjtx.rs         # WSJT-X / JTDX binary UDP frame decoder
    ├── gui/                 # Immediate-mode egui user interface
    │   ├── advanced_filter.rs# Multi-criteria logbook search & filter
    │   ├── app.rs           # Main application state, event loops, hotkeys
    │   ├── astronomy_dialog.rs # Celestial ephemeris modal
    │   ├── awards_matrix.rs # Interactive award matrix viewer
    │   ├── bandmap.rs       # Visual graphical band map with fading spots
    │   ├── cat_settings.rs  # CAT & rotator configuration dialog
    │   ├── cluster_panel.rs # DX Cluster spot table with ATNO highlighting
    │   ├── contest.rs       # Contest operating window with live score
    │   ├── cw_macros.rs     # CW macro configuration modal
    │   ├── cw_terminal.rs   # CW keyer terminal window
    │   ├── find_duplicates.rs# Smart duplicate QSO detection and batch cleanup
    │   ├── iota_browser.rs  # IOTA directory browser
    │   ├── journal_manager.rs # Multi-journal profile manager
    │   ├── logbook_table.rs # Paginated QSO log table with sorting
    │   ├── menu.rs          # Top menu bar and customizable toolbar
    │   ├── mini_hud.rs      # Compact desktop VFO HUD
    │   ├── online_sync.rs   # Cloud synchronization manager dialog
    │   ├── photo_viewer.rs  # QSL and station photo viewer
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
    │   ├── station_profiles.rs# Workstation multi-profiles (Home, /P, SOTA, Contest)
    │   ├── statistics.rs    # Visual analytics & charts dashboard
    │   ├── vfo_panel.rs     # Primary transceiver VFO & PTT panel
    │   ├── welcome_wizard.rs# First-time station setup wizard
    │   ├── wol_dialog.rs    # Wake-on-LAN remote rig trigger
    │   ├── world_map.rs     # Interactive world map with Grey Line & rotator
    │   └── wspr_panel.rs    # Real-time WSPR monitor
    └── media/               # Audio alerts & assets
        ├── audio_recorder.rs# On-air QSO audio recording engine
        └── sounds.rs        # Synthesized audio alerts (rodio)
```

---

## 🔒 Configuration & Data Safety

- **Storage Location:** All user databases, journal files, and configuration files are stored safely in:
  ```
  %APPDATA%\SPLogbook\
  ```
- **Portable Mode:** If a `station_config.json` file is present in the application's local directory, SPLogbook automatically switches to **Portable Mode**, storing all databases in the program folder (ideal for USB sticks and contest field operations).
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
- QRZ Profile: [SP6INA on QRZ.com](https://www.qrz.com/db/SP6INA)  
- GitHub: [@sp6ina](https://github.com/sp6ina)  
- ☕ Support & Donate: [buycoffee.to/sp6ina](https://buycoffee.to/sp6ina)  

*Vy 73 & Good DX!* 📻
