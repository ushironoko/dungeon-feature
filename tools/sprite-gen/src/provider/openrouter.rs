use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};

use super::TIMEOUT;
use crate::error::SpriteGenError;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODEL: &str = "openai/gpt-image-1";

pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    modalities: Vec<String>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ResponseMessage>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ApiError {
    message: Option<String>,
}

impl OpenRouterProvider {
    pub fn new() -> Result<Self, SpriteGenError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| SpriteGenError::MissingApiKey("OPENROUTER_API_KEY"))?;

        let model = std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|e| {
                SpriteGenError::ImageGeneration(format!("failed to build HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    pub async fn generate(&self, prompt: &str) -> Result<Vec<u8>, SpriteGenError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            modalities: vec!["image".to_string()],
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SpriteGenError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let body: ChatResponse = response.json().await?;

        if let Some(err) = body.error {
            return Err(SpriteGenError::Api {
                status: 400,
                message: err
                    .message
                    .unwrap_or_else(|| "unknown API error".to_string()),
            });
        }

        extract_image_from_response(body)
    }
}

fn extract_image_from_response(response: ChatResponse) -> Result<Vec<u8>, SpriteGenError> {
    let choices = response.choices.ok_or(SpriteGenError::NoImageInResponse)?;
    let choice = choices
        .into_iter()
        .next()
        .ok_or(SpriteGenError::NoImageInResponse)?;
    let message = choice.message.ok_or(SpriteGenError::NoImageInResponse)?;
    let content = message.content.ok_or(SpriteGenError::NoImageInResponse)?;

    // content はテキスト or 配列（multimodal レスポンス）
    match content {
        // 配列形式: [{ "type": "image_url", "image_url": { "url": "data:..." } }]
        serde_json::Value::Array(parts) => {
            for part in parts {
                if let Some(image_url) = part
                    .get("image_url")
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str())
                {
                    return decode_data_url(image_url);
                }
                // b64_json 形式
                if let Some(b64) = part.get("b64_json").and_then(|v| v.as_str()) {
                    return Ok(BASE64.decode(b64)?);
                }
            }
            Err(SpriteGenError::NoImageInResponse)
        }
        // 文字列形式: data URL 直接
        serde_json::Value::String(s) => {
            if s.starts_with("data:") {
                decode_data_url(&s)
            } else {
                // Base64 文字列として試行
                Ok(BASE64.decode(&s)?)
            }
        }
        _ => Err(SpriteGenError::NoImageInResponse),
    }
}

fn decode_data_url(url: &str) -> Result<Vec<u8>, SpriteGenError> {
    // data:image/png;base64,AAAA...
    let parts: Vec<&str> = url.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err(SpriteGenError::InvalidDataUrl);
    }
    Ok(BASE64.decode(parts[1])?)
}
