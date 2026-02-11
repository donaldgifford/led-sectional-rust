use crate::error::{Error, Result};

/// RGB color representation, compatible with smart-leds RGB8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

// Flight category colors (matching the original C project)
pub const COLOR_VFR: Color = Color::new(0, 255, 0);
pub const COLOR_MVFR: Color = Color::new(0, 0, 255);
pub const COLOR_IFR: Color = Color::new(255, 0, 0);
pub const COLOR_LIFR: Color = Color::new(255, 0, 255);
pub const COLOR_WIND: Color = Color::new(255, 255, 0);
pub const COLOR_UNKNOWN: Color = Color::new(0, 0, 0);
pub const COLOR_LIGHTNING: Color = Color::new(255, 255, 255);

// Status colors
pub const COLOR_CONNECTING: Color = Color::new(255, 165, 0);
pub const COLOR_CONNECTED: Color = Color::new(128, 0, 128);
pub const COLOR_FETCH_ERROR: Color = Color::new(0, 255, 255);

/// Manages the LED color buffer and brightness.
pub struct LedState {
    leds: Vec<Color>,
    brightness: u8,
    lightning_indices: Vec<usize>,
    lightning_saved: Vec<(usize, Color)>,
}

impl LedState {
    pub fn new(num_leds: usize, brightness: u8) -> Self {
        Self {
            leds: vec![COLOR_UNKNOWN; num_leds],
            brightness,
            lightning_indices: Vec::new(),
            lightning_saved: Vec::new(),
        }
    }

    pub fn num_leds(&self) -> usize {
        self.leds.len()
    }

    pub fn set(&mut self, index: usize, color: Color) -> Result<()> {
        if index >= self.leds.len() {
            return Err(Error::LedIndexOutOfBounds {
                index,
                num_leds: self.leds.len(),
            });
        }
        self.leds[index] = color;
        Ok(())
    }

    pub fn get(&self, index: usize) -> Result<Color> {
        self.leds.get(index).copied().ok_or(Error::LedIndexOutOfBounds {
            index,
            num_leds: self.leds.len(),
        })
    }

    pub fn set_all(&mut self, color: Color) {
        self.leds.fill(color);
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Returns the LED buffer with brightness scaling applied.
    pub fn brightness_scaled_buffer(&self) -> Vec<Color> {
        let scale = self.brightness as u16;
        self.leds
            .iter()
            .map(|c| Color {
                r: ((c.r as u16 * scale) / 255) as u8,
                g: ((c.g as u16 * scale) / 255) as u8,
                b: ((c.b as u16 * scale) / 255) as u8,
            })
            .collect()
    }

    // -- Lightning management --

    /// Set which LED indices should flash for lightning.
    pub fn set_lightning_indices(&mut self, indices: Vec<usize>) {
        self.lightning_saved = indices
            .iter()
            .filter_map(|&i| self.leds.get(i).map(|&c| (i, c)))
            .collect();
        self.lightning_indices = indices;
    }

    /// Flash lightning LEDs to white. Returns true if any LEDs were flashed.
    pub fn apply_lightning_flash(&mut self) -> bool {
        if self.lightning_indices.is_empty() {
            return false;
        }
        // Save current colors before flashing
        self.lightning_saved = self
            .lightning_indices
            .iter()
            .filter_map(|&i| self.leds.get(i).map(|&c| (i, c)))
            .collect();
        for &idx in &self.lightning_indices {
            if idx < self.leds.len() {
                self.leds[idx] = COLOR_LIGHTNING;
            }
        }
        true
    }

    /// Restore lightning LEDs to their pre-flash colors.
    pub fn restore_lightning(&mut self) {
        for &(idx, color) in &self.lightning_saved {
            if idx < self.leds.len() {
                self.leds[idx] = color;
            }
        }
    }
}

/// Determine LED color for a flight category.
pub fn flight_category_color(
    category: Option<&str>,
    wind_speed: Option<u32>,
    wind_gust: Option<u32>,
    wind_threshold: u32,
    do_winds: bool,
) -> Color {
    let max_wind = wind_speed.unwrap_or(0).max(wind_gust.unwrap_or(0));
    let is_windy = max_wind > wind_threshold;

    match category {
        Some("VFR") if is_windy && do_winds => COLOR_WIND,
        Some("VFR") => COLOR_VFR,
        Some("MVFR") => COLOR_MVFR,
        Some("IFR") => COLOR_IFR,
        Some("LIFR") => COLOR_LIFR,
        _ => COLOR_UNKNOWN,
    }
}

/// Return the static legend color for a special airport code, or None for real airports.
pub fn special_code_color(code: &str) -> Option<Color> {
    match code {
        "VFR" => Some(COLOR_VFR),
        "MVFR" => Some(COLOR_MVFR),
        "IFR" => Some(COLOR_IFR),
        "LIFR" => Some(COLOR_LIFR),
        "WVFR" => Some(COLOR_WIND),
        "LTNG" => Some(COLOR_VFR), // Lightning demo shows green, flashes white
        "NULL" => Some(COLOR_UNKNOWN),
        _ => None,
    }
}

/// Update LED state from config and METAR reports. Returns lightning LED indices.
pub fn update_leds_from_metars(
    led_state: &mut LedState,
    airports: &[crate::config::Airport],
    metars: &std::collections::HashMap<String, crate::metar::MetarReport>,
    wind_threshold: u32,
    do_winds: bool,
) -> Vec<usize> {
    let mut lightning_indices = Vec::new();

    for (i, airport) in airports.iter().enumerate() {
        if i >= led_state.num_leds() {
            break;
        }

        if let Some(color) = special_code_color(&airport.code) {
            let _ = led_state.set(i, color);
            // LTNG special code always flashes
            if airport.code == "LTNG" {
                lightning_indices.push(i);
            }
        } else if let Some(metar) = metars.get(&airport.code) {
            let color = flight_category_color(
                metar.flt_cat.as_deref(),
                metar.wspd,
                metar.wgst,
                wind_threshold,
                do_winds,
            );
            let _ = led_state.set(i, color);

            if metar.has_thunderstorm() {
                lightning_indices.push(i);
            }
        } else {
            let _ = led_state.set(i, COLOR_UNKNOWN);
        }
    }

    lightning_indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_constants_match_original() {
        assert_eq!(COLOR_VFR, Color::new(0, 255, 0));
        assert_eq!(COLOR_MVFR, Color::new(0, 0, 255));
        assert_eq!(COLOR_IFR, Color::new(255, 0, 0));
        assert_eq!(COLOR_LIFR, Color::new(255, 0, 255));
        assert_eq!(COLOR_WIND, Color::new(255, 255, 0));
        assert_eq!(COLOR_UNKNOWN, Color::new(0, 0, 0));
        assert_eq!(COLOR_LIGHTNING, Color::new(255, 255, 255));
    }

    #[test]
    fn led_state_new() {
        let state = LedState::new(5, 128);
        assert_eq!(state.num_leds(), 5);
        assert_eq!(state.brightness(), 128);
        for i in 0..5 {
            assert_eq!(state.get(i).unwrap(), COLOR_UNKNOWN);
        }
    }

    #[test]
    fn led_state_set_and_get() {
        let mut state = LedState::new(3, 255);
        state.set(0, COLOR_VFR).unwrap();
        state.set(1, COLOR_IFR).unwrap();
        state.set(2, COLOR_MVFR).unwrap();
        assert_eq!(state.get(0).unwrap(), COLOR_VFR);
        assert_eq!(state.get(1).unwrap(), COLOR_IFR);
        assert_eq!(state.get(2).unwrap(), COLOR_MVFR);
    }

    #[test]
    fn led_state_out_of_bounds() {
        let mut state = LedState::new(2, 255);
        assert!(state.set(2, COLOR_VFR).is_err());
        assert!(state.get(2).is_err());
    }

    #[test]
    fn led_state_set_all() {
        let mut state = LedState::new(3, 255);
        state.set_all(COLOR_IFR);
        for i in 0..3 {
            assert_eq!(state.get(i).unwrap(), COLOR_IFR);
        }
    }

    #[test]
    fn brightness_scaling_full() {
        let mut state = LedState::new(1, 255);
        state.set(0, Color::new(255, 128, 64)).unwrap();
        let scaled = state.brightness_scaled_buffer();
        assert_eq!(scaled[0], Color::new(255, 128, 64));
    }

    #[test]
    fn brightness_scaling_half() {
        let mut state = LedState::new(1, 128);
        state.set(0, Color::new(255, 255, 255)).unwrap();
        let scaled = state.brightness_scaled_buffer();
        // 255 * 128 / 255 = 128
        assert_eq!(scaled[0].r, 128);
        assert_eq!(scaled[0].g, 128);
        assert_eq!(scaled[0].b, 128);
    }

    #[test]
    fn brightness_scaling_zero() {
        let mut state = LedState::new(1, 0);
        state.set(0, Color::new(255, 255, 255)).unwrap();
        let scaled = state.brightness_scaled_buffer();
        assert_eq!(scaled[0], Color::new(0, 0, 0));
    }

    #[test]
    fn flight_category_colors() {
        assert_eq!(flight_category_color(Some("VFR"), None, None, 25, true), COLOR_VFR);
        assert_eq!(flight_category_color(Some("MVFR"), None, None, 25, true), COLOR_MVFR);
        assert_eq!(flight_category_color(Some("IFR"), None, None, 25, true), COLOR_IFR);
        assert_eq!(flight_category_color(Some("LIFR"), None, None, 25, true), COLOR_LIFR);
        assert_eq!(flight_category_color(None, None, None, 25, true), COLOR_UNKNOWN);
        assert_eq!(flight_category_color(Some("GARBAGE"), None, None, 25, true), COLOR_UNKNOWN);
    }

    #[test]
    fn flight_category_wind_override() {
        // VFR with high wind -> yellow
        assert_eq!(
            flight_category_color(Some("VFR"), Some(30), None, 25, true),
            COLOR_WIND
        );
        // VFR with high gust -> yellow
        assert_eq!(
            flight_category_color(Some("VFR"), Some(10), Some(30), 25, true),
            COLOR_WIND
        );
        // VFR with high wind but do_winds=false -> green
        assert_eq!(
            flight_category_color(Some("VFR"), Some(30), None, 25, false),
            COLOR_VFR
        );
        // IFR with high wind -> still red (wind override only affects VFR)
        assert_eq!(
            flight_category_color(Some("IFR"), Some(30), None, 25, true),
            COLOR_IFR
        );
    }

    #[test]
    fn special_code_colors() {
        assert_eq!(special_code_color("VFR"), Some(COLOR_VFR));
        assert_eq!(special_code_color("MVFR"), Some(COLOR_MVFR));
        assert_eq!(special_code_color("IFR"), Some(COLOR_IFR));
        assert_eq!(special_code_color("LIFR"), Some(COLOR_LIFR));
        assert_eq!(special_code_color("WVFR"), Some(COLOR_WIND));
        assert_eq!(special_code_color("LTNG"), Some(COLOR_VFR));
        assert_eq!(special_code_color("NULL"), Some(COLOR_UNKNOWN));
        assert_eq!(special_code_color("KSFO"), None);
    }

    #[test]
    fn lightning_flash_and_restore() {
        let mut state = LedState::new(3, 255);
        state.set(0, COLOR_VFR).unwrap();
        state.set(1, COLOR_IFR).unwrap();
        state.set(2, COLOR_MVFR).unwrap();

        state.set_lightning_indices(vec![0, 2]);

        assert!(state.apply_lightning_flash());
        assert_eq!(state.get(0).unwrap(), COLOR_LIGHTNING);
        assert_eq!(state.get(1).unwrap(), COLOR_IFR); // unaffected
        assert_eq!(state.get(2).unwrap(), COLOR_LIGHTNING);

        state.restore_lightning();
        assert_eq!(state.get(0).unwrap(), COLOR_VFR);
        assert_eq!(state.get(1).unwrap(), COLOR_IFR);
        assert_eq!(state.get(2).unwrap(), COLOR_MVFR);
    }

    #[test]
    fn lightning_no_indices() {
        let mut state = LedState::new(3, 255);
        state.set_lightning_indices(vec![]);
        assert!(!state.apply_lightning_flash());
    }
}
