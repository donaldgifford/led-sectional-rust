use serde::Deserialize;

use crate::error::Result;

const METAR_BASE_URL: &str = "https://aviationweather.gov/api/data/metar?format=json&ids=";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetarReport {
    pub icao_id: String,
    pub flt_cat: Option<String>,
    pub wspd: Option<u32>,
    pub wgst: Option<u32>,
    pub wx_string: Option<String>,
}

impl MetarReport {
    /// Check if the weather string indicates thunderstorms.
    pub fn has_thunderstorm(&self) -> bool {
        self.wx_string
            .as_deref()
            .is_some_and(|wx| wx.contains("TS"))
    }

    /// Return the maximum of wind speed and wind gust.
    pub fn max_wind(&self) -> u32 {
        self.wspd.unwrap_or(0).max(self.wgst.unwrap_or(0))
    }
}

/// Parse a JSON string containing an array of METAR reports.
pub fn parse_metars(json: &str) -> Result<Vec<MetarReport>> {
    let reports: Vec<MetarReport> = serde_json::from_str(json)?;
    Ok(reports)
}

/// Build the METAR API URL for the given airport codes.
pub fn build_metar_url(codes: &[&str]) -> String {
    let mut url = String::from(METAR_BASE_URL);
    url.push_str(&codes.join(","));
    url
}
