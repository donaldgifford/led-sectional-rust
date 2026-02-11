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

/// Build a HashMap from ICAO ID to MetarReport for quick lookup.
pub fn metars_by_icao(reports: Vec<MetarReport>) -> std::collections::HashMap<String, MetarReport> {
    reports
        .into_iter()
        .map(|r| (r.icao_id.clone(), r))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"[
        {
            "icaoId": "KSFO",
            "fltCat": "VFR",
            "wspd": 12,
            "wgst": null,
            "wxString": "HZ"
        },
        {
            "icaoId": "KLAX",
            "fltCat": "MVFR",
            "wspd": 8,
            "wgst": 20,
            "wxString": null
        },
        {
            "icaoId": "KJFK",
            "fltCat": "IFR",
            "wspd": 15,
            "wgst": 30,
            "wxString": "TS BR"
        }
    ]"#;

    #[test]
    fn parse_valid_json() {
        let reports = parse_metars(SAMPLE_JSON).unwrap();
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].icao_id, "KSFO");
        assert_eq!(reports[0].flt_cat.as_deref(), Some("VFR"));
        assert_eq!(reports[0].wspd, Some(12));
        assert_eq!(reports[0].wgst, None);
        assert_eq!(reports[0].wx_string.as_deref(), Some("HZ"));
    }

    #[test]
    fn parse_empty_array() {
        let reports = parse_metars("[]").unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn parse_null_fields() {
        let json = r#"[{"icaoId": "KORD", "fltCat": null, "wspd": null, "wgst": null, "wxString": null}]"#;
        let reports = parse_metars(json).unwrap();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].flt_cat.is_none());
        assert!(reports[0].wspd.is_none());
        assert!(reports[0].wgst.is_none());
        assert!(reports[0].wx_string.is_none());
    }

    #[test]
    fn parse_invalid_json_errors() {
        assert!(parse_metars("not json").is_err());
        assert!(parse_metars("{\"not\": \"array\"}").is_err());
    }

    #[test]
    fn has_thunderstorm_detects_ts() {
        let reports = parse_metars(SAMPLE_JSON).unwrap();
        assert!(!reports[0].has_thunderstorm()); // HZ - no thunderstorm
        assert!(!reports[1].has_thunderstorm()); // null - no thunderstorm
        assert!(reports[2].has_thunderstorm()); // TS BR - thunderstorm
    }

    #[test]
    fn has_thunderstorm_with_none() {
        let report = MetarReport {
            icao_id: "TEST".to_string(),
            flt_cat: None,
            wspd: None,
            wgst: None,
            wx_string: None,
        };
        assert!(!report.has_thunderstorm());
    }

    #[test]
    fn max_wind_with_both() {
        let reports = parse_metars(SAMPLE_JSON).unwrap();
        assert_eq!(reports[0].max_wind(), 12); // wspd=12, wgst=null
        assert_eq!(reports[1].max_wind(), 20); // wspd=8, wgst=20
        assert_eq!(reports[2].max_wind(), 30); // wspd=15, wgst=30
    }

    #[test]
    fn max_wind_with_none() {
        let report = MetarReport {
            icao_id: "TEST".to_string(),
            flt_cat: None,
            wspd: None,
            wgst: None,
            wx_string: None,
        };
        assert_eq!(report.max_wind(), 0);
    }

    #[test]
    fn build_metar_url_single() {
        let url = build_metar_url(&["KSFO"]);
        assert_eq!(
            url,
            "https://aviationweather.gov/api/data/metar?format=json&ids=KSFO"
        );
    }

    #[test]
    fn build_metar_url_multiple() {
        let url = build_metar_url(&["KSFO", "KLAX", "KJFK"]);
        assert_eq!(
            url,
            "https://aviationweather.gov/api/data/metar?format=json&ids=KSFO,KLAX,KJFK"
        );
    }

    #[test]
    fn build_metar_url_empty() {
        let url = build_metar_url(&[]);
        assert_eq!(
            url,
            "https://aviationweather.gov/api/data/metar?format=json&ids="
        );
    }

    #[test]
    fn metars_by_icao_lookup() {
        let reports = parse_metars(SAMPLE_JSON).unwrap();
        let map = metars_by_icao(reports);
        assert_eq!(map.len(), 3);
        assert!(map.contains_key("KSFO"));
        assert!(map.contains_key("KLAX"));
        assert!(map.contains_key("KJFK"));
        assert_eq!(map["KSFO"].flt_cat.as_deref(), Some("VFR"));
    }
}
