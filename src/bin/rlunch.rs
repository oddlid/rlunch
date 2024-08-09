use anyhow::Result;
use rlunch::{cli, signals};
use std::io;
use tokio::sync::broadcast;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, trace, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(Debug, Clone)]
enum ScrapeCmd {
    Run,
    Stop,
}

// might need to switch out tokio::main for some other variant from whatever web framework I end up
// using
#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse_opts(std::env::args_os())?;
    init_logger(&cli)?;

    match &cli.command {
        cli::Commands::Scrape {} => error!("One-shot scrape not yet implemented"),
        cli::Commands::Serve { listen, cron } => match cron {
            Some(c) => {
                trace!("Listening on: {}", listen);
                let mut sig = signals::listen().await?;
                setup_scrapers(c.as_str(), &mut sig).await?;
            }
            None => {
                error!("Start without schedule not yet supported");
            }
        },
    }

    trace!("Main done");

    Ok(())
}

#[tracing::instrument]
async fn scrape_once() {
    warn!("One-off scraping not yet implemented")
}

#[tracing::instrument]
async fn setup_scrapers(
    schedule: &str,
    signals: &mut broadcast::Receiver<signals::Signal>,
) -> Result<()> {
    let (tx, rx) = broadcast::channel(2);
    drop(rx);
    let tx_run = tx.clone();
    let mut sched = JobScheduler::new().await?;
    let job = Job::new(schedule, move |uuid, _lock| {
        trace!("{}: Notifying all scrapers to run", uuid);
        tx_run.send(ScrapeCmd::Run).unwrap();
    })?;
    sched.add(job).await?;

    let mut handles = vec![
        tokio::spawn(run_scraper("S1", tx.subscribe())),
        tokio::spawn(run_scraper("S2", tx.subscribe())),
    ];

    sched.start().await?;

    tokio::select! {
        sig = signals.recv() => match sig {
            Ok(s) => {
                trace!("Got signal: {:?}", s);
                sched.shutdown().await?;
                tx.send(ScrapeCmd::Stop)?;
            },
            Err(e) => {
                error!("Signal error: {}", e);
                sched.shutdown().await?;
                tx.send(ScrapeCmd::Stop)?;
            }
        }
    }

    for hnd in handles.drain(..) {
        hnd.await?;
    }

    Ok(())
}

#[tracing::instrument]
async fn run_scraper(name: &'static str, mut cmds: broadcast::Receiver<ScrapeCmd>) {
    loop {
        let cmd = cmds.recv().await;
        match cmd {
            Ok(cmd) => match cmd {
                ScrapeCmd::Stop => {
                    trace!("{} stopping due to stop command", name);
                    break;
                }
                ScrapeCmd::Run => trace!("{} running scraper", name),
            },
            Err(e) => {
                trace!("{} stopping due to error: {:?}", name, e);
                break;
            }
        }
    }
}

#[tracing::instrument]
fn init_logger(cli: &cli::Cli) -> Result<()> {
    let layer = match cli.log_format {
        cli::LogFormat::Json => fmt::layer().json().with_writer(io::stderr).boxed(),
        cli::LogFormat::Pretty => fmt::layer().pretty().with_writer(io::stderr).boxed(),
        cli::LogFormat::Compact => fmt::layer()
            .without_time()
            .compact()
            .with_writer(io::stderr)
            .boxed(),
        cli::LogFormat::Normal => fmt::layer().with_writer(io::stderr).boxed(),
    };
    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(cli.tracing_level_filter().into())
                .from_env()?,
        )
        .with(layer)
        .init();
    Ok(())
}
