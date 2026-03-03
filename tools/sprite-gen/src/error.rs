use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpriteGenError {
    #[error("Missing API key: {0}")]
    MissingApiKey(&'static str),

    #[error("API error (HTTP {status}): {message}")]
    Api { status: u16, message: String },

    #[error("No image in API response")]
    NoImageInResponse,

    #[error("Invalid data URL format")]
    InvalidDataUrl,

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Image generation failed: {0}")]
    ImageGeneration(String),

    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    #[error("Sprite '{0}' not found in manifest")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
