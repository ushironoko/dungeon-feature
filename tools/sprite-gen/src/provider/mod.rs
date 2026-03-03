pub mod openai;
pub mod openrouter;

use std::time::Duration;

use crate::error::SpriteGenError;

use self::openai::OpenAiProvider;
use self::openrouter::OpenRouterProvider;

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;

pub enum Provider {
    OpenRouter(OpenRouterProvider),
    OpenAi(OpenAiProvider),
}

impl Provider {
    pub fn openrouter() -> Result<Self, SpriteGenError> {
        Ok(Self::OpenRouter(OpenRouterProvider::new()?))
    }

    pub fn openai() -> Result<Self, SpriteGenError> {
        Ok(Self::OpenAi(OpenAiProvider::new()?))
    }

    pub async fn generate(&self, prompt: &str) -> Result<Vec<u8>, SpriteGenError> {
        match self {
            Self::OpenRouter(p) => with_retry(|| p.generate(prompt)).await,
            Self::OpenAi(p) => with_retry(|| p.generate(prompt)).await,
        }
    }
}

async fn with_retry<F, Fut>(f: F) -> Result<Vec<u8>, SpriteGenError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, SpriteGenError>>,
{
    let mut last_err = None;

    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let should_retry = match &e {
                    SpriteGenError::Api { status, .. } => *status == 429 || *status >= 500,
                    SpriteGenError::Http(_) => true,
                    _ => false,
                };

                if !should_retry || attempt + 1 >= MAX_RETRIES {
                    return Err(e);
                }

                let delay = BASE_DELAY_MS * 2u64.pow(attempt);
                let jitter = rand_jitter(delay / 4);
                let total_delay = Duration::from_millis(delay + jitter);

                tracing::warn!(
                    attempt = attempt + 1,
                    delay_ms = total_delay.as_millis() as u64,
                    error = %e,
                    "retrying after error"
                );

                tokio::time::sleep(total_delay).await;
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap_or(SpriteGenError::NoImageInResponse))
}

fn rand_jitter(max_ms: u64) -> u64 {
    // Simple jitter using timestamp as seed
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    nanos % max_ms.max(1)
}
