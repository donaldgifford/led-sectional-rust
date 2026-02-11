use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::http::Method;
use led_sectional_core::metar::{self, MetarReport};
use log::{debug, info};

const USER_AGENT: &str = "LED-Sectional-Rust/0.1";
const READ_TIMEOUT_MS: u64 = 15_000;
const RESPONSE_BUF_SIZE: usize = 4096;

pub struct MetarClient;

impl MetarClient {
    pub fn new() -> Self {
        Self
    }

    /// Fetch METAR reports for the given airport codes via HTTPS.
    pub fn fetch(&self, airport_codes: &[&str]) -> Result<Vec<MetarReport>, MetarFetchError> {
        if airport_codes.is_empty() {
            return Ok(Vec::new());
        }

        let url = metar::build_metar_url(airport_codes);
        info!("Fetching METARs: {}", url);

        let config = HttpConfig {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            timeout: Some(std::time::Duration::from_millis(READ_TIMEOUT_MS)),
            ..Default::default()
        };

        let mut connection = EspHttpConnection::new(&config)
            .map_err(|e| MetarFetchError::Connection(format!("{e:?}")))?;

        let headers = [("User-Agent", USER_AGENT)];

        connection
            .initiate_request(Method::Get, &url, &headers)
            .map_err(|e| MetarFetchError::Request(format!("{e:?}")))?;

        connection
            .initiate_response()
            .map_err(|e| MetarFetchError::Response(format!("{e:?}")))?;

        let status = connection.status();
        if status != 200 {
            return Err(MetarFetchError::HttpStatus(status));
        }

        // Read response body
        let mut body = Vec::new();
        let mut buf = [0u8; RESPONSE_BUF_SIZE];
        loop {
            use embedded_svc::io::Read;
            let n = connection
                .read(&mut buf)
                .map_err(|e| MetarFetchError::Read(format!("{e:?}")))?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&buf[..n]);
        }

        let body_str = String::from_utf8(body)
            .map_err(|e| MetarFetchError::Utf8(e.to_string()))?;

        debug!("METAR response: {} bytes", body_str.len());

        let reports = metar::parse_metars(&body_str)
            .map_err(|e| MetarFetchError::Parse(e.to_string()))?;

        info!("Parsed {} METAR reports", reports.len());
        Ok(reports)
    }
}

#[derive(Debug)]
pub enum MetarFetchError {
    Connection(String),
    Request(String),
    Response(String),
    HttpStatus(u16),
    Read(String),
    Utf8(String),
    Parse(String),
}

impl std::fmt::Display for MetarFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "HTTP connection error: {e}"),
            Self::Request(e) => write!(f, "HTTP request error: {e}"),
            Self::Response(e) => write!(f, "HTTP response error: {e}"),
            Self::HttpStatus(code) => write!(f, "HTTP status {code}"),
            Self::Read(e) => write!(f, "HTTP read error: {e}"),
            Self::Utf8(e) => write!(f, "UTF-8 decode error: {e}"),
            Self::Parse(e) => write!(f, "JSON parse error: {e}"),
        }
    }
}

impl std::error::Error for MetarFetchError {}
