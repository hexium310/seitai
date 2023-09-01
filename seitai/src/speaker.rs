use std::fmt;

use anyhow::{bail, Context as _, Result};
use voicevox::{
    speaker::response::{GetSpeakersResult, Speaker as VoicevoxSpeaker},
    Voicevox,
};

#[derive(Debug)]
pub(crate) struct Speaker {
    speakers: Vec<VoicevoxSpeaker>,
}

pub(crate) struct NamePair<'a>(pub(crate) &'a str, pub(crate) &'a str);

impl fmt::Display for NamePair<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}（{}）", self.0, self.1)
    }
}

impl NamePair<'_> {
    pub(crate) fn contains(&self, text: &str) -> bool {
        self.0.contains(text) || self.1.contains(text)
    }
}

impl Speaker {
    pub(crate) async fn build(voicevox: &Voicevox) -> Result<Self> {
        let speakers = match voicevox.speaker.list().await.context("failed to get speakers")? {
            GetSpeakersResult::Ok(speakers) => speakers,
            GetSpeakersResult::UnprocessableEntity(error) => {
                bail!("failed to get speakers\nError: {error:?}");
            },
        };

        Ok(Self { speakers })
    }

    pub(crate) fn get_name(&self, speaker_id: u16) -> Result<String> {
        let (name_pair, _) = self
            .pairs()
            .find(|(_, id)| id == &speaker_id)
            .context("cannot find speaker {speaker_id}")?;

        Ok(format!("{name_pair}"))
    }

    pub(crate) fn pairs(&self) -> impl Iterator<Item = (NamePair, u16)> + '_ {
        Self::to_speaker_tuples(&self.speakers)
    }

    pub(crate) fn default_speed() -> f32 {
        1.2
    }

    fn to_speaker_tuples(speakers: &[VoicevoxSpeaker]) -> impl Iterator<Item = (NamePair, u16)> + '_ {
        speakers.iter().flat_map(|speaker| {
            speaker.styles.iter().map(|style| {
                (NamePair(speaker.name.as_str(), style.name.as_str()), style.id)
            })
        })
    }
}
