use anyhow::Result;
use async_trait::async_trait;
use songbird::input::Input;

#[async_trait]
pub(crate) trait AudioGenerator<I> {
    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<I>;
}

#[async_trait]
impl AudioGenerator<Input> for voicevox::audio::AudioGenerator {
    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Input> {
        let audio = self.generate(speaker, text, speed).await?;
        Ok(audio.into())
    }
}
