use anyhow::Result;
use tracing::subscriber;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init() -> Result<()> {
    let builder = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .compact();

    if subscriber::set_global_default(builder.finish()).is_err() {
        // Someone else configured logging; fall back silently.
    }

    Ok(())
}
