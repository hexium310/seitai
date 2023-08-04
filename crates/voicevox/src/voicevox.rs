use anyhow::Result;
use url::Url;

use crate::{audio::AudioGenerator, dictionary::Dictionary};

pub struct Voicevox {
    pub audio_generator: AudioGenerator,
    pub dictionary: Dictionary,
}

impl Voicevox {
    pub fn build(host: &str) -> Result<Self> {
        let base = Url::parse(&format!("http://{host}:50021"))?;

        Ok(Self {
            audio_generator: AudioGenerator {
                base: base.clone(),
                default_speed: 1.2,
            },
            dictionary: Dictionary { base: base.clone() },
        })
    }
}
