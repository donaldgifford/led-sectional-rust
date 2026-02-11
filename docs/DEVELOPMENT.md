# Development Guide

Set up a local machine to build, test, and flash the LED Sectional firmware onto an ESP32-C3.

## Prerequisites

### Hardware

- **ESP32-C3** development board (e.g., ESP32-C3-DevKitM-1, Seeed XIAO ESP32C3)
- **USB cable** (USB-C or micro-USB depending on your board)
- **WS2812B LED strip** connected to GPIO 2 (configurable in `cfg.toml`)

### System Dependencies

**macOS:**

```bash
brew install cmake ninja python3 libuv
```

**Linux (Debian/Ubuntu):**

```bash
sudo apt-get install -y git curl gcc cmake ninja-build python3 python3-venv libssl-dev pkg-config libuv1-dev
```

**Linux (Fedora):**

```bash
sudo dnf install -y git curl gcc cmake ninja-build python3 openssl-devel pkg-config libuv-devel
```

### Rust Toolchain

Install Rust via [rustup](https://rustup.rs/) if you haven't already:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The project uses two Rust channels:

| Crate | Channel | Purpose |
|-------|---------|---------|
| Core library (`crates/led-sectional-core/`) | Stable (1.92+) | Host-side builds and tests |
| Firmware (`firmware/`) | Nightly | Required for `build-std` cross-compilation |

The nightly toolchain is automatically selected inside `firmware/` via `rust-toolchain.toml`. No manual override needed.

### ESP32 Tools

Install the linker proxy and flash tool:

```bash
cargo install ldproxy espflash
```

- **ldproxy** — Proxy linker that delegates to the ESP-IDF build system
- **espflash** — Flashes firmware binaries and opens a serial monitor

### Optional: mise

The project includes a `mise.toml` for managing tool versions. If you use [mise](https://mise.jdx.dev/):

```bash
mise install
```

This installs Rust 1.92 (for host builds), shellcheck, and shfmt.

## Project Structure

```
led-sectional-rust/
├── Cargo.toml                  # Workspace root (core library only)
├── cfg.toml.example            # Example configuration file
├── crates/
│   └── led-sectional-core/     # Pure Rust library (host-testable)
│       └── src/
│           ├── config.rs       # TOML config parsing
│           ├── error.rs        # Error types (thiserror)
│           ├── led.rs          # LED state, colors, brightness, lightning
│           └── metar.rs        # METAR JSON parsing, URL building
├── firmware/                   # ESP32-C3 binary (NOT in workspace)
│   ├── .cargo/config.toml      # Cross-compilation target & flags
│   ├── rust-toolchain.toml     # Nightly toolchain
│   ├── sdkconfig.defaults      # ESP-IDF SDK configuration
│   ├── build.rs                # ESP-IDF build integration
│   └── src/
│       ├── main.rs             # Entry point, main loop
│       ├── wifi.rs             # WiFi STA + NVS credentials
│       ├── led_driver.rs       # WS2812B hardware driver
│       ├── metar_client.rs     # HTTPS METAR fetcher
│       └── provisioning.rs     # Captive portal (SoftAP + HTTP)
└── docs/
```

The core library is a workspace member and compiles on the host with stable Rust. The firmware crate lives outside the workspace and requires the ESP-IDF toolchain for cross-compilation.

## Building

### Core Library (Host)

From the repository root:

```bash
# Build
cargo build --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Test
cargo nextest run --workspace

# Format check
cargo fmt --check
```

### Firmware (ESP32-C3)

From the `firmware/` directory:

```bash
cd firmware

# Build only (no flash)
cargo build

# Build release (smaller binary)
cargo build --release
```

The first build downloads and compiles ESP-IDF v5.3.3 automatically. This initial build is slow — subsequent builds are incremental and much faster.

## Flashing and Running

Connect your ESP32-C3 via USB, then:

```bash
cd firmware

# Build, flash, and open serial monitor
cargo run

# Or flash a release build
cargo run --release
```

This uses `espflash flash --monitor` as configured in `.cargo/config.toml`. The serial monitor displays log output (boot messages, WiFi status, METAR fetches).

Press `Ctrl+R` to reset the device, `Ctrl+C` to exit the monitor.

### Manual Flash

If you need more control:

```bash
cd firmware

# Build first
cargo build --release

# Flash with explicit port
espflash flash target/riscv32imc-esp-espidf/release/led-sectional-firmware --port /dev/ttyUSB0

# Open serial monitor separately
espflash monitor --port /dev/ttyUSB0
```

On macOS, the port is typically `/dev/cu.usbmodem*` or `/dev/cu.usbserial-*`. On Linux, it's usually `/dev/ttyUSB0` or `/dev/ttyACM0`.

## Configuration

Copy the example config and customize it:

```bash
cp cfg.toml.example cfg.toml
```

Edit `cfg.toml` to match your setup:

```toml
[settings]
brightness = 20                # LED brightness (0-255)
request_interval_secs = 900    # METAR fetch interval in seconds (60-3600)
wind_threshold_kt = 25         # Wind speed for yellow indication (knots)
do_lightning = true             # Flash white for thunderstorm airports
do_winds = true                 # Show yellow for high-wind VFR
data_pin = 2                   # GPIO pin for WS2812B data line

[wifi]
# Uncomment for development (avoids captive portal each time)
# ssid = "YourNetworkName"
# password = "YourPassword"

# Map each LED position to an airport ICAO code or special code.
# Special codes: NULL (off), VFR, MVFR, IFR, LIFR, WVFR (legend colors), LTNG (lightning demo)

[[airports]]
code = "KSFO"

[[airports]]
code = "KLAX"

[[airports]]
code = "KJFK"
```

The firmware includes `cfg.toml.example` at compile time via `include_str!`. To use a custom config during development, set WiFi credentials in `[wifi]` so you don't have to go through captive portal provisioning on every flash.

## WiFi Provisioning

On first boot without WiFi credentials (no NVS entry and no `[wifi]` in config):

1. The device starts a WiFi access point named **LED-Sectional-Setup**
2. Connect to it from your phone or laptop
3. Open a browser to any URL — you'll be redirected to the setup form
4. Enter your WiFi SSID and password, then submit
5. Credentials are saved to NVS (flash storage) and the device reboots
6. On subsequent boots, stored credentials are used automatically

## Running Tests

Run a single test by name:

```bash
cargo nextest run --workspace -E 'test(=metar::tests::parse_valid_json)'
```

Run all tests in a module:

```bash
cargo nextest run --workspace -E 'test(~config::tests)'
```

## Troubleshooting

### `espflash` can't find the device

- Check that the USB cable supports data (not charge-only)
- On Linux, add your user to the `dialout` group: `sudo usermod -aG dialout $USER` (then log out and back in)
- On macOS, install the appropriate USB-to-UART driver if your board uses CP2102 or CH340

### First firmware build fails or hangs

The initial ESP-IDF download and compilation can take a while and needs internet access. If it fails:

- Check your internet connection
- Ensure Python 3 and venv are installed (ESP-IDF uses Python internally)
- Try cleaning and rebuilding: `cd firmware && cargo clean && cargo build`

### Stack overflow at runtime

If you see a stack overflow panic in the serial monitor, increase the stack size in `firmware/sdkconfig.defaults`:

```
CONFIG_ESP_MAIN_TASK_STACK_SIZE=16384
```

Then do a clean rebuild: `cd firmware && cargo clean && cargo build`

### `ldproxy` not found

```bash
cargo install ldproxy
```

Make sure `~/.cargo/bin` is in your `PATH`.
