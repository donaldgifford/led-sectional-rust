use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AccessPointConfiguration, BlockingWifi, Configuration, EspWifi};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::wifi;

const AP_SSID: &str = "LED-Sectional-Setup";
const AP_MAX_CONNECTIONS: u16 = 4;
const PORTAL_TIMEOUT_SECS: u64 = 180;

const HTML_FORM: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>LED Sectional Setup</title>
<style>
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:system-ui,sans-serif;background:#1a1a2e;color:#e0e0e0;display:flex;justify-content:center;align-items:center;min-height:100vh;padding:1rem}
.card{background:#16213e;border-radius:12px;padding:2rem;width:100%;max-width:400px;box-shadow:0 4px 24px rgba(0,0,0,.4)}
h1{font-size:1.4rem;margin-bottom:1.5rem;text-align:center;color:#a8d8ea}
label{display:block;margin-bottom:.3rem;font-size:.9rem;color:#a0a0a0}
input[type=text],input[type=password]{width:100%;padding:.7rem;border:1px solid #333;border-radius:6px;background:#0f3460;color:#fff;font-size:1rem;margin-bottom:1rem}
input:focus{outline:none;border-color:#a8d8ea}
button{width:100%;padding:.8rem;border:none;border-radius:6px;background:#e94560;color:#fff;font-size:1rem;cursor:pointer;font-weight:600}
button:hover{background:#c73e54}
p{text-align:center;margin-top:1rem;font-size:.85rem;color:#666}
</style>
</head>
<body>
<div class="card">
<h1>LED Sectional WiFi Setup</h1>
<form method="POST" action="/connect">
<label for="ssid">WiFi Network Name (SSID)</label>
<input type="text" id="ssid" name="ssid" required maxlength="32" autocomplete="off">
<label for="password">Password</label>
<input type="password" id="password" name="password" maxlength="64" autocomplete="off">
<button type="submit">Connect</button>
</form>
<p>Device will reboot after saving credentials.</p>
</div>
</body>
</html>"#;

const HTML_SUCCESS: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>LED Sectional - Saved</title>
<style>
body{font-family:system-ui,sans-serif;background:#1a1a2e;color:#e0e0e0;display:flex;justify-content:center;align-items:center;min-height:100vh}
.card{background:#16213e;border-radius:12px;padding:2rem;text-align:center;max-width:400px}
h1{color:#a8d8ea;margin-bottom:1rem}
</style>
</head>
<body>
<div class="card">
<h1>Credentials Saved</h1>
<p>Rebooting to connect to your WiFi network...</p>
</div>
</body>
</html>"#;

/// Start the captive portal for WiFi provisioning.
///
/// This function blocks until credentials are received or timeout elapses.
/// On successful credential submission, the device reboots.
pub fn start_captive_portal(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting captive portal AP: {}", AP_SSID);

    // Start WiFi in AP mode
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), Some(nvs.clone()))?,
        sysloop,
    )?;

    let ap_config = AccessPointConfiguration {
        ssid: AP_SSID.try_into().unwrap_or_default(),
        max_connections: AP_MAX_CONNECTIONS,
        ..Default::default()
    };

    wifi.set_configuration(&Configuration::AccessPoint(ap_config))?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;
    info!("AP started. IP: {}, SSID: {}", ip_info.ip, AP_SSID);

    // Track whether credentials have been received
    let credentials_received = Arc::new(AtomicBool::new(false));
    let credentials_received_clone = credentials_received.clone();
    let nvs_clone = nvs.clone();

    // Start HTTP server
    let mut server = EspHttpServer::new(&HttpConfig::default())?;

    // GET / — serve the WiFi config form
    server.fn_handler("/", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write_all(HTML_FORM.as_bytes())?;
        Ok(())
    })?;

    // POST /connect — receive credentials, store in NVS, reboot
    server.fn_handler("/connect", Method::Post, move |mut req| {
        // Read the POST body
        let mut body = vec![0u8; 256];
        let len = req.read(&mut body).unwrap_or(0);
        let body_str = String::from_utf8_lossy(&body[..len]);

        // Parse form-urlencoded data
        let (ssid, password) = parse_form_data(&body_str);

        if ssid.is_empty() {
            let mut resp = req.into_response(400, None, &[("Content-Type", "text/plain")])?;
            resp.write_all(b"SSID is required")?;
            return Ok(());
        }

        info!("Received WiFi credentials for SSID: {}", ssid);

        // Store in NVS
        if let Err(e) = wifi::store_credentials(nvs_clone.clone(), &ssid, &password) {
            warn!("Failed to store credentials: {:?}", e);
            let mut resp = req.into_response(500, None, &[("Content-Type", "text/plain")])?;
            resp.write_all(b"Failed to save credentials")?;
            return Ok(());
        }

        // Send success response
        let mut resp = req.into_ok_response()?;
        resp.write_all(HTML_SUCCESS.as_bytes())?;

        credentials_received_clone.store(true, Ordering::Relaxed);
        Ok(())
    })?;

    // Wait for credentials or timeout
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(PORTAL_TIMEOUT_SECS);
    while std::time::Instant::now() < deadline {
        if credentials_received.load(Ordering::Relaxed) {
            info!("Credentials received. Rebooting in 2 seconds...");
            std::thread::sleep(std::time::Duration::from_secs(2));
            // SAFETY: esp_restart() is always safe to call and triggers a clean reboot.
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    warn!("Captive portal timed out after {}s. Rebooting...", PORTAL_TIMEOUT_SECS);
    // SAFETY: esp_restart() is always safe to call and triggers a clean reboot.
    unsafe { esp_idf_svc::sys::esp_restart() };
}

/// Parse form-urlencoded POST body into (ssid, password).
fn parse_form_data(body: &str) -> (String, String) {
    let mut ssid = String::new();
    let mut password = String::new();

    for pair in body.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded = url_decode(value);
            match key {
                "ssid" => ssid = decoded,
                "password" => password = decoded,
                _ => {}
            }
        }
    }

    (ssid, password)
}

/// Basic URL decoding (handles %XX and + for spaces).
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            }
            _ => result.push(c),
        }
    }

    result
}
