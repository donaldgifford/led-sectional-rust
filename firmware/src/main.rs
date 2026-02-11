mod led_driver;
mod wifi;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use led_sectional_core::config::Config;
use led_sectional_core::led::{
    update_leds_from_metars, Color, LedState, COLOR_CONNECTED, COLOR_CONNECTING, COLOR_FETCH_ERROR,
};
use log::{error, info, warn};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default config used when no config file is available on flash.
const DEFAULT_CONFIG_TOML: &str = include_str!("../../cfg.toml.example");

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("LED Sectional booting...");

    let peripherals = Peripherals::take().expect("failed to take peripherals");
    let sysloop = EspSystemEventLoop::take().expect("failed to take event loop");
    let nvs = EspDefaultNvsPartition::take().expect("failed to take NVS partition");

    // Load config (from flash filesystem in production, fallback to built-in default)
    let config = Config::from_toml(DEFAULT_CONFIG_TOML).expect("failed to parse default config");
    info!("Config loaded: {} airports, {} LEDs", config.airports.len(), config.num_leds());

    // Initialize LED state
    let mut led_state = LedState::new(config.num_leds(), config.settings.brightness);
    led_state.set_all(COLOR_CONNECTING);
    // TODO: write to hardware via led_driver

    // Try to connect WiFi
    let (ssid, password) = resolve_wifi_credentials(&nvs, &config);
    if let Some((ssid, password)) = ssid.zip(password) {
        let mut wifi_mgr = wifi::WifiManager::new(peripherals.modem, sysloop, nvs)
            .expect("failed to create WiFi manager");

        match wifi_mgr.connect_sta(&ssid, &password) {
            Ok(()) => {
                info!("WiFi connected");
                led_state.set_all(COLOR_CONNECTED);
                // TODO: write to hardware
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                error!("WiFi connection failed: {:?}", e);
                led_state.set_all(COLOR_FETCH_ERROR);
                // TODO: write to hardware, enter provisioning
            }
        }
    } else {
        warn!("No WiFi credentials found — provisioning needed");
        // TODO: Phase 8 captive portal provisioning
    }

    info!("Entering main loop");

    let fetch_interval = Duration::from_secs(config.settings.request_interval_secs);
    let mut last_fetch = Instant::now() - fetch_interval; // Force immediate first fetch

    loop {
        if last_fetch.elapsed() >= fetch_interval {
            info!("METAR fetch interval reached — fetching...");
            // TODO: Phase 5 METAR client fetch
            // For now, just reset the timer
            last_fetch = Instant::now();
        }

        // Lightning animation
        if config.settings.do_lightning {
            if led_state.apply_lightning_flash() {
                // TODO: write to hardware
                std::thread::sleep(Duration::from_millis(25));
                led_state.restore_lightning();
                // TODO: write to hardware
            }
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Resolve WiFi credentials: NVS first, then TOML config fallback.
fn resolve_wifi_credentials(
    nvs: &EspDefaultNvsPartition,
    config: &Config,
) -> (Option<String>, Option<String>) {
    // Try NVS first
    match wifi::load_credentials(nvs.clone()) {
        Ok(Some((ssid, password))) => return (Some(ssid), Some(password)),
        Ok(None) => {}
        Err(e) => warn!("Failed to load NVS credentials: {:?}", e),
    }

    // Fall back to TOML config
    (
        config.wifi.ssid.clone(),
        config.wifi.password.clone().or(Some(String::new())),
    )
}
