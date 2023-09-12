use anyhow::Result;
use async_trait::async_trait;
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, Input},
};

pub(crate) struct SongbirdAudioProcessor;

#[async_trait]
pub(crate) trait AudioProcessor {
    type Raw;
    type Compressed;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed>;
    fn raw(&self, compressed: &Self::Compressed) -> Self::Raw;
}

#[async_trait]
impl AudioProcessor for SongbirdAudioProcessor {
    type Raw = Input;
    type Compressed = Compressed;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed> {
        let compressed = Compressed::new(raw, Bitrate::BitsPerSecond(128_000)).await?;
        let _ = compressed.raw.spawn_loader();

        Ok(compressed)
    }

    fn raw(&self, compressed: &Self::Compressed) -> Self::Raw {
        compressed.new_handle().into()
    }
}
