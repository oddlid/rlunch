use anyhow::Result;
use rlunch::cli;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{filter::LevelFilter, fmt, EnvFilter};

// might need to switch out tokio::main for some other variant from whatever web framework I end up
// using
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse_opts(std::env::args_os())?;
    init_logger(cli.tracing_level_filter())?;

    test_logging().await;
    Ok(())
}

fn init_logger(level_filter: LevelFilter) -> Result<()> {
    // tracing_subscriber::registry()
    //     .with(fmt::layer().json().with_filter(cli.tracing_level_filter()))
    //     .init();
    // fmt()
    //     .json()
    //     .with_max_level(cli.tracing_level_filter())
    //     .init();
    fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(level_filter.into())
                .from_env()?,
        )
        .json()
        .init();
    Ok(())
}

#[tracing::instrument]
async fn test_logging() {
    // let span = span!(Level::TRACE, "test_span");
    // let _guard = span.enter();

    // these are no longer duplicated after removing default-features from
    // the tracing and tracing-subscriber crates, and only enabling what I want
    trace!("Tracing at TRACE level");
    debug!("Tracing at DEBUG level");
    info!("Tracing at INFO level");
    warn!("Tracing at WARN level");
    error!("Tracing at ERROR level");

    // these do nothing with the changes mentioned above
    log::trace!("Log at TRACE level");
    log::debug!("Log at DEBUG level");
    log::info!("Log at INFO level");
    log::warn!("Log at WARN level");
    log::error!("Log at ERROR level");

    // event!(Level::TRACE, "Event log at TRACE level");
    // event!(Level::DEBUG, "Event log at DEBUG level");
    // event!(Level::INFO, "Event log at INFO level");
    // event!(Level::WARN, "Event log at WARN level");
    // event!(Level::ERROR, "Event log at ERROR level");
}
