use sea_query::Iden;

#[derive(Iden)]
pub(crate) enum DatabaseSoundsticker {
    #[iden = "soundstickers"]
    Table,
    Id,
    StickerId,
    SoundId,
}
