use sea_query::Iden;

#[derive(Iden)]
pub(crate) enum DatabaseSpeaker {
    #[iden = "speakers"]
    Table,
    Id,
    Speed,
}
