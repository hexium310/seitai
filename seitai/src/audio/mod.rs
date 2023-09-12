use std::{
    hash::Hash,
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex},
};

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
    cache: Arc<Mutex<HashMap<Audio, C>>>,
    phantom: PhantomData<fn() -> I>,
}

#[async_trait]
pub(crate) trait AudioRepository {
    type Input;
    type Compressed;

    async fn get(&self, audio: Audio) -> Result<Self::Input>;
}

impl<G, P, C, I> VoicevoxAudioRepository<G, P, C, I>
where
    G: AudioGenerator + Send + Sync,
    P: AudioProcessor + Send + Sync,
{
    pub(crate) fn new(audio_generator: G, audio_processor: P) -> Self {
        Self {
            audio_generator,
            audio_processor,
            cache: Arc::new(Mutex::new(HashMap::default())),
            phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<G, P, C, I> AudioRepository for VoicevoxAudioRepository<G, P, C, I>
where
    G: AudioGenerator<Input = I> + Send + Sync,
    P: AudioProcessor<Compressed = C, Raw = I> + Send + Sync,
    I: Send,
    C: Send,
{
    type Input = I;
    type Compressed = C;

    async fn get(&self, audio: Audio) -> Result<Self::Input> {
        if let Some(sound) = self.cache.lock().expect("audio cache has been poisoned").get(&audio) {
            let raw = self.audio_processor.raw(sound);
            return Ok(raw);
        }

        let raw = self
            .audio_generator
            .generate(&audio.speaker, &audio.text, *audio.speed)
            .await?;

        if CacheTarget::from_str(&audio.text).is_ok() {
            let compressed = self.audio_processor.compress(raw).await?;
            let raw = self.audio_processor.raw(&compressed);
            self.cache
                .lock()
                .expect("audio cache has been poisoned")
                .insert(audio, compressed);
            return Ok(raw);
        }

        Ok(raw)
    }
}
