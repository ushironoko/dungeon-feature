use rig::client::ProviderClient;
use rig::image_generation::ImageGenerationModel as _;
use rig::prelude::ImageGenerationClient;
use rig::providers::openai;

use crate::error::SpriteGenError;

pub struct OpenAiProvider {
    model: openai::image_generation::ImageGenerationModel,
}

impl OpenAiProvider {
    pub fn new() -> Result<Self, SpriteGenError> {
        // OPENAI_API_KEY 環境変数を rig が自動で読む
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| SpriteGenError::MissingApiKey("OPENAI_API_KEY"))?;

        let client = openai::Client::from_env();
        let model = client.image_generation_model(openai::GPT_IMAGE_1);

        Ok(Self { model })
    }

    pub async fn generate(&self, prompt: &str) -> Result<Vec<u8>, SpriteGenError> {
        let response = self
            .model
            .image_generation_request()
            .prompt(prompt)
            .width(1024)
            .height(1024)
            .send()
            .await
            .map_err(|e| {
                SpriteGenError::ImageGeneration(format!("OpenAI image generation failed: {}", e))
            })?;

        Ok(response.image)
    }
}
