use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use async_trait::async_trait;
use hashbrown::HashMap;
use ordered_float::NotNan;

use self::{cache::Cacheable, generator::AudioGenerator, processor::AudioProcessor};

pub mod cache;
pub mod generator;
pub mod processor;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct Audio {
    pub(crate) text: String,
    pub(crate) speaker: String,
    pub(crate) speed: NotNan<f32>,
}

pub(crate) struct VoicevoxAudioRepository<AudioCacheable, Compressed, Generator, Input, Processor, Raw> {
    audio_generator: Generator,
    audio_processor: Processor,
    cache: Arc<Mutex<HashMap<Audio, Compressed>>>,
    cacheable: AudioCacheable,
    phantom: PhantomData<fn() -> (Input, Raw)>,
}

#[async_trait]
pub(crate) trait AudioRepository {
    type Input;

    async fn get(&self, audio: Audio) -> Result<Self::Input>;
}

impl<AudioCacheable, Compressed, Generator, Input, Processor, Raw>
    VoicevoxAudioRepository<AudioCacheable, Compressed, Generator, Input, Processor, Raw>
where
    Generator: AudioGenerator + Send + Sync,
    Processor: AudioProcessor + Send + Sync,
{
    pub(crate) fn new(audio_generator: Generator, audio_processor: Processor, cacheable: AudioCacheable) -> Self {
        Self {
            audio_generator,
            audio_processor,
            cache: Arc::new(Mutex::new(HashMap::default())),
            cacheable,
            phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<AudioCacheable, Compressed, Generator, Input, Processor, Raw> AudioRepository
    for VoicevoxAudioRepository<AudioCacheable, Compressed, Generator, Input, Processor, Raw>
where
    AudioCacheable: Cacheable + Send + Sync,
    Compressed: Send,
    Generator: AudioGenerator<Raw = Raw> + Send + Sync,
    Input: Send,
    Processor: AudioProcessor<Compressed = Compressed, Input = Input, Raw = Raw> + Send + Sync,
    Raw: Into<Input> + Send,
{
    type Input = Input;

    async fn get(&self, audio: Audio) -> Result<Self::Input> {
        if let Some(sound) = self.cache.lock().expect("audio cache has been poisoned").get(&audio) {
            let input = self.audio_processor.to_input(sound);
            return Ok(input);
        }

        let raw = self
            .audio_generator
            .generate(&audio.speaker, &audio.text, *audio.speed)
            .await?;

        if self.cacheable.should_cache(&audio.text) {
            let compressed = self.audio_processor.compress(raw).await?;
            let input = self.audio_processor.to_input(&compressed);
            self.cache
                .lock()
                .expect("audio cache has been poisoned")
                .insert(audio, compressed);
            return Ok(input);
        }

        Ok(raw.into())
    }
}
