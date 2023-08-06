use time::macros::format_description;
use tracing_subscriber::{fmt::time::LocalTime, EnvFilter};

pub fn initialize_logging() {
    let local_timer = LocalTime::new(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]+[offset_hour]:[offset_minute]"
    ));
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(local_timer)
        .init();
}
