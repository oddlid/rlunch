use anyhow::Result;
use compact_str::CompactString;
use rlunch::{
    cli, scrape,
    web::{api, html},
};
use sqlx::PgPool;
use tracing::{trace, warn};

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
            cli::ServeCommands::Html { gtag } => run_server_html(pool, listen, gtag).await?,
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
async fn run_server_html(pg: PgPool, addr: CompactString, gtag: CompactString) -> Result<()> {
    html::serve(pg, &addr, gtag).await
}
