use anyhow::Result;
use compact_str::CompactString;
use rlunch::{cli, scrape};
use tracing::{trace, warn};

#[tokio::main]
async fn main() -> Result<()> {
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
    match c.command {
        cli::Commands::Scrape { cron } => scrape::run(cron).await?,
        cli::Commands::Serve { listen, commands } => match commands {
            cli::ServeCommands::Json => run_server_json(listen).await?,
            cli::ServeCommands::Admin => run_server_admin(listen).await?,
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
async fn run_server_admin(addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start ADMIN server on addr: {addr}");
    Ok(())
}

// #[tracing::instrument]
async fn run_server_html(addr: CompactString, backend_addr: CompactString) -> Result<()> {
    warn!("TODO: Actually start HTML server on addr: {addr}, with backend on: {backend_addr}");
    Ok(())
}
