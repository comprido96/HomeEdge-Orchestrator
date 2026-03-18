use std::env;
use tracing_subscriber::{fmt, EnvFilter};


pub fn init_tracing(log_level: &str, log_format: &str) {
    let env_filter =
        EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    // eprintln! is intentional here: tracing is not yet initialized at this point,
    // so tracing::warn! would be silently dropped. stderr is the only reliable output.
    match log_format {
        "json" => {
            fmt()
                .with_env_filter(env_filter)
                .json()
                .init();
        }
        "pretty" => {
            fmt()
                .with_env_filter(env_filter)
                .pretty()
                .init();
        }
        other => {
            eprintln!(
                "invalid LOG_FORMAT '{other}', falling back to 'pretty'"
            );

            fmt()
                .with_env_filter(env_filter)
                .pretty()
                .init();
        }
    }
}
