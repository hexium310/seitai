use std::{str::FromStr, hash::Hash, marker::PhantomData};

use anyhow::Result;
use async_trait::async_trait;
use hashbrown::HashMap;
use ordered_float::NotNan;

use self::{cache::CacheTarget, generator::AudioGenerator, processor::AudioProcessor};

pub mod cache;
pub mod generator;
pub mod processor;

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

        if CacheTarget::from_str(&audio.text).is_ok() {
            let compressed = self.audio_processor.compress(raw).await?;
            let raw = self.audio_processor.raw(&compressed);
            self.cache.insert(audio, compressed);
            return Ok(raw);
        }

        Ok(raw)
    }
}
