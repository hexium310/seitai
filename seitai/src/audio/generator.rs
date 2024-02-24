use std::future::Future;

use anyhow::Result;
use voicevox::Bytes;

#[cfg_attr(test, mockall::automock(type Raw = Vec<u8>;))]
pub(crate) trait AudioGenerator {
    type Raw;

    fn generate(&self, speaker: &str, text: &str, speed: f32) -> impl Future<Output = Result<Self::Raw>> + Send;
}

impl AudioGenerator for voicevox::audio::AudioGenerator {
    type Raw = Bytes;

    async fn generate(&self, speaker: &str, text: &str, speed: f32) -> Result<Self::Raw> {
        let audio = self.generate(speaker, text, speed).await?;
        Ok(audio)
    }
}
