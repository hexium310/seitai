use std::str::FromStr;

use anyhow::{Context as _, Result};
use hashbrown::HashMap;
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, Input},
};
use voicevox::audio::AudioGenerator;

type Speaker = String;
type Sounds = HashMap<Speaker, Compressed>;
type Caches = HashMap<CacheKey, Sounds>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CacheKey {
    Code,
    Url,
    Connected,
    Attachment,
    Registered,
}

#[derive(Clone)]
pub(crate) struct Sound {
    pub(crate) caches: Caches,
    audio_generator: AudioGenerator,
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

impl Sound {
    pub(crate) fn new(audio_generator: &AudioGenerator, caches: Caches) -> Self {
        Self {
            caches,
            audio_generator: audio_generator.clone(),
        }
    }

    pub(crate) async fn compress(input: Input) -> Result<Compressed> {
        let audio = Compressed::new(input, Bitrate::BitsPerSecond(128_000)).await?;
        let _ = audio.raw.spawn_loader();

        Ok(audio)
    }

    pub(crate) fn store(&mut self, key: CacheKey, speaker: impl Into<String>, sound: Compressed) {
        self.caches.entry(key).or_default().insert(speaker.into(), sound);
    }

    pub(crate) async fn generate(&mut self, text: &str, speaker: &str, speed: f32) -> Result<Input> {
        let cache_key = CacheKey::from_str(text);

        let sound = match cache_key
            .as_ref()
            .ok()
            .and_then(|key| self.caches.get(key).and_then(|sound| sound.get(speaker)))
        {
            Some(sound) => sound.new_handle().into(),
            None => {
                let bytes = self
                    .audio_generator
                    .generate(speaker, text, speed)
                    .await
                    .with_context(|| format!("failed to generate audio with {text}"))?;

                if let Ok(key) = cache_key {
                    self.store(key, speaker, Self::compress(bytes.clone().into()).await?);
                    tracing::info!("cached {text} with speaker = {speaker}");
                }

                bytes.into()
            },
        };

        Ok(sound)
    }
}
