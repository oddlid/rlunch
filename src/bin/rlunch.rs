use anyhow::Result;
use compact_str::CompactString;
use rlunch::{cli, db, scrape};
use sqlx::PgPool;
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
            cli::ServeCommands::Json => run_server_json(listen).await?,
            cli::ServeCommands::Admin => run_server_admin(pool, listen).await?,
            cli::ServeCommands::Html { backend_addr } => {
                run_server_html(listen, backend_addr).await?
            }
        },
    }
    Ok(())
}

// #[tracing::instrument]
async fn run_server_json(addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start JSON server on addr: {addr}");
    Ok(())
}

// #[tracing::instrument]
async fn run_server_admin(_pg: PgPool, addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start ADMIN server on addr: {addr}");

    // let id = Uuid::parse_str("51a3b3cb-b120-4f7a-a8af-8d91f9a94f68")?;
    // let ld = db::get_site_by_id(&pg, id).await?;
    //
    // dbg!(ld);

    Ok(())
}

// #[tracing::instrument]
async fn run_server_html(addr: CompactString, backend_addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start HTML server on addr: {addr}, with backend on: {backend_addr}");
    Ok(())
}
