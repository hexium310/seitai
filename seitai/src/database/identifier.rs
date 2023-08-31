use sea_query::Iden;

#[derive(Iden)]
pub(crate) enum User {
    #[iden = "users"]
    Table,
    Id,
    SpeakerId,
}
