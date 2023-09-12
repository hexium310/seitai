use anyhow::Result;
use async_trait::async_trait;
use songbird::input::Input;

#[async_trait]
pub(crate) trait AudioGenerator {
    type Input;

    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Self::Input>;
}

#[async_trait]
impl AudioGenerator for voicevox::audio::AudioGenerator {
    type Input = Input;

    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Self::Input> {
        let audio = self.generate(speaker, text, speed).await?;
        Ok(audio.into())
    }
}
