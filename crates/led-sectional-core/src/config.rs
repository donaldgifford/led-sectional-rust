use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub wifi: WifiConfig,
    #[serde(default)]
    pub airports: Vec<Airport>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default = "default_request_interval")]
    pub request_interval_secs: u64,
    #[serde(default = "default_wind_threshold")]
    pub wind_threshold_kt: u32,
    #[serde(default = "default_true")]
    pub do_lightning: bool,
    #[serde(default = "default_true")]
    pub do_winds: bool,
    #[serde(default = "default_data_pin")]
    pub data_pin: u8,
}

#[derive(Debug, Default, Deserialize)]
pub struct WifiConfig {
    pub ssid: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Airport {
    pub code: String,
}

fn default_brightness() -> u8 {
    20
}
fn default_request_interval() -> u64 {
    900
}
fn default_wind_threshold() -> u32 {
    25
}
fn default_true() -> bool {
    true
}
fn default_data_pin() -> u8 {
    2
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness: default_brightness(),
            request_interval_secs: default_request_interval(),
            wind_threshold_kt: default_wind_threshold(),
            do_lightning: default_true(),
            do_winds: default_true(),
            data_pin: default_data_pin(),
        }
    }
}

impl Config {
    pub fn from_toml(s: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(s)?;
        config.validate();
        Ok(config)
    }

    pub fn num_leds(&self) -> usize {
        self.airports.len()
    }

    /// Returns only real ICAO airport codes, filtering out special codes.
    pub fn metar_airport_codes(&self) -> Vec<&str> {
        self.airports
            .iter()
            .filter_map(|a| {
                if is_special_code(&a.code) {
                    None
                } else {
                    Some(a.code.as_str())
                }
            })
            .collect()
    }

    fn validate(&mut self) {
        self.settings.request_interval_secs =
            self.settings.request_interval_secs.clamp(60, 3600);
        self.settings.wind_threshold_kt =
            self.settings.wind_threshold_kt.clamp(0, 100);
    }
}

/// Special codes that are not real ICAO airport identifiers.
const SPECIAL_CODES: &[&str] = &["NULL", "VFR", "MVFR", "IFR", "LIFR", "WVFR", "LTNG", "WBNK"];

pub fn is_special_code(code: &str) -> bool {
    SPECIAL_CODES.contains(&code)
}
