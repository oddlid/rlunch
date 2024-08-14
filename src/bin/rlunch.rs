use anyhow::Result;
use compact_str::CompactString;
use rlunch::cli;
use tracing::{trace, warn};
// use std::time::Duration;
// use tokio::sync::broadcast;
// use tokio_cron_scheduler::{Job, JobScheduler};

// #[derive(Debug, Clone)]
// enum ScrapeCmd {
//     Run,
//     Stop,
// }

#[tokio::main]
async fn main() -> Result<()> {
    let c = cli::Cli::parse_args();
    c.init_logger()?;

    // just for testing log output during development
    cli::test_tracing();

    // let mut tasks = vec![];
    // let mut sig = signals::listen().await?;
    //
    // match &cli.command {
    //     cli::Commands::Scrape {} => scrape_once().await,
    //     cli::Commands::Serve { listen, cron } => match cron {
    //         Some(c) => {
    //             trace!("Listening on: {}", listen);
    //             let schedule = c.clone();
    //             tasks.push(tokio::spawn(async move {
    //                 setup_scrapers(schedule.as_str(), &mut sig).await
    //             }));
    //         }
    //         None => {
    //             error!("Serve without schedule not yet supported");
    //         }
    //     },
    // }
    //
    // let sleep_duration = Duration::from_secs(10);
    // trace!("Sleeping for {:?}...", sleep_duration);
    // tokio::time::sleep(sleep_duration).await;
    //
    // trace!("Stopping tasks...");
    // for h in tasks.drain(..) {
    //     if let Err(e) = h.await? {
    //         error!("Sub task failed: {:?}", e);
    //     }
    // }

    dispatch_commands(c).await?;

    trace!("Main done");

    Ok(())
}

// #[tracing::instrument]
async fn dispatch_commands(c: cli::Cli) -> Result<()> {
    trace!("Checking args and running desired subcommand");
    match c.command {
        cli::Commands::Scrape { cron } => run_scrapers(cron).await?,
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
async fn run_scrapers(schedule: Option<CompactString>) -> Result<()> {
    if let Some(v) = schedule {
        trace!("Running scrapers on schedule: {v}");
    } else {
        trace!("No schedule given, running one-off scrape");
    }
    warn!("TODO: Actually start scrapers");
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

// #[tracing::instrument]
// async fn scrape_once() {
//     warn!("One-off scraping not yet implemented")
// }

// #[tracing::instrument]
// async fn setup_scrapers(
//     schedule: &str,
//     signals: &mut broadcast::Receiver<signals::Signal>,
// ) -> Result<()> {
//     let (tx, rx) = broadcast::channel(2);
//     drop(rx);
//     let tx_run = tx.clone();
//     trace!("Creating scheduler...");
//     let mut sched = JobScheduler::new().await?;
//     trace!("Creating job...");
//     let job = Job::new(schedule, move |uid, _lock| {
//         trace!(id = uid.to_string(), "Notifying all scrapers to run");
//         tx_run.send(ScrapeCmd::Run).unwrap();
//     })?;
//     trace!("Adding job...");
//     sched.add(job).await?;
//
//     let mut tasks = vec![
//         tokio::spawn(run_scraper("S1", tx.subscribe())),
//         tokio::spawn(run_scraper("S2", tx.subscribe())),
//     ];
//
//     trace!("Starting scheduler...");
//     sched.start().await?;
//
//     tokio::select! {
//         sig = signals.recv() => match sig {
//             Ok(s) => {
//                 trace!("Got signal: {:?}", s);
//                 sched.shutdown().await?;
//                 tx.send(ScrapeCmd::Stop)?;
//             },
//             Err(e) => {
//                 error!("Signal error: {}", e);
//                 sched.shutdown().await?;
//                 tx.send(ScrapeCmd::Stop)?;
//             }
//         }
//     }
//
//     for hnd in tasks.drain(..) {
//         hnd.await?;
//     }
//
//     Ok(())
// }

// #[tracing::instrument]
// async fn run_scraper(name: &'static str, mut cmds: broadcast::Receiver<ScrapeCmd>) {
//     loop {
//         let cmd = cmds.recv().await;
//         match cmd {
//             Ok(cmd) => match cmd {
//                 ScrapeCmd::Stop => {
//                     trace!("{} stopping due to stop command", name);
//                     break;
//                 }
//                 ScrapeCmd::Run => trace!("{} running scraper", name),
//             },
//             Err(e) => {
//                 trace!("{} stopping due to error: {:?}", name, e);
//                 break;
//             }
//         }
//     }
// }
