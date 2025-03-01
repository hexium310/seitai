use error::SoundboardError;
use serenity::all::{GuildId, Http};
use sound::{SoundId, SoundboardSound};

pub use crate::soundboard::*;

pub mod client;
pub mod error;
pub mod sound;
mod soundboard;

pub trait SoundboardExt {
    fn soundboards(self, http: impl AsRef<Http>) -> impl Future<Output = Result<Soundboard, SoundboardError>>;

    fn sound(self, http: impl AsRef<Http>, sound_id: SoundId) -> impl Future<Output = Result<SoundboardSound, SoundboardError>>;
}

impl SoundboardExt for GuildId {
    async fn soundboards(self, http: impl AsRef<Http>) -> Result<Soundboard, SoundboardError> {
        soundboard::soundboards(http, self).await
    }

    async fn sound(self, http: impl AsRef<Http>, sound_id: SoundId) -> Result<SoundboardSound, SoundboardError> {
        soundboard::sound(http, self, sound_id).await
    }
}
