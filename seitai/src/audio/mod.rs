use std::{str::FromStr, hash::Hash, marker::PhantomData};

use anyhow::Result;
use async_trait::async_trait;
use hashbrown::HashMap;
use ordered_float::NotNan;

use self::{generator::AudioGenerator, processor::AudioProcessor};

pub mod generator;
pub mod processor;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CacheKey {
    Code,
    Url,
    Connected,
    Attachment,
    Registered,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct Audio {
    pub(crate) text: String,
    pub(crate) speaker: String,
    pub(crate) speed: NotNan<f32>,
}

pub(crate) struct VoicevoxAudioRepository<G, P, C, I> {
    audio_generator: G,
    audio_processor: P,
    cache: HashMap<Audio, C>,
    phantom: PhantomData<fn() -> I>,
}

#[async_trait]
pub(crate) trait AudioRepository<I> {
    async fn get(&mut self, audio: Audio) -> Result<I>;
}

impl<G, P, C, I> VoicevoxAudioRepository<G, P, C, I> {
    pub(crate) fn new(audio_generator: G, audio_processor: P) -> Self {
        Self {
            audio_generator,
            audio_processor,
            cache: HashMap::new(),
            phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<G, P, C, I> AudioRepository<I> for VoicevoxAudioRepository<G, P, C, I>
where
    G: AudioGenerator<I> + Send + Sync,
    P: AudioProcessor<I, C> + Send + Sync,
    C: Send,
    I: Send,
{
    async fn get(&mut self, audio: Audio) -> Result<I> {
        if let Some(sound) = self.cache.get(&audio) {
            let raw = self.audio_processor.raw(sound);
            return Ok(raw);
        }

        let raw = self.audio_generator.generate(&audio.speaker, &audio.text, *audio.speed).await?;

        if CacheKey::from_str(&audio.text).is_ok() {
            let compressed = self.audio_processor.compress(raw).await?;
            let raw = self.audio_processor.raw(&compressed);
            self.cache.insert(audio, compressed);
            return Ok(raw);
        }

        Ok(raw)
    }
}

impl FromStr for CacheKey {
    type Err = ();

    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        match text {
            "コード省略" => Ok(Self::Code),
            "URL" => Ok(Self::Url),
            "接続しました" => Ok(Self::Connected),
            "添付ファイル" => Ok(Self::Attachment),
            "を登録しました" => Ok(Self::Registered),
            _ => Err(()),
        }
    }
}

impl CacheKey {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Code => "コード省略",
            Self::Url => "URL",
            Self::Connected => "接続しました",
            Self::Attachment => "添付ファイル",
            Self::Registered => "を登録しました",
        }
    }
}
