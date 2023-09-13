use anyhow::Result;
use async_trait::async_trait;
use voicevox::Bytes;

#[cfg_attr(test, mockall::automock(type Raw = super::tests::DummyRaw;))]
#[async_trait]
pub(crate) trait AudioGenerator {
    type Raw;

    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Self::Raw>;
}

#[async_trait]
impl AudioGenerator for voicevox::audio::AudioGenerator {
    type Raw = Bytes;

    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Self::Raw> {
        let audio = self.generate(speaker, text, speed).await?;
        Ok(audio)
    }
}
