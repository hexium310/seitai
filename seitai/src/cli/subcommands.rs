use migration::Migration;
use restarter::Restarter;

mod migration;
mod restarter;

#[derive(clap::Subcommand)]
pub enum Subcommand {
    Migration(Migration),
    Restarter(Restarter),
}
