use anyhow::Result;
use async_trait::async_trait;
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, Input},
};
use voicevox::Bytes;

pub(crate) struct SongbirdAudioProcessor;

#[async_trait]
pub(crate) trait AudioProcessor {
    type Raw;
    type Compressed;
    type Input;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed>;
    fn to_input(&self, compressed: &Self::Compressed) -> Self::Input;
}

#[async_trait]
impl AudioProcessor for SongbirdAudioProcessor {
    type Raw = Bytes;
    type Compressed = Compressed;
    type Input = Input;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed> {
        let compressed = Compressed::new(raw.into(), Bitrate::BitsPerSecond(128_000)).await?;
        let _ = compressed.raw.spawn_loader();

        Ok(compressed)
    }

    fn to_input(&self, compressed: &Self::Compressed) -> Self::Input {
        compressed.new_handle().into()
    }
}
