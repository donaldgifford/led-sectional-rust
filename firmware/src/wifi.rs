use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};
use log::{info, warn};

const NVS_NAMESPACE: &str = "wifi";
const NVS_KEY_SSID: &str = "ssid";
const NVS_KEY_PASS: &str = "pass";
const CONNECT_TIMEOUT_SECS: u64 = 60;

pub struct WifiManager {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl WifiManager {
    pub fn new(
        modem: Modem,
        sysloop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
    ) -> Result<Self, esp_idf_svc::sys::EspError> {
        let wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
        let wifi = BlockingWifi::wrap(wifi, sysloop)?;
        Ok(Self { wifi })
    }

    pub fn connect_sta(&mut self, ssid: &str, password: &str) -> Result<(), esp_idf_svc::sys::EspError> {
        info!("Connecting to WiFi SSID: {}", ssid);

        let auth = if password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        };

        let config = Configuration::Client(ClientConfiguration {
            ssid: ssid.try_into().unwrap_or_default(),
            password: password.try_into().unwrap_or_default(),
            auth_method: auth,
            ..Default::default()
        });

        self.wifi.set_configuration(&config)?;
        self.wifi.start()?;
        self.wifi.connect()?;
        self.wifi.wait_netif_up()?;

        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        info!("WiFi connected. IP: {}", ip_info.ip);

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.wifi.is_connected().unwrap_or(false)
    }

    pub fn disconnect(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        self.wifi.disconnect()?;
        Ok(())
    }

    /// Get the underlying EspWifi for use with provisioning.
    pub fn into_inner(self) -> BlockingWifi<EspWifi<'static>> {
        self.wifi
    }
}

/// Store WiFi credentials in NVS.
pub fn store_credentials(
    nvs_partition: EspDefaultNvsPartition,
    ssid: &str,
    password: &str,
) -> Result<(), esp_idf_svc::sys::EspError> {
    let mut nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;
    nvs.set_str(NVS_KEY_SSID, ssid)?;
    nvs.set_str(NVS_KEY_PASS, password)?;
    info!("WiFi credentials stored in NVS");
    Ok(())
}

/// Load WiFi credentials from NVS. Returns None if not found.
pub fn load_credentials(
    nvs_partition: EspDefaultNvsPartition,
) -> Result<Option<(String, String)>, esp_idf_svc::sys::EspError> {
    let nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, false)?;

    let mut ssid_buf = [0u8; 64];
    let mut pass_buf = [0u8; 128];

    let ssid = match nvs.get_str(NVS_KEY_SSID, &mut ssid_buf)? {
        Some(s) => s.to_string(),
        None => {
            warn!("No WiFi SSID found in NVS");
            return Ok(None);
        }
    };

    let password = match nvs.get_str(NVS_KEY_PASS, &mut pass_buf)? {
        Some(s) => s.to_string(),
        None => String::new(),
    };

    info!("Loaded WiFi credentials from NVS for SSID: {}", ssid);
    Ok(Some((ssid, password)))
}
