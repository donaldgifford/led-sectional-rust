# Quick Start Guide

Get the LED Sectional firmware onto your ESP32-C3 without building from source.

## What You Need

- **ESP32-C3** development board (e.g., ESP32-C3-DevKitM-1, Seeed XIAO ESP32C3)
- **USB cable** that supports data (not charge-only)
- **WS2812B LED strip** — one LED per airport on your sectional chart
- A computer running **macOS** or **Linux**

## Step 1: Install espflash

`espflash` is the tool that writes firmware to your ESP32. Install it with:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then:

```bash
cargo install espflash
```

If you already have Rust installed, skip the first command.

> **Linux users:** Add your user to the `dialout` group for USB access:
>
> ```bash
> sudo usermod -aG dialout $USER
> ```
>
> Log out and back in for this to take effect.

## Step 2: Download the Firmware

Go to the [Releases](https://github.com/donaldgifford/led-sectional-rust/releases) page and download the latest `.bin` firmware file for your board.

The release asset is named:

```
led-sectional-firmware.bin
```

## Step 3: Flash the Firmware

Connect your ESP32-C3 to your computer via USB.

```bash
espflash flash led-sectional-firmware.bin
```

`espflash` auto-detects the serial port. If you have multiple devices connected, specify the port:

```bash
# macOS (port name varies by board)
espflash flash led-sectional-firmware.bin --port /dev/cu.usbmodem14101

# Linux
espflash flash led-sectional-firmware.bin --port /dev/ttyUSB0
```

## Step 4: Configure WiFi

On first boot, the device has no WiFi credentials and starts a captive portal:

1. On your phone or laptop, connect to the WiFi network **LED-Sectional-Setup**
2. A setup page should open automatically. If it doesn't, open a browser and go to `http://192.168.4.1`
3. Enter your WiFi network name (SSID) and password
4. Tap **Connect**
5. The device saves your credentials and reboots

Your WiFi credentials are stored in flash memory and persist across reboots and power cycles. You only need to do this once.

## Step 5: Wire the LEDs

Connect your WS2812B LED strip to the ESP32-C3:

| LED Strip Wire | ESP32-C3 Pin |
|----------------|-------------|
| Data (DIN)     | GPIO 2      |
| Power (5V/VCC) | 5V or external supply |
| Ground (GND)   | GND (shared with power supply) |

For strips longer than ~10 LEDs, use an external 5V power supply for the LEDs instead of powering them from the board's USB. Always connect grounds together.

## Step 6: Verify

After flashing and WiFi setup, the device:

1. Boots and connects to your WiFi network
2. Fetches current METAR weather data from aviationweather.gov
3. Lights up each LED based on the flight category of its mapped airport:

| Color | Meaning |
|-------|---------|
| Green | VFR — Visual Flight Rules |
| Blue | MVFR — Marginal VFR |
| Red | IFR — Instrument Flight Rules |
| Magenta | LIFR — Low IFR |
| Yellow | VFR with high winds |
| White flash | Thunderstorm reported |

Weather data refreshes every 15 minutes by default.

You can open a serial monitor to see log output:

```bash
espflash monitor
```

Press `Ctrl+C` to exit.

## Default Airport Layout

The firmware ships with a sample configuration mapping the first few LEDs to legend colors and example airports:

| LED | Code | Color |
|-----|------|-------|
| 0 | LIFR | Magenta (legend) |
| 1 | IFR | Red (legend) |
| 2 | MVFR | Blue (legend) |
| 3 | VFR | Green (legend) |
| 4 | WVFR | Yellow (legend) |
| 5 | KSFO | Live weather |
| 6 | KLAX | Live weather |
| 7 | NULL | Off |

To customize the airport layout for your sectional chart, you'll need to build from source with a modified `cfg.toml`. See [DEVELOPMENT.md](DEVELOPMENT.md) for instructions.

## Troubleshooting

### espflash can't find the device

- Try a different USB cable — many cables are charge-only
- On Linux, check that you're in the `dialout` group: `groups` should list it
- On macOS, install the USB-to-UART driver if your board uses a CP2102 or CH340 chip

### LEDs don't light up

- Verify the data wire is connected to GPIO 2
- Check that the LED strip has power (5V and GND)
- Ensure the data direction is correct — WS2812B strips have an arrow showing signal flow; connect to the input end

### WiFi setup page doesn't appear

- Make sure you're connected to the **LED-Sectional-Setup** network
- Try navigating to `http://192.168.4.1` manually
- The captive portal times out after 3 minutes and reboots. Power cycle the device to try again

### All LEDs show red/orange

This means the METAR data fetch failed. Check that:

- Your WiFi network has internet access
- The device successfully connected to WiFi (check serial monitor with `espflash monitor`)
- The device will retry automatically — wait for the next fetch cycle
