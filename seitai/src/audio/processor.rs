use anyhow::Result;
use async_trait::async_trait;
use songbird::{driver::Bitrate, input::{cached::Compressed, Input}};

pub(crate) struct SongbirdAudioProcessor;

#[async_trait]
pub(crate) trait AudioProcessor<R, C> {
    async fn compress(&self, raw: R) -> Result<C>;
    fn raw(&self, compressed: &C) -> R;
}

#[async_trait]
impl AudioProcessor<Input, Compressed> for SongbirdAudioProcessor {
    async fn compress(&self, raw: Input) -> Result<Compressed> {
        let compressed = Compressed::new(raw, Bitrate::BitsPerSecond(128_000)).await?;
        let _ = compressed.raw.spawn_loader();

        Ok(compressed)
    }

    fn raw(&self, compressed: &Compressed) -> Input {
        compressed.new_handle().into()
    }
}
