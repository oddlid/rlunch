use anyhow::Result;
use compact_str::CompactString;
use rlunch::{
    cache, cli, scrape,
    web::{api, html},
};
use sqlx::PgPool;
use tracing::{trace, warn};

// Use Jemalloc only for musl-64 bits platforms
#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        warn!(err = %e, "Failed to load .env file");
    }

    let c = cli::Cli::parse_args();
    c.init_logger()?;

    // just for testing log output during development
    // cli::test_tracing();

    dispatch_commands(c).await?;

    trace!("Main done");

    Ok(())
}

// #[tracing::instrument]
async fn dispatch_commands(c: cli::Cli) -> Result<()> {
    trace!("Checking args and running desired subcommand");
    let pool = c.get_pg_pool().await?;
    match c.command {
        cli::Commands::Scrape {
            cron,
            request_delay,
            request_timeout,
            cache_ttl,
            cache_capacity,
            cache_path,
        } => {
            scrape::run(
                pool,
                cron,
                cache::Opts {
                    request_delay: request_delay.into(),
                    request_timeout: request_timeout.into(),
                    cache_ttl: cache_ttl.into(),
                    cache_capacity,
                    cache_path,
                },
            )
            .await?
        }
        cli::Commands::Serve { listen, commands } => match commands {
            cli::ServeCommands::Json => run_server_json(pool, listen).await?,
            cli::ServeCommands::Admin => run_server_admin(pool, listen).await?,
            cli::ServeCommands::Html => run_server_html(pool, listen).await?,
        },
    }
    Ok(())
}

// #[tracing::instrument]
async fn run_server_json(pg: PgPool, addr: CompactString) -> Result<()> {
    api::serve(pg, &addr).await
}

// #[tracing::instrument]
async fn run_server_admin(_pg: PgPool, addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start ADMIN server on addr: {addr}");

    Ok(())
}

// #[tracing::instrument]
async fn run_server_html(pg: PgPool, addr: CompactString) -> Result<()> {
    html::serve(pg, &addr).await
}
