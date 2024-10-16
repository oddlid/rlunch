use anyhow::Result;
use compact_str::CompactString;
use rlunch::{cli, db, scrape, web::api};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{trace, warn};
use uuid::Uuid;

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
        } => scrape::run(pool, cron, request_delay.into()).await?,
        cli::Commands::Serve { listen, commands } => match commands {
            cli::ServeCommands::Json => run_server_json(pool, listen).await?,
            cli::ServeCommands::Admin => run_server_admin(pool, listen).await?,
            cli::ServeCommands::Html { backend_addr } => {
                run_server_html(pool, listen, backend_addr).await?
            }
        },
    }
    Ok(())
}

// #[tracing::instrument]
async fn run_server_json(pg: PgPool, addr: CompactString) -> Result<()> {
    api::serve(pg, &addr).await
}

// #[tracing::instrument]
async fn run_server_admin(pg: PgPool, addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start ADMIN server on addr: {addr}");

    // temp, just testing
    // let start = Instant::now();
    // let ld = db::list_dishes_for_site_by_id(
    //     &pg,
    //     Uuid::parse_str("51a3b3cb-b120-4f7a-a8af-8d91f9a94f68")?,
    // )
    // .await?;
    // let duration = start.elapsed();
    // trace!("Query ran in {:?}", duration);
    // dbg!(ld);

    Ok(())
}

// #[tracing::instrument]
async fn run_server_html(
    _pg: PgPool,
    addr: CompactString,
    backend_addr: CompactString,
) -> Result<()> {
    warn!("TODO: Actually start HTML server on addr: {addr}, with backend on: {backend_addr}");
    Ok(())
}
