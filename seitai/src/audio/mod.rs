use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use anyhow::Result;
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

pub(crate) trait AudioRepository {
    type Input;

    fn get(&self, audio: Audio) -> impl Future<Output = Result<Self::Input>> + Send;
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
    use futures::future::ok;
    use ordered_float::NotNan;

    use super::{Audio, AudioRepository, VoicevoxAudioRepository};
    use crate::audio::{cache::MockCacheable, generator::MockAudioGenerator, processor::MockAudioProcessor};

    #[tokio::test]
    async fn get_audio() {
        let audio = Audio {
            text: "foo".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::new(1.0).unwrap(),
        };

        let mut mock_cacheable = MockCacheable::new();
        mock_cacheable
            .expect_should_cache()
            .times(1)
            .withf(|x| x == "foo")
            .returning(|_| false);

        let mut mock_audio_generator = MockAudioGenerator::new();
        mock_audio_generator
            .expect_generate()
            .times(1)
            .withf(|x, y, z| (x, y, z) == ("1", "foo", &1.0))
            .returning(|_, _, _| Box::pin(ok(vec![0x00, 0x01, 0x02, 0x03])));

        let mock_audio_processor = MockAudioProcessor::new();

        let audio_repository = VoicevoxAudioRepository::new(mock_audio_generator, mock_audio_processor, mock_cacheable);

        let actual = audio_repository.get(audio).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);
    }

    #[tokio::test]
    async fn get_cached_audio() {
        let audio = Audio {
            text: "bar".to_string(),
            speaker: "1".to_string(),
            speed: NotNan::new(1.0).unwrap(),
        };

        let mut mock_cacheable = MockCacheable::new();
        mock_cacheable
            .expect_should_cache()
            .times(1)
            .withf(|x| x == "bar")
            .returning(|_| true);

        let mut mock_audio_generator = MockAudioGenerator::new();
        mock_audio_generator
            .expect_generate()
            .times(1)
            .withf(|x, y, z| (x, y, z) == ("1", "bar", &1.0))
            .returning(|_, _, _| Box::pin(ok(vec![0x00, 0x01, 0x02, 0x03])));

        let mut mock_audio_processor = MockAudioProcessor::new();
        mock_audio_processor
            .expect_compress()
            .times(1)
            .withf(|x| x == &[0x00, 0x01, 0x02, 0x03])
            .returning(|_| Box::pin(ok(vec![0x04, 0x05])));

        mock_audio_processor
            .expect_to_input()
            .times(2)
            .withf(|x| x == &[0x04, 0x05])
            .returning(|_| vec![0x00, 0x01, 0x02, 0x03]);

        let audio_repository = VoicevoxAudioRepository::new(mock_audio_generator, mock_audio_processor, mock_cacheable);

        let actual = audio_repository.get(audio.clone()).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);

        let actual = audio_repository.get(audio).await.unwrap();
        assert_eq!(actual, vec![0x00, 0x01, 0x02, 0x03]);
    }
}
