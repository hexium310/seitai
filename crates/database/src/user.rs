#[derive(Iden)]
pub(crate) enum DatabaseUser {
    #[iden = "users"]
    Table,
    Id,
    SpeakerId,
}
