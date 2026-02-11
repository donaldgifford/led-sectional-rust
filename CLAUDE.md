# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LED Sectional aviation weather display — ESP32-C3 (RISC-V) Rust conversion of [WKHarmon/led-sectional](https://github.com/WKHarmon/led-sectional). Fetches METAR data from aviationweather.gov and displays flight category colors (VFR/MVFR/IFR/LIFR) on a WS2812B LED strip mapped to airports on a VFR sectional chart.

## Architecture

Workspace with two crates:

- **`crates/led-sectional-core/`** — Pure Rust library. Config parsing (TOML/serde), METAR JSON parsing, flight category→color mapping, LED state management with brightness scaling and lightning animation. Compiles and tests on the host.
- **`firmware/`** — ESP32-C3 binary (NOT in workspace). Depends on core + `esp-idf-svc`. Contains WiFi STA connection, WS2812B LED driver (`ws2812-esp32-rmt-driver`), HTTPS METAR client, and captive portal WiFi provisioning.

## Build Commands

### Core library (host — all tests run here)
```
cargo build
cargo clippy --all-targets -- -D warnings
cargo nextest run --workspace
cargo fmt --check
```

### Firmware (ESP32-C3 cross-compilation)
```
cd firmware && cargo build
cd firmware && cargo run  # build + flash + serial monitor
```

### Run a single test
```
cargo nextest run --workspace -E 'test(=metar::tests::parse_valid_json)'
```

## Development Environment

Managed via [mise](https://mise.jdx.dev/). Run `mise install` to set up Rust 1.92, shellcheck, shfmt.

The firmware crate uses `nightly` Rust (set via `firmware/rust-toolchain.toml`) for `build-std` support. The core library works with stable Rust.

ESP-IDF v5.3.3 is auto-downloaded on first firmware build. Install `ldproxy` and `espflash`:
```
cargo install ldproxy espflash
```

## Key Design Decisions

- Config via TOML file (`cfg.toml.example`) instead of hardcoded values
- WiFi credentials stored in NVS (flash key-value store), not in config file
- Captive portal provisioning (SoftAP + HTTP form) for first-time WiFi setup
- Full JSON deserialization (not streaming) since ESP32-C3 has 400KB SRAM
- `thiserror` for error types, no `.unwrap()` in library code
- `SAFETY` comments on all `unsafe` blocks (only `esp_restart()` calls)
