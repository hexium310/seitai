use sea_query::Iden;

#[derive(Iden)]
pub(crate) enum User {
    #[iden = "users"]
    Table,
    Id,
    SpeakerId,
}

#[derive(Iden)]
pub(crate) enum Speaker {
    #[iden = "speakers"]
    Table,
    Id,
    Speed,
}
