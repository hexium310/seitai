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
    use std::{str::FromStr, sync::Mutex};

    use mockall::{mock, predicate};
    use ordered_float::NotNan;

    use super::{Audio, AudioRepository, VoicevoxAudioRepository};
    use crate::audio::{generator::MockAudioGenerator, processor::MockAudioProcessor};

    mock! {
        CacheTarget {}
        impl FromStr for CacheTarget {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err>;
        }
    }

    static CACHE_TARGET: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn get_audio() {
        let _m = CACHE_TARGET.lock().unwrap();

        let audio = Audio {
            text: "foo".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::new(1.0).unwrap(),
        };

        let mock_cache_target_from_str = MockCacheTarget::from_str_context();
        mock_cache_target_from_str
            .expect()
            .times(1)
            .with(predicate::eq("foo"))
            .returning(|_| Err("no cache"));

        let mut mock_audio_generator = MockAudioGenerator::new();
        mock_audio_generator
            .expect_generate()
            .times(1)
            .with(predicate::eq("1"), predicate::eq("foo"), predicate::eq(1.0))
            .returning(|_, _, _| Ok(vec![0x00, 0x01, 0x02, 0x03]));

        let mock_audio_processor = MockAudioProcessor::new();

        let audio_repository = VoicevoxAudioRepository::<MockCacheTarget, Vec<u8>, _, Vec<u8>, _, Vec<u8>>::new(mock_audio_generator, mock_audio_processor);

        let actual = audio_repository.get(audio).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);
    }

    #[tokio::test]
    async fn get_cached_audio() {
        let _m = CACHE_TARGET.lock().unwrap();

        let audio = Audio {
            text: "bar".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::new(1.0).unwrap(),
        };

        let mock_cache_target_from_str = MockCacheTarget::from_str_context();
        mock_cache_target_from_str
            .expect()
            .times(1)
            .with(predicate::eq("bar"))
            .returning(|_| Ok(MockCacheTarget {}));

        let mut mock_audio_generator = MockAudioGenerator::new();
        mock_audio_generator
            .expect_generate()
            .times(1)
            .with(predicate::eq("1"), predicate::eq("bar"), predicate::eq(1.0))
            .returning(|_, _, _| Ok(vec![0x00, 0x01, 0x02, 0x03]));

        let mut mock_audio_processor = MockAudioProcessor::new();
        mock_audio_processor
            .expect_compress()
            .times(1)
            .with(predicate::eq(vec![0x00, 0x01, 0x02, 0x03]))
            .returning(|_| Ok(vec![0x04, 0x05]));

        mock_audio_processor
            .expect_to_input()
            .times(2)
            .with(predicate::eq(vec![0x04, 0x05]))
            .returning(|_| vec![0x00, 0x01, 0x02, 0x03]);

        let audio_repository = VoicevoxAudioRepository::<MockCacheTarget, Vec<u8>, _, Vec<u8>, _, Vec<u8>>::new(mock_audio_generator, mock_audio_processor);

        let actual = audio_repository.get(audio.clone()).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);

        let actual = audio_repository.get(audio).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);
    }
}
