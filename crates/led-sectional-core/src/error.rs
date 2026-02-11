use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("LED index {index} out of bounds (num_leds: {num_leds})")]
    LedIndexOutOfBounds { index: usize, num_leds: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
