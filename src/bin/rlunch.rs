use anyhow::Result;
use rlunch::cli;
use std::io;
use tokio::{select, sync::broadcast};
use tracing::{debug, error, info, trace, warn};
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

    let (tx, mut rx1) = broadcast::channel(16);
    let mut rx2 = tx.subscribe();

    let mut hnds = vec![
        tokio::spawn(async move {
            loop {
                match rx2.recv().await {
                    Ok(cmd) => match cmd {
                        ScrapeCmd::Run => {
                            trace!("ScraperOne running scrape");
                        }
                        ScrapeCmd::Stop => {
                            trace!("ScraperOne stopping due to stop command");
                            break;
                        }
                    },
                    Err(e) => {
                        trace!("ScraperOne stopping due to error: {:?}", e);
                        break;
                    }
                }
            }
        }),
        tokio::spawn(async move {
            loop {
                match rx1.recv().await {
                    Ok(cmd) => match cmd {
                        ScrapeCmd::Run => {
                            trace!("ScraperTwo running scrape");
                        }
                        ScrapeCmd::Stop => {
                            trace!("ScraperTwo stopping due to stop command");
                            break;
                        }
                    },
                    Err(e) => {
                        trace!("ScraperTwo stopping due to error: {:?}", e);
                        break;
                    }
                }
            }
        }),
    ];

    tx.send(ScrapeCmd::Run).unwrap();
    tx.send(ScrapeCmd::Stop).unwrap();

    for hnd in hnds.drain(..) {
        hnd.await.unwrap();
    }

    Ok(())
}

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

// async fn run_scraper(
//     name: &'static str,
//     mut cmds: broadcast::Receiver<ScrapeCmd>,
// ) -> tokio::task::JoinHandle<()> {
//     tokio::spawn(async move {
//         loop {
//             let cmd = cmds.recv().await;
//             match cmd {
//                 Ok(cmd) => match cmd {
//                     ScrapeCmd::Stop => {
//                         trace!("{} stopping due to stop command", name);
//                         break;
//                     }
//                     ScrapeCmd::Run => trace!("{} running scraper", name),
//                 },
//                 Err(e) => {
//                     trace!("{} stopping due to error: {:?}", name, e);
//                     break;
//                 }
//             }
//         }
//     })
// }

// #[tracing::instrument]
// async fn test_logging() {
//     // let span = span!(Level::TRACE, "test_span");
//     // let _guard = span.enter();
//
//     // these are no longer duplicated after removing default-features from
//     // the tracing and tracing-subscriber crates, and only enabling what I want
//     trace!("Tracing at TRACE level");
//     debug!("Tracing at DEBUG level");
//     info!("Tracing at INFO level");
//     warn!("Tracing at WARN level");
//     error!("Tracing at ERROR level");
//
//     // these do nothing with the changes mentioned above
//     log::trace!("Log at TRACE level");
//     log::debug!("Log at DEBUG level");
//     log::info!("Log at INFO level");
//     log::warn!("Log at WARN level");
//     log::error!("Log at ERROR level");
//
//     // event!(Level::TRACE, "Event log at TRACE level");
//     // event!(Level::DEBUG, "Event log at DEBUG level");
//     // event!(Level::INFO, "Event log at INFO level");
//     // event!(Level::WARN, "Event log at WARN level");
//     // event!(Level::ERROR, "Event log at ERROR level");
// }
