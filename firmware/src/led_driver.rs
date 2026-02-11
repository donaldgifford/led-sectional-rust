use esp_idf_svc::hal::gpio::OutputPin;
use led_sectional_core::led::{Color, LedState};
use smart_leds::SmartLedsWrite;
use smart_leds::RGB8;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

pub struct LedDriver {
    driver: Ws2812Esp32Rmt,
}

impl LedDriver {
    pub fn new(pin: impl OutputPin, channel: u8) -> Result<Self, ws2812_esp32_rmt_driver::LedPixelError> {
        let driver = Ws2812Esp32Rmt::new(channel, pin.pin())?;
        Ok(Self { driver })
    }

    /// Write the current LED state to the hardware strip.
    pub fn write(&mut self, state: &LedState) -> Result<(), ws2812_esp32_rmt_driver::LedPixelError> {
        let buf = state.brightness_scaled_buffer();
        let pixels: Vec<RGB8> = buf.iter().map(|c| to_rgb8(*c)).collect();
        self.driver.write(pixels.into_iter())
    }
}

fn to_rgb8(c: Color) -> RGB8 {
    RGB8 {
        r: c.r,
        g: c.g,
        b: c.b,
    }
}
