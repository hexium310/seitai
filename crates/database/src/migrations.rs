use sqlx::Postgres;
use sqlx_migrator::migrator;

pub use sqlx_migrator::MigrationCommand;

mod v1_users_and_speakers;
mod v2_soundstickers;
mod v3_soundstickers_index_fix;

pub struct Migrator {
    inner: migrator::Migrator<Postgres>,
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<migrator::Migrator<Postgres>> for Migrator {
    fn as_ref(&self) -> &migrator::Migrator<Postgres> {
        &self.inner
    }
}

impl Migrator {
    pub fn new() -> Self {
        let mut migrator = migrator::Migrator::default();
        migrator
            .add_migration(Box::new(v3_soundstickers_index_fix::V3Migration))
            .expect("failed to add migrations to migrator");

        Self { inner: migrator }
    }
}
