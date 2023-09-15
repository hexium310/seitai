use anyhow::Result;
use async_trait::async_trait;
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, Input},
};
use voicevox::Bytes;

pub(crate) struct SongbirdAudioProcessor;

#[cfg_attr(test, mockall::automock(type Compressed = Vec<u8>; type Input = Vec<u8>; type Raw = Vec<u8>;))]
#[async_trait]
pub(crate) trait AudioProcessor {
    type Compressed;
    type Input;
    type Raw;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed>;
    fn to_input(&self, compressed: &Self::Compressed) -> Self::Input;
}

#[async_trait]
impl AudioProcessor for SongbirdAudioProcessor {
    type Compressed = Compressed;
    type Input = Input;
    type Raw = Bytes;

    async fn compress(&self, raw: Self::Raw) -> Result<Self::Compressed> {
        let compressed = Compressed::new(raw.into(), Bitrate::BitsPerSecond(128_000)).await?;
        let _ = compressed.raw.spawn_loader();

        Ok(compressed)
    }

    fn to_input(&self, compressed: &Self::Compressed) -> Self::Input {
        compressed.new_handle().into()
    }
}
