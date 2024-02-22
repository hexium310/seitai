use std::future::Future;

use anyhow::Result;
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, Input},
};
use voicevox::Bytes;

pub(crate) struct SongbirdAudioProcessor;

#[cfg_attr(test, mockall::automock(type Compressed = Vec<u8>; type Input = Vec<u8>; type Raw = Vec<u8>;))]
pub(crate) trait AudioProcessor {
    type Compressed;
    type Input;
    type Raw;

    fn compress(&self, raw: Self::Raw) -> impl Future<Output = Result<Self::Compressed>> + Send;
    fn to_input(&self, compressed: &Self::Compressed) -> Self::Input;
}

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
