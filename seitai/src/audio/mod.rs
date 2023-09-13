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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[cfg(test)]
mod tests {
    // use mockall::predicate;
    use ordered_float::NotNan;

    use super::{Audio, AudioRepository, VoicevoxAudioRepository};
    use crate::audio::{cache::CacheTarget, generator::MockAudioGenerator, processor::MockAudioProcessor};

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct DummyCompressed;
    #[derive(Debug, PartialEq)]
    pub(crate) struct DummyInput;
    #[derive(Debug, PartialEq)]
    pub(crate) struct DummyRaw;

    impl From<DummyRaw> for DummyInput {
        fn from(_value: DummyRaw) -> Self {
            Self
        }
    }

    #[tokio::test]
    async fn get_audio() {
        let audio = Audio {
            text: "foo".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::default(),
        };

        let mut generator_mock = MockAudioGenerator::new();
        generator_mock
            .expect_generate()
            // .with(predicate::eq(audio.clone().speaker), predicate::eq(audio.clone().text), predicate::eq(*audio.speed))
            .times(1)
            .returning(|_, _, _| Ok(DummyRaw));

        let mut processor_mock = MockAudioProcessor::new();
        processor_mock
            .expect_compress()
            .times(0)
            .returning(|_| Ok(DummyCompressed));
        processor_mock
            .expect_to_input()
            .times(0)
            .returning(|_| DummyInput);

        let audio_repository =
            VoicevoxAudioRepository::<CacheTarget, DummyCompressed, _, DummyInput, _, DummyRaw>::new(generator_mock, processor_mock);

        let input = audio_repository.get(audio).await;
        assert_eq!(input.unwrap(), DummyInput);
    }

    #[tokio::test]
    async fn get_and_cache_audio() {
        let audio = Audio {
            text: "URL".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::default(),
        };

        let mut generator_mock = MockAudioGenerator::new();
        generator_mock
            .expect_generate()
            // .with(predicate::eq(audio.clone().speaker), predicate::eq(audio.clone().text), predicate::eq(*audio.speed))
            .times(1)
            .returning(|_, _, _| Ok(DummyRaw));

        let mut processor_mock = MockAudioProcessor::new();
        processor_mock
            .expect_compress()
            .times(1)
            .returning(|_| Ok(DummyCompressed));
        processor_mock
            .expect_to_input()
            .times(1)
            .returning(|_| DummyInput);

        let audio_repository =
            VoicevoxAudioRepository::<CacheTarget, DummyCompressed, _, DummyInput, _, DummyRaw>::new(generator_mock, processor_mock);

        let input = audio_repository.get(audio.clone()).await;
        assert_eq!(input.unwrap(), DummyInput);

        let cached = {
            audio_repository.cache.lock().unwrap().get(&audio).cloned()
        };
        assert_eq!(cached, Some(DummyCompressed));
    }

    #[tokio::test]
    async fn get_cached_audio() {
        let audio = Audio {
            text: "URL".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::default(),
        };

        let mut generator_mock = MockAudioGenerator::new();
        generator_mock
            .expect_generate()
            // .with(predicate::eq(audio.clone().speaker), predicate::eq(audio.clone().text), predicate::eq(*audio.speed))
            .times(0)
            .returning(|_, _, _| Ok(DummyRaw));

        let mut processor_mock = MockAudioProcessor::new();
        processor_mock
            .expect_compress()
            .times(0)
            .returning(|_| Ok(DummyCompressed));
        processor_mock
            .expect_to_input()
            .times(1)
            .returning(|_| DummyInput);

        let audio_repository =
            VoicevoxAudioRepository::<CacheTarget, DummyCompressed, _, DummyInput, _, DummyRaw>::new(generator_mock, processor_mock);

        {
            let mut cache = audio_repository.cache.lock().unwrap();
            cache.insert(audio.clone(), DummyCompressed);
        }

        let input = audio_repository.get(audio).await;
        assert_eq!(input.unwrap(), DummyInput);
    }
}
