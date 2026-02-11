# Hardware Build Reference

This guide covers what you need to build an LED Sectional using the Rust firmware on an ESP32-C3. It references the original build guide at [led-sectional.kyleharmon.com](https://led-sectional.kyleharmon.com/) and calls out what's different.

## What Stays the Same

The physical build — frame, sectional chart, backing board, LED mounting, drilling, and general assembly — is identical to the original guide. Follow [led-sectional.kyleharmon.com](https://led-sectional.kyleharmon.com/) for:

- Sectional chart sourcing (FAA charts or printed reproductions)
- Backing board selection (foamcore or hardboard)
- Frame selection (SP8 picture frame with 2.25" depth recommended)
- Drilling holes for LEDs (foamboard hole drill for foamcore, 7mm bit for wood)
- LED mounting with hot glue
- Physical assembly and routing the LED strand
- Spray adhesive (3M Super 77) for mounting the chart

## What's Different

### Microcontroller: ESP32-C3 replaces Wemos D1 Mini

| | Original | This Project |
|---|----------|-------------|
| **Board** | Wemos D1 Mini Lite (ESP8266) | ESP32-C3 dev board |
| **CPU** | Xtensa L106 (80 MHz) | RISC-V (160 MHz) |
| **RAM** | ~40 KB usable | ~400 KB SRAM |
| **WiFi** | 802.11 b/g/n | 802.11 b/g/n |
| **GPIO voltage** | 3.3V | 3.3V |
| **USB** | Micro-USB (no native USB) | USB-C (native USB on most boards) |
| **Programming** | Arduino IDE or web flasher | `espflash` CLI (see [QUICKSTART.md](QUICKSTART.md)) |

Recommended ESP32-C3 boards:

- **ESP32-C3-DevKitM-1** (Espressif reference board) — widely available, USB-C, onboard antenna
- **Seeed XIAO ESP32C3** — very compact (21x17mm), USB-C, good for tight enclosures
- **WeAct ESP32-C3** — low cost, USB-C

Any ESP32-C3 board with a USB port and exposed GPIO pins will work.

### LED Strand: WS2812B replaces WS2811

| | Original | This Project |
|---|----------|-------------|
| **LED type** | WS2811 | WS2812B |
| **Data signal** | 5V | 5V (but tolerant of 3.3V on short runs) |
| **Protocol** | Same (NeoPixel/WS28xx) | Same |

The firmware uses the `ws2812-esp32-rmt-driver` crate, which targets WS2812B LEDs specifically. WS2812B strands are widely available and functionally equivalent to WS2811 for this application.

**WS2811 strands may still work** since the protocol is compatible, but WS2812B is the tested target.

### Level Shifter: Likely Not Needed

The original build requires a 3.3V-to-5V logic level shifter because the ESP8266's 3.3V GPIO output is below the WS2811 spec for a logic "high" on a 5V data line.

With ESP32-C3 and WS2812B:

- **Short runs (< 1 meter of wire between board and first LED):** WS2812B LEDs typically accept 3.3V data signals reliably. You can try without a level shifter first.
- **Long runs or reliability issues:** Add a level shifter. A single-channel 74HCT125 or the same 74HCT245 from the original guide works.

If you skip the level shifter, that eliminates the level shifter, the PCB, and some wiring from the parts list.

## Parts List

### Required

| Part | Notes |
|------|-------|
| ESP32-C3 development board | See board recommendations above |
| WS2812B LED strand | One LED per airport. Get a strand with a few spares |
| 5V DC power supply | 2A minimum for up to ~50 LEDs. Size up for larger builds (each LED draws ~60mA at full white) |
| DC barrel connector (female, panel-mount) | Solder to LED strand power pigtails |
| USB cable (data-capable) | For flashing firmware. Usually USB-C for ESP32-C3 boards |
| Hookup wire (22 AWG solid core) | For connections between board and LED strand |
| VFR Sectional chart(s) | From FAA or printed reproduction |
| Backing board | 3/8" foamcore (easiest) or 1/8" hardboard |
| Picture frame with depth | SP8 from PictureFrames.com (2.25" deep) or similar shadow box frame |
| 3M Super 77 spray adhesive | For mounting chart to backing |
| Hot glue gun + sticks | For mounting LEDs in holes |
| Foamboard hole drill set | For clean holes in foamcore. Use smallest bit |

### Optional

| Part | Notes |
|------|-------|
| 3.3V to 5V level shifter | 74HCT245 or similar. Only needed if LEDs don't respond to 3.3V signal |
| 220 ohm resistor | On data line between board and first LED. Protects against signal reflections |
| 1000 µF capacitor (6.3V+) | Between 5V and GND at power input. Smooths power-on inrush |
| JST-SM connectors | Match LED strand connectors for tool-free disconnect |
| Small project enclosure | To house the ESP32-C3 board behind the frame |
| Velcro strips | Attach power supply and board to backing |

## Wiring

```
                        ┌─────────────┐
  5V Power Supply ──────┤ DC Barrel   ├──── 5V (red) ──── LED Strand 5V
                        │ Connector   ├──── GND (white) ─ LED Strand GND
                        └─────────────┘
                                                │
  ESP32-C3 Board                                │
  ┌──────────┐                                  │
  │ GPIO 2   ├──── Data (green) ───────── LED Strand DIN
  │ GND      ├──────────────────────────────────┘
  │ 5V (USB) │  (board powered via USB or tap from LED strand 5V)
  └──────────┘
```

**Key differences from the original wiring:**
- No level shifter in the signal path (try direct first)
- GPIO 2 is the default data pin (configurable in `cfg.toml`)
- The ESP32-C3 can be powered via its USB port or from the LED strand's 5V rail (check your board's voltage input specs)

**Important:** All ground connections must be common — the ESP32-C3 GND, LED strand GND, and power supply GND must all be connected together.

## GPIO Pin Configuration

The default data pin is GPIO 2. If your board layout or wiring makes a different pin easier, change it in `cfg.toml`:

```toml
[settings]
data_pin = 2   # Change to your preferred GPIO pin
```

Most GPIO pins on the ESP32-C3 support the RMT peripheral used for WS2812B signaling. Avoid GPIO 8 (often used for the boot button) and GPIO 18/19 (USB D-/D+ on boards with native USB).

## Software Setup

After wiring, follow one of:

- **[QUICKSTART.md](QUICKSTART.md)** — Flash a pre-built binary, no build tools needed
- **[DEVELOPMENT.md](DEVELOPMENT.md)** — Build from source with a custom airport configuration

## Key Differences Summary

| Aspect | Original (C/Arduino) | This Project (Rust/ESP32) |
|--------|----------------------|---------------------------|
| Microcontroller | Wemos D1 Mini (ESP8266) | ESP32-C3 |
| LED type | WS2811 | WS2812B |
| Level shifter | Required | Usually not needed |
| WiFi config | Web flasher tool or hardcoded | Captive portal on first boot |
| Airport config | Web tool or hardcoded in source | TOML config file (`cfg.toml`) |
| Programming | Arduino IDE / Chrome web flasher | `espflash` CLI |
| Weather refresh | Every 5 minutes | Every 15 minutes (configurable) |
| Firmware language | C (Arduino) | Rust |
