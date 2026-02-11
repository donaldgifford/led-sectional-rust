mod led_driver;
mod metar_client;
mod provisioning;
mod wifi;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use led_sectional_core::config::Config;
use led_sectional_core::led::{
    update_leds_from_metars, LedState, COLOR_CONNECTED, COLOR_CONNECTING, COLOR_FETCH_ERROR,
};
use led_sectional_core::metar;
use log::{error, info, warn};
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
    info!(
        "Config loaded: {} airports, {} LEDs",
        config.airports.len(),
        config.num_leds()
    );

    // Initialize LED state
    let mut led_state = LedState::new(config.num_leds(), config.settings.brightness);
    led_state.set_all(COLOR_CONNECTING);
    // TODO: write to hardware via led_driver once GPIO pin is configured

    // Resolve WiFi credentials: NVS first, then TOML config, else provisioning
    let credentials = resolve_wifi_credentials(&nvs, &config);

    match credentials {
        Some((ssid, password)) => {
            // Connect to WiFi
            let mut wifi_mgr =
                wifi::WifiManager::new(peripherals.modem, sysloop, nvs.clone())
                    .expect("failed to create WiFi manager");

            match wifi_mgr.connect_sta(&ssid, &password) {
                Ok(()) => {
                    info!("WiFi connected");
                    led_state.set_all(COLOR_CONNECTED);
                    std::thread::sleep(Duration::from_millis(500));
                }
                Err(e) => {
                    error!("WiFi connection failed: {:?}", e);
                    led_state.set_all(COLOR_FETCH_ERROR);
                    // Connection failed — could enter provisioning here
                    // For MVP, log and continue (will retry on next reboot)
                }
            }

            run_main_loop(&config, &mut led_state);
        }
        None => {
            warn!("No WiFi credentials found — starting captive portal");
            led_state.set_all(COLOR_CONNECTING);

            if let Err(e) =
                provisioning::start_captive_portal(peripherals.modem, sysloop, nvs)
            {
                error!("Captive portal failed: {:?}", e);
            }
            // start_captive_portal reboots on success or timeout, so we shouldn't reach here
        }
    }
}

/// Main application loop: fetch METARs, update LEDs, animate lightning.
fn run_main_loop(config: &Config, led_state: &mut LedState) {
    info!("Entering main loop");

    let airport_codes = config.metar_airport_codes();
    let fetch_interval = Duration::from_secs(config.settings.request_interval_secs);
    let mut last_fetch = Instant::now() - fetch_interval; // Force immediate first fetch
    let client = metar_client::MetarClient::new();

    loop {
        if last_fetch.elapsed() >= fetch_interval {
            info!("Fetching METAR data...");

            let code_refs: Vec<&str> = airport_codes.iter().copied().collect();
            match client.fetch(&code_refs) {
                Ok(reports) => {
                    info!("Received {} METAR reports", reports.len());
                    let metar_map = metar::metars_by_icao(reports);
                    let lightning = update_leds_from_metars(
                        led_state,
                        &config.airports,
                        &metar_map,
                        config.settings.wind_threshold_kt,
                        config.settings.do_winds,
                    );
                    led_state.set_lightning_indices(lightning);
                    last_fetch = Instant::now();
                    // TODO: write to hardware
                }
                Err(e) => {
                    error!("METAR fetch failed: {}", e);
                    led_state.set_all(COLOR_FETCH_ERROR);
                    // TODO: write to hardware
                    // Retry sooner (60 seconds)
                    last_fetch = Instant::now() - fetch_interval + Duration::from_secs(60);
                }
            }
        }

        // Lightning animation
        if config.settings.do_lightning && led_state.apply_lightning_flash() {
            // TODO: write to hardware
            std::thread::sleep(Duration::from_millis(25));
            led_state.restore_lightning();
            // TODO: write to hardware
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Resolve WiFi credentials: NVS first, then TOML config fallback.
fn resolve_wifi_credentials(
    nvs: &EspDefaultNvsPartition,
    config: &Config,
) -> Option<(String, String)> {
    // Try NVS first
    match wifi::load_credentials(nvs.clone()) {
        Ok(Some((ssid, password))) => return Some((ssid, password)),
        Ok(None) => {}
        Err(e) => warn!("Failed to load NVS credentials: {:?}", e),
    }

    // Fall back to TOML config
    config
        .wifi
        .ssid
        .as_ref()
        .map(|ssid| {
            (
                ssid.clone(),
                config.wifi.password.clone().unwrap_or_default(),
            )
        })
}
