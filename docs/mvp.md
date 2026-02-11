# LED Sectional Rust — MVP Implementation Plan

This document is a detailed plan for converting [WKHarmon/led-sectional](https://github.com/WKHarmon/led-sectional) from C/Arduino on ESP8266 to Rust on ESP32.

---

## Table of Contents

- [Source Project Analysis](#source-project-analysis)
- [Platform Decision](#platform-decision)
- [Architecture Overview](#architecture-overview)
- [MVP Scope](#mvp-scope)
- [Project Structure](#project-structure)
- [Implementation Phases](#implementation-phases)
  - [Phase 1: Project Scaffolding](#phase-1-project-scaffolding)
  - [Phase 2: LED Driver](#phase-2-led-driver)
  - [Phase 3: Configuration System](#phase-3-configuration-system)
  - [Phase 4: WiFi Connectivity](#phase-4-wifi-connectivity)
  - [Phase 5: METAR Fetching and Parsing](#phase-5-metar-fetching-and-parsing)
  - [Phase 6: Flight Category to LED Mapping](#phase-6-flight-category-to-led-mapping)
  - [Phase 7: Wind and Lightning Effects](#phase-7-wind-and-lightning-effects)
  - [Phase 8: WiFi Provisioning (Captive Portal)](#phase-8-wifi-provisioning-captive-portal)
- [Post-MVP Features](#post-mvp-features)
- [Hardware BOM](#hardware-bom)
- [Crate Dependencies](#crate-dependencies)
- [Reference Links](#reference-links)

---

## Source Project Analysis

The original [led-sectional](https://github.com/WKHarmon/led-sectional) runs on an **ESP8266 (Wemos D1 Mini Lite)** with the Arduino framework. Here is what it does:

### Core Data Flow

1. Boot → mount LittleFS → load config from `/config.json`
2. Connect to WiFi (via WiFiManager captive portal if no saved credentials)
3. Every 15 minutes, fetch METAR data from `https://aviationweather.gov/api/data/metar?format=json&ids=KABC,KDEF,...`
4. Stream-parse the JSON response (SAX-style, because ESP8266 has ~40KB free heap)
5. For each METAR, map the `fltCat` (flight category) field to a color and set the corresponding LED
6. Apply wind and lightning animations in the main loop

### Flight Category Colors

| Category | Color   | Meaning                          |
|----------|---------|----------------------------------|
| VFR      | Green   | Visual Flight Rules (ceiling >3000ft, vis >5mi) |
| MVFR     | Blue    | Marginal VFR                     |
| IFR      | Red     | Instrument Flight Rules          |
| LIFR     | Magenta | Low IFR (worst conditions)       |
| Wind     | Yellow  | VFR with high winds (>25kt default) |
| Unknown  | Black   | No data available                |

### Special LED Codes

The airport list supports placeholder codes that don't fetch METARs:

- `NULL` — skip (LED off)
- `VFR`, `MVFR`, `IFR`, `LIFR`, `WVFR` — static legend colors
- `LTNG` — lightning demo LED
- `WBNK` — wind blink demo LED

### Features Inventory

| Feature | MVP? | Notes |
|---------|------|-------|
| METAR fetch + parse | Yes | Core functionality |
| Flight category → LED color | Yes | Core functionality |
| Airport list config | Yes | TOML file instead of hardcoded |
| WiFi connection | Yes | Required for METAR fetch |
| WiFi provisioning (captive portal) | Yes | Key improvement over hardcoding |
| High wind indication (static yellow) | Yes | Simple color swap |
| Lightning flash animation | Yes | White flash on thunderstorm airports |
| Wind alternation mode | No | Post-MVP enhancement |
| Ambient light sensor | No | Post-MVP (requires I2C or ADC) |
| MQTT / Home Assistant | No | Post-MVP |
| Web Serial config interface | No | Post-MVP (replaced by TOML config for MVP) |
| Legend LEDs | Yes | Static colors for chart legend |

---

## Platform Decision

### Chip: ESP32-C3

| Factor | Decision | Rationale |
|--------|----------|-----------|
| **Chip** | ESP32-C3 | RISC-V = standard Rust toolchain (no Espressif fork needed). Single GPIO pin is all we need for WS2812B. WiFi 4 is sufficient. Cheapest option (~$2-4 for dev boards). |
| **Alt chip** | ESP32-C6 | Drop-in alternative if WiFi 6 / Thread / Zigbee are desired later. Same RISC-V toolchain, same crate compatibility. |
| **Framework** | `esp-idf-svc` (std) | Mature WiFi stack (production-proven ESP-IDF underneath). Full `std` library (String, Vec, threads). HTTP client with TLS. NVS for credential storage. HTTP server for captive portal. |
| **LED driver** | `ws2812-esp32-rmt-driver` | Uses the RMT peripheral for precise WS2812B timing. `smart-leds-trait` feature for ergonomic API. Actively maintained (v0.13.1). |
| **Config format** | TOML | Human-readable, editable with any text editor. `toml` crate v0.9+ has serde support. Stored on flash filesystem. |

### Why Not ESP32-S3?

The S3 is overkill (dual-core Xtensa @ 240MHz, 45 GPIOs, PSRAM) and requires the Espressif forked Rust compiler because Xtensa is not yet in upstream LLVM. The C3's single RISC-V core at 160MHz with 400KB SRAM is more than enough for fetching weather data and driving LEDs.

### Why Not `esp-hal` (bare-metal / no_std)?

While `esp-hal` reached 1.0 in October 2025 and is Espressif-funded, its WiFi support is still marked unstable. The `esp-idf-svc` WiFi stack is the same C code powering millions of production ESP32 devices. For a project that fundamentally depends on reliable WiFi + HTTPS, the std approach is the pragmatic choice.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    main loop                        │
│                                                     │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────┐ │
│  │  config   │  │   wifi    │  │   metar_client   │ │
│  │  (TOML)   │  │ (STA/AP)  │  │  (HTTPS fetch)   │ │
│  └─────┬─────┘  └─────┬─────┘  └────────┬─────────┘ │
│        │              │                  │           │
│        ▼              │                  ▼           │
│  ┌──────────┐         │         ┌──────────────────┐ │
│  │  flash    │         │         │  metar_parser    │ │
│  │  (NVS +   │         │         │  (JSON → struct) │ │
│  │  LittleFS)│         │         └────────┬─────────┘ │
│  └──────────┘         │                  │           │
│                       │                  ▼           │
│                       │         ┌──────────────────┐ │
│                       │         │  led_controller  │ │
│                       │         │  (colors, fx)    │ │
│                       │         └────────┬─────────┘ │
│                       │                  │           │
│                       │                  ▼           │
│                       │         ┌──────────────────┐ │
│                       │         │  WS2812B strip   │ │
│                       │         │  (RMT peripheral)│ │
│                       │         └──────────────────┘ │
│                       │                              │
│              ┌────────┴────────┐                     │
│              │  http_server    │                     │
│              │  (captive portal│                     │
│              │   + config API) │                     │
│              └─────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `main` | Boot sequence, main loop orchestration, timing |
| `config` | TOML parsing, serde structs, defaults, validation |
| `wifi` | STA connection, AP mode for provisioning, credential storage in NVS |
| `metar_client` | HTTPS GET to aviationweather.gov, response handling |
| `metar_parser` | Deserialize JSON METAR response into typed structs |
| `led_controller` | Map flight categories to colors, manage LED state, animations |
| `provisioning` | Captive portal HTTP server, WiFi config form |

---

## MVP Scope

The MVP delivers a working LED sectional that:

1. Reads airport list and settings from a TOML config file on flash
2. Connects to WiFi (with captive portal fallback for first-time setup)
3. Fetches METAR data from the FAA API every 15 minutes
4. Displays flight category colors on a WS2812B LED strip
5. Shows high-wind airports in yellow
6. Flashes lightning (thunderstorm) airports white
7. Supports legend LEDs and NULL (skip) entries

What the MVP does **not** include: MQTT, ambient light sensor, Web Serial, wind alternation mode, OTA updates.

---

## Project Structure

The project uses a **workspace architecture** that separates host-testable pure logic from ESP32 hardware-dependent code:

```
led-sectional-rust/
├── Cargo.toml                        # Workspace root (members: crates/led-sectional-core)
├── crates/
│   └── led-sectional-core/           # Pure Rust library — compiles and tests on host
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs             # TOML config structs, parsing, defaults, validation
│           ├── metar.rs              # MetarReport struct, JSON parsing
│           ├── led.rs                # Color constants, flight category mapping, effects logic
│           └── error.rs              # thiserror error types
├── firmware/                         # ESP32-C3 binary (NOT in workspace — built separately)
│   ├── Cargo.toml                    # Depends on led-sectional-core + ESP-IDF crates
│   ├── build.rs
│   ├── sdkconfig.defaults
│   ├── rust-toolchain.toml
│   ├── .cargo/
│   │   └── config.toml              # target = riscv32imc-esp-espidf, build-std
│   └── src/
│       ├── main.rs                   # Boot sequence, main loop
│       ├── wifi.rs                   # WiFi STA connection via esp-idf-svc
│       ├── led_driver.rs             # WS2812B driver via ws2812-esp32-rmt-driver
│       └── provisioning.rs           # SoftAP + captive portal HTTP server
├── cfg.toml.example                  # Example config for users
└── docs/
    └── mvp.md                        # This file
```

**Why two crates?** The core library contains all pure logic (config parsing, METAR JSON parsing, flight category → color mapping, effects logic) and compiles/tests on any host with standard `cargo build && cargo test`. The firmware crate is a thin wrapper that adds ESP32 hardware integration (WiFi, LED driver, HTTP client) and is cross-compiled for the ESP32-C3 target.

**Build commands:**
- Core library (host): `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run`
- Firmware (ESP32): `cd firmware && cargo build`

---

## Implementation Phases

### Phase 1: Project Scaffolding

**Goal**: Set up the workspace with core library crate and firmware crate structure. Core crate compiles and passes clippy on the host.

**Steps**:

1. Create workspace root `Cargo.toml` with `members = ["crates/led-sectional-core"]`
2. Create `crates/led-sectional-core/` library crate with `lib.rs` stub
3. Create `firmware/` ESP32-C3 binary crate with:
   - `Cargo.toml` depending on `led-sectional-core` (path) + `esp-idf-svc`
   - `.cargo/config.toml` targeting `riscv32imc-esp-espidf` with `build-std`
   - `build.rs` with `embuild::espidf::sysenv::output()`
   - `sdkconfig.defaults` for WiFi + HTTPS + NVS
   - `rust-toolchain.toml` pinning nightly with `rust-src` component
   - `src/main.rs` with minimal ESP-IDF boot
4. Create `cfg.toml.example` with documented default config

**Acceptance Criteria**:

- [x] Workspace root `Cargo.toml` exists with core crate as member
- [x] `cargo build` from workspace root succeeds (builds core crate)
- [x] `cargo clippy --all-targets -- -D warnings` passes
- [x] `cargo nextest run` passes (no tests yet, but harness runs)
- [x] `firmware/` directory exists with all ESP-IDF scaffolding files
- [x] `cfg.toml.example` exists with documented settings, wifi, and airports sections

### Phase 2: LED Driver

**Goal**: Create LED color constants and state management logic in the core crate. ESP32 WS2812B driver wrapper in firmware crate.

**Steps**:

1. Create `crates/led-sectional-core/src/led.rs`:
   - Define `Color` struct (RGB8-compatible but owned, no dependency on `smart-leds` in core)
   - Define color constants: VFR (green), MVFR (blue), IFR (red), LIFR (magenta), WIND (yellow), UNKNOWN (black), LIGHTNING (white), status colors (CONNECTING=orange, CONNECTED=purple, FETCH_ERROR=cyan)
   - `LedState` struct managing a `Vec<Color>` buffer with brightness
   - Methods: `new(num_leds)`, `set(index, color)`, `set_all(color)`, `get(index)`, `brightness_scaled_buffer()` — returns colors with brightness applied
   - Brightness applied by scaling RGB values (same approach as FastLED's global brightness)
2. Add `ws2812-esp32-rmt-driver` and `smart-leds` deps to `firmware/Cargo.toml`
3. Create `firmware/src/led_driver.rs` — wraps `Ws2812Esp32Rmt`, takes `&LedState` to write to hardware

**Acceptance Criteria**:

- [x] `led.rs` module exists in core crate with `Color` struct and all color constants
- [x] `LedState` struct manages LED buffer with brightness scaling
- [x] `brightness_scaled_buffer()` correctly scales colors (0 = all off, 255 = full)
- [x] Unit tests pass for color constants, brightness scaling, set/get operations
- [x] `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run` passes
- [x] `firmware/src/led_driver.rs` exists with WS2812B driver wrapper

### Phase 3: Configuration System

**Goal**: Parse airport list and settings from TOML strings. Config structs and parsing live in the core crate. Filesystem I/O is in the firmware crate.

**Config file format** (`cfg.toml.example`):

```toml
[settings]
brightness = 20
request_interval_secs = 900
wind_threshold_kt = 25
do_lightning = true
do_winds = true
data_pin = 2

[wifi]
# ssid = "MyNetwork"
# password = "MyPassword"

[[airports]]
code = "LIFR"

[[airports]]
code = "IFR"

[[airports]]
code = "MVFR"

[[airports]]
code = "VFR"

[[airports]]
code = "WVFR"

[[airports]]
code = "KSFO"

[[airports]]
code = "KLAX"

[[airports]]
code = "NULL"
```

**Steps**:

1. Add `toml` and `serde` dependencies to core crate
2. Create `crates/led-sectional-core/src/config.rs`:
   - `Config`, `Settings`, `WifiConfig`, `Airport` structs with `Deserialize`
   - `serde(default)` on all fields for partial config support
   - `Config::from_toml(s: &str) -> Result<Config>` parses TOML string
   - `Config::num_leds()` returns airport count
   - `Config::metar_airport_codes()` filters out special codes, returns ICAO codes only
   - Validation method that clamps values to safe ranges

**Note on WiFi credentials**: WiFi SSID/password will be stored in NVS (not in the TOML file) after captive portal provisioning. The TOML `[wifi]` section is an optional override for development convenience.

**Acceptance Criteria**:

- [x] `config.rs` module with `Config`, `Settings`, `WifiConfig`, `Airport` structs
- [x] `Config::from_toml()` parses a full config string correctly
- [x] `Config::from_toml()` handles partial configs (missing fields use defaults)
- [x] `Config::metar_airport_codes()` filters out NULL, VFR, MVFR, IFR, LIFR, WVFR, LTNG
- [x] Validation clamps brightness (0-255), request_interval (60-3600), wind_threshold (0-100)
- [x] Unit tests cover: full config, partial config, defaults, airport filtering, validation
- [x] `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run` passes

### Phase 4: WiFi Connectivity

**Goal**: Connect to WiFi in STA mode using credentials from NVS (or TOML fallback). Firmware crate only.

**Steps**:

1. Create `firmware/src/wifi.rs`:
   - `WifiManager` struct wrapping `EspWifi<'static>`
   - `new(modem, sysloop, nvs)` constructor
   - `connect_sta(ssid, password)` — configure STA mode, start, wait for IP with timeout
   - `is_connected()` — check current connection status
   - `store_credentials(nvs, ssid, password)` — persist to NVS
   - `load_credentials(nvs)` — read from NVS, returns `Option<(String, String)>`
2. Credential lookup order in `main.rs`:
   1. Check NVS for stored `wifi_ssid` and `wifi_pass`
   2. Fall back to `config.wifi.ssid` / `config.wifi.password` from TOML
   3. If neither exists, enter provisioning mode (Phase 8)
3. Connection flow with LED status indication via `LedState`

**Acceptance Criteria**:

- [x] `firmware/src/wifi.rs` exists with `WifiManager` struct
- [x] NVS credential load/store functions implemented
- [x] STA connection with configurable timeout
- [x] `firmware/src/main.rs` integrates WiFi with credential lookup order
- [x] Core crate still builds and passes: `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run`

### Phase 5: METAR Fetching and Parsing

**Goal**: Parse METAR JSON into Rust structs (core crate). HTTPS fetch via ESP-IDF (firmware crate).

**API endpoint**: `GET https://aviationweather.gov/api/data/metar?format=json&ids=KSFO,KLAX,...`

**Response format** (JSON array):
```json
[
  {
    "icaoId": "KSFO",
    "fltCat": "VFR",
    "wspd": 12,
    "wgst": null,
    "wxString": "HZ",
    ...
  }
]
```

**Steps**:

1. Add `serde_json` to core crate
2. Create `crates/led-sectional-core/src/metar.rs`:
   - `MetarReport` struct with serde `rename_all = "camelCase"`
   - `parse_metars(json: &str) -> Result<Vec<MetarReport>>`
   - `MetarReport::has_thunderstorm()` — checks `wx_string` for "TS"
   - `MetarReport::max_wind()` — returns max of `wspd` and `wgst`
   - `build_metar_url(codes: &[&str]) -> String` — constructs the API URL
3. Create `firmware/src/metar_client.rs`:
   - `MetarClient` struct using `EspHttpConnection`
   - `fetch(airport_codes)` — HTTPS GET, read body, delegate to core's `parse_metars()`

**Note on TLS**: `EspHttpConnection` uses mbedTLS. Enable Mozilla CA bundle in `sdkconfig.defaults` (`CONFIG_MBEDTLS_CERTIFICATE_BUNDLE=y`).

**Acceptance Criteria**:

- [ ] `metar.rs` module in core crate with `MetarReport` struct
- [ ] `parse_metars()` correctly parses JSON array of METAR objects
- [ ] `has_thunderstorm()` detects "TS" in wxString
- [ ] `max_wind()` returns the larger of wspd and wgst
- [ ] `build_metar_url()` constructs correct URL with comma-separated codes
- [ ] Unit tests cover: valid JSON, empty array, null fields, thunderstorm detection, wind calculation
- [ ] `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run` passes
- [ ] `firmware/src/metar_client.rs` exists with HTTP client implementation

### Phase 6: Flight Category to LED Mapping

**Goal**: Map parsed METARs to LED colors. All logic in core crate.

**Steps**:

1. Add to `led.rs`:
   - `flight_category_color(category, wind_speed, wind_gust, wind_threshold, do_winds) -> Color`
   - `special_code_color(code: &str) -> Option<Color>` — returns static legend colors or None for real airports
   - `update_leds_from_metars(led_state, config, metars)` — orchestrates the full mapping:
     - Build `HashMap<String, MetarReport>` keyed by `icao_id`
     - Iterate airports: special codes get legend colors, real airports get METAR-based colors
     - Returns list of lightning LED indices (for Phase 7)

**Acceptance Criteria**:

- [ ] `flight_category_color()` returns correct color for each category
- [ ] `flight_category_color()` returns yellow for VFR with winds above threshold when do_winds=true
- [ ] `special_code_color()` maps all special codes correctly, returns None for ICAO codes
- [ ] `update_leds_from_metars()` correctly sets LED buffer from config + METAR data
- [ ] Unit tests cover: each flight category, wind override, unknown category, special codes, full mapping
- [ ] `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run` passes

### Phase 7: Wind and Lightning Effects

**Goal**: Lightning flash animation logic and thunderstorm detection. Core crate manages state; firmware drives the timing.

**Lightning**:
- `update_leds_from_metars()` (Phase 6) already collects lightning indices
- Also include airports with special code `LTNG`
- `LedState` gains lightning management:
  - `set_lightning_indices(indices)` — stores which LEDs flash
  - `apply_lightning_flash()` — sets lightning LEDs to white, returns true if any were set
  - `restore_lightning()` — restores original colors
- Firmware main loop calls flash → sleep 25ms → restore each iteration
- Guard with `config.settings.do_lightning`

**Wind** (static yellow mode for MVP):
- Already handled by `flight_category_color()` in Phase 6
- Wind alternation mode is post-MVP

**Acceptance Criteria**:

- [ ] `LedState::set_lightning_indices()` stores indices and saves original colors
- [ ] `LedState::apply_lightning_flash()` sets lightning LEDs to white, returns whether any exist
- [ ] `LedState::restore_lightning()` restores original colors
- [ ] Lightning indices include LTNG special code positions
- [ ] Unit tests cover: no lightning (empty), single flash, multiple flashes, restore correctness
- [ ] `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run` passes

### Phase 8: WiFi Provisioning (Captive Portal)

**Goal**: On first boot (no stored credentials), start a WiFi AP with a captive portal that lets the user enter their WiFi SSID and password via a web form. Firmware crate only.

This is the key UX improvement over hardcoded credentials.

**Flow**:

1. On boot, check NVS for stored WiFi credentials
2. If none found (and none in TOML), enter provisioning mode:
   a. Set all LEDs to slow orange pulse (visual indicator)
   b. Start WiFi in AP mode: SSID = `LED-Sectional-Setup`, no password
   c. Start HTTP server on port 80
   d. Serve a simple HTML form at `GET /` asking for WiFi SSID and password
   e. Handle `POST /connect` — validate input, store in NVS, reboot
3. After reboot, credentials are loaded from NVS → normal STA connection

**Implementation in `firmware/src/provisioning.rs`**:

- `CaptivePortal` struct wrapping `EspHttpServer` + `EspWifi`
- `start(modem, sysloop, nvs)` — configures AP mode + HTTP server
- HTML form embedded as `const &str` (minimal, inline CSS, mobile-friendly)
- `POST /connect` handler: parse form, write to NVS, respond "Rebooting...", call `esp_restart()`
- 3-minute timeout: if no credentials submitted, reboot and retry

**Acceptance Criteria**:

- [ ] `firmware/src/provisioning.rs` exists with `CaptivePortal` struct
- [ ] HTML form with SSID input, password input, submit button embedded as const
- [ ] AP mode starts with SSID "LED-Sectional-Setup"
- [ ] GET / serves the HTML form
- [ ] POST /connect parses credentials and stores in NVS
- [ ] Device reboots after credential submission
- [ ] Firmware `main.rs` enters provisioning when no credentials found
- [ ] Core crate still passes: `cargo build && cargo clippy --all-targets -- -D warnings && cargo nextest run`

---

## Main Loop

```rust
fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();

    // 1. Initialize NVS, event loop, LittleFS
    // 2. Load config from TOML
    // 3. Initialize LED strip, set all orange (connecting)
    // 4. Connect WiFi (or enter provisioning)
    // 5. Flash purple on success

    let mut last_fetch = Instant::now() - fetch_interval; // Force immediate first fetch

    loop {
        // Check WiFi, reconnect if needed

        if last_fetch.elapsed() >= fetch_interval {
            match metar_client.fetch(&airport_codes) {
                Ok(reports) => {
                    update_leds(&config, &reports, &mut led_controller);
                    last_fetch = Instant::now();
                }
                Err(e) => {
                    log::error!("METAR fetch failed: {e}");
                    led_controller.set_all(COLOR_FETCH_ERROR);
                    led_controller.show()?;
                    // Retry sooner (60 seconds)
                    last_fetch = Instant::now() - fetch_interval + Duration::from_secs(60);
                }
            }
        }

        // Lightning animation (runs every loop iteration)
        if config.settings.do_lightning {
            led_controller.animate_lightning()?;
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}
```

---

## Post-MVP Features

These are explicitly **out of scope** for MVP but documented for future work:

| Feature | Description | Complexity |
|---------|-------------|------------|
| **REST API for config** | HTTP endpoints to GET/PUT config without reflashing | Medium |
| **Wind alternation mode** | Blink windy LEDs between category color and yellow | Low |
| **MQTT / Home Assistant** | Publish state, subscribe to commands, auto-discovery | Medium |
| **Ambient light sensor** | Auto-brightness via TSL2561 (I2C) or photoresistor (ADC) | Low |
| **OTA updates** | Flash new firmware over WiFi | Medium |
| **Web dashboard** | Serve a status page showing all airports + conditions | Medium |
| **Multiple LED strip support** | Drive independent strips for sectional + legend | Low |
| **mDNS** | Discover device as `led-sectional.local` | Low |
| **Config hot-reload** | Watch TOML file for changes, reload without reboot | Low |
| **ESP32-C6 support** | Alternate target for WiFi 6 / Thread | Low (same crates) |

---

## Hardware BOM

| Component | Recommendation | Approx. Cost |
|-----------|---------------|--------------|
| ESP32-C3 dev board | ESP32-C3 Super Mini or ESP32-C3-DevKitM-1 | $3-8 |
| WS2812B LED string | 50-count or 100-count, individual addressable | $8-15 |
| 5V power supply | Sized for LED count (60mA per LED at full white) | $5-10 |
| USB-C cable | Data-capable (not charge-only) for flashing | $3 |
| VFR sectional chart | FAA chart for your region | $5-10 |

**Pin connection**: Single wire from ESP32-C3 GPIO (default GPIO 2) to WS2812B data-in. Shared ground between ESP32 and LED power supply.

---

## Crate Dependencies

### Core library (`crates/led-sectional-core/Cargo.toml`)

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.9"
thiserror = "2"
log = "0.4"
```

### Firmware (`firmware/Cargo.toml`)

```toml
[dependencies]
led-sectional-core = { path = "../crates/led-sectional-core" }
esp-idf-svc = { version = "0.51", features = ["binstart", "critical-section"] }
ws2812-esp32-rmt-driver = { version = "0.13", features = ["smart-leds-trait"] }
smart-leds = "0.4"
log = "0.4"

[build-dependencies]
embuild = "0.33"
```

---

## Reference Links

- [esp-rs/esp-idf-template](https://github.com/esp-rs/esp-idf-template) — project generator
- [esp-rs/esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) — WiFi, HTTP, NVS, GPIO
- [ws2812-esp32-rmt-driver](https://github.com/cat-in-136/ws2812-esp32-rmt-driver) — WS2812B driver
- [FAA METAR API](https://aviationweather.gov/data/api/) — weather data source
- [esp-rs/esp-idf-svc WiFi example](https://github.com/esp-rs/esp-idf-svc/blob/master/examples/wifi.rs)
- [The Rust on ESP Book](https://docs.espressif.com/projects/rust/book/)
- [Original led-sectional project](https://github.com/WKHarmon/led-sectional)
