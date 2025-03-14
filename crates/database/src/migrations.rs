use std::ops::Deref;

use sqlx::Postgres;
use sqlx_migrator::{Info, migrator, vec_box};

pub use sqlx_migrator::MigrationCommand;

pub mod v1_users_and_speakers;
pub mod v2_soundstickers;

pub struct Migrator {
    inner: migrator::Migrator<Postgres>,
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Migrator {
    type Target = migrator::Migrator<Postgres>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Migrator {
    pub fn new() -> Self {
        let mut migrator = migrator::Migrator::new();
        migrator.add_migrations(vec_box!(
            v1_users_and_speakers::V1Migration,
            v2_soundstickers::V2Migration,
        )).expect("failed to add migrations to migrator");

        Self { inner: migrator }
    }

    pub fn into_boxed_inner(self) -> Box<migrator::Migrator<Postgres>> {
        Box::new(self.inner)
    }
}
