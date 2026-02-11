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

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_CONFIG: &str = r#"
[settings]
brightness = 50
request_interval_secs = 300
wind_threshold_kt = 30
do_lightning = false
do_winds = false
data_pin = 5

[wifi]
ssid = "TestNetwork"
password = "TestPass123"

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

[[airports]]
code = "LTNG"
"#;

    #[test]
    fn parse_full_config() {
        let config = Config::from_toml(FULL_CONFIG).unwrap();
        assert_eq!(config.settings.brightness, 50);
        assert_eq!(config.settings.request_interval_secs, 300);
        assert_eq!(config.settings.wind_threshold_kt, 30);
        assert!(!config.settings.do_lightning);
        assert!(!config.settings.do_winds);
        assert_eq!(config.settings.data_pin, 5);
        assert_eq!(config.wifi.ssid.as_deref(), Some("TestNetwork"));
        assert_eq!(config.wifi.password.as_deref(), Some("TestPass123"));
        assert_eq!(config.airports.len(), 9);
    }

    #[test]
    fn parse_empty_config_uses_defaults() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config.settings.brightness, 20);
        assert_eq!(config.settings.request_interval_secs, 900);
        assert_eq!(config.settings.wind_threshold_kt, 25);
        assert!(config.settings.do_lightning);
        assert!(config.settings.do_winds);
        assert_eq!(config.settings.data_pin, 2);
        assert!(config.wifi.ssid.is_none());
        assert!(config.wifi.password.is_none());
        assert!(config.airports.is_empty());
    }

    #[test]
    fn parse_partial_config() {
        let toml = r#"
[settings]
brightness = 100
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.settings.brightness, 100);
        // Other fields should be defaults
        assert_eq!(config.settings.request_interval_secs, 900);
        assert!(config.settings.do_lightning);
    }

    #[test]
    fn num_leds() {
        let config = Config::from_toml(FULL_CONFIG).unwrap();
        assert_eq!(config.num_leds(), 9);
    }

    #[test]
    fn metar_airport_codes_filters_special() {
        let config = Config::from_toml(FULL_CONFIG).unwrap();
        let codes = config.metar_airport_codes();
        assert_eq!(codes, vec!["KSFO", "KLAX"]);
    }

    #[test]
    fn metar_airport_codes_empty() {
        let toml = r#"
[[airports]]
code = "NULL"

[[airports]]
code = "VFR"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.metar_airport_codes().is_empty());
    }

    #[test]
    fn validation_clamps_interval_low() {
        let toml = r#"
[settings]
request_interval_secs = 10
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.settings.request_interval_secs, 60);
    }

    #[test]
    fn validation_clamps_interval_high() {
        let toml = r#"
[settings]
request_interval_secs = 99999
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.settings.request_interval_secs, 3600);
    }

    #[test]
    fn validation_clamps_wind_threshold() {
        let toml = r#"
[settings]
wind_threshold_kt = 200
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.settings.wind_threshold_kt, 100);
    }

    #[test]
    fn is_special_code_checks() {
        assert!(is_special_code("NULL"));
        assert!(is_special_code("VFR"));
        assert!(is_special_code("MVFR"));
        assert!(is_special_code("IFR"));
        assert!(is_special_code("LIFR"));
        assert!(is_special_code("WVFR"));
        assert!(is_special_code("LTNG"));
        assert!(is_special_code("WBNK"));
        assert!(!is_special_code("KSFO"));
        assert!(!is_special_code("KLAX"));
        assert!(!is_special_code(""));
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = Config::from_toml("{{{{invalid");
        assert!(result.is_err());
    }
}
