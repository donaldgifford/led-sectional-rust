use esp_idf_svc::hal::prelude::*;
use log::info;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("LED Sectional booting...");

    let _peripherals = Peripherals::take().expect("failed to take peripherals");

    info!("Initialization complete");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
