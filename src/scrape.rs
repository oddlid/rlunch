use crate::{data, db, scrapers};
use anyhow::{anyhow, Result};
use compact_str::CompactString;
use reqwest::{Client, IntoUrl};
use sqlx::PgPool;
use tokio::{
    sync::{broadcast, mpsc},
    task,
};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, trace};
use uuid::Uuid;

// Name your user agent after your app?
// static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
// Pretend to be a real browser
const APP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36";

pub trait RestaurantScraper {
    #[allow(async_fn_in_trait)]
    async fn run(&self) -> Result<ScrapeResult>;

    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub site_id: Uuid,
    pub restaurants: Vec<data::Restaurant>,
}

#[derive(Debug, Clone)]
enum ScrapeCommand {
    Run,
    Shutdown,
}

pub fn get_client() -> Result<Client> {
    Client::builder()
        .user_agent(APP_USER_AGENT)
        .timeout(tokio::time::Duration::from_millis(1500))
        .build()
        .map_err(anyhow::Error::from)
}

pub async fn get<U>(client: &Client, url: U) -> Result<String>
where
    U: IntoUrl,
{
    client
        .get(url)
        .send()
        .await?
        .text()
        .await
        .map_err(anyhow::Error::from)
}

pub async fn run(db: PgPool, schedule: Option<CompactString>) -> Result<()> {
    let shutdown = crate::signals::shutdown_channel().await?;
    let (cmd_tx, _) = broadcast::channel(8); // don't know optimal buffer size yet
    let (res_tx, res_rx) = mpsc::channel::<Result<ScrapeResult>>(100); // same here
    match start_scheduler(schedule, cmd_tx.clone()).await {
        Ok(sched) => run_loop(db, sched, shutdown, cmd_tx, res_tx, res_rx).await,
        Err(e) => {
            trace!("{}: running one-shot scrape", e);
            run_oneshot(db, shutdown, cmd_tx, res_tx, res_rx).await
        }
    }
}

async fn start_scheduler(
    schedule: Option<CompactString>,
    tx: broadcast::Sender<ScrapeCommand>,
) -> Result<JobScheduler> {
    match schedule {
        Some(s) => {
            let sched = JobScheduler::new().await?;
            trace!("Setting up cron job with schedule: {s}");
            sched
                .add(Job::new(s.as_str(), move |uid, _lock| {
                    trace!(id = %uid, "Notifying all scrapers to run");
                    tx.send(ScrapeCommand::Run)
                        .expect("Failed to send scheduled run command");
                })?)
                .await?;
            trace!("Starting cron scheduler");
            sched.start().await?;
            Ok(sched)
        }
        None => Err(anyhow!("empty cron spec")),
    }
}

async fn run_oneshot(
    db: PgPool,
    mut shutdown: broadcast::Receiver<()>,
    cmd_tx: broadcast::Sender<ScrapeCommand>,
    res_tx: mpsc::Sender<Result<ScrapeResult>>,
    mut res_rx: mpsc::Receiver<Result<ScrapeResult>>,
) -> Result<()> {
    let tasks = setup_scrapers(&db, cmd_tx.clone(), res_tx).await?;

    trace!("Triggering scrapers once...");
    cmd_tx.send(ScrapeCommand::Run)?;

    for _ in 0..tasks.len() {
        tokio::select! {
            _ = shutdown.recv() => {
                trace!("Got shutdown signal");
                break;
            },
            res = res_rx.recv() => match res {
                Some(v) => match v {
                    Ok(v) => {
                        // trace!("Scrape OK: {:?}", v);
                        println!("{:#?}", v);
                        // TODO: update DB
                    },
                    Err(e) => {
                        error!(err = e.to_string(), "Scraping failed");
                    },
                },
                None => {
                    trace!("Channel closed, quitting");
                    break;
                }
            },
        }
    }

    stop_scrapers(cmd_tx, tasks).await?;
    db.close().await;

    Ok(())
}

async fn run_loop(
    db: PgPool,
    mut sched: JobScheduler,
    mut shutdown: broadcast::Receiver<()>,
    cmd_tx: broadcast::Sender<ScrapeCommand>,
    res_tx: mpsc::Sender<Result<ScrapeResult>>,
    mut res_rx: mpsc::Receiver<Result<ScrapeResult>>,
) -> Result<()> {
    let tasks = setup_scrapers(&db, cmd_tx.clone(), res_tx).await?;

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                trace!("Got shutdown signal");
                break;
            },
            res = res_rx.recv() => match res {
                Some(v) => match v {
                    Ok(v) => {
                        trace!("Scrape OK: {:?}", v);
                        // TODO: update DB
                    },
                    Err(e) => {
                        error!(err = e.to_string(), "Scraping failed");
                    },
                },
                None => {
                    trace!("Channel closed, quitting");
                    break;
                },
            },
        }
    }

    sched.shutdown().await?;
    stop_scrapers(cmd_tx, tasks).await?;
    db.close().await;

    Ok(())
}

// manual add/remove scraper implementations
async fn setup_scrapers(
    db: &PgPool,
    cmds: broadcast::Sender<ScrapeCommand>,
    results: mpsc::Sender<Result<ScrapeResult>>,
) -> Result<task::JoinSet<()>> {
    let uuid_lh = db::get_site_uuid(&db, "se", "gbg", "lh").await?;

    let mut set = task::JoinSet::new();
    set.spawn(run_scraper(
        scrapers::se::gbg::lh::LHScraper::new(uuid_lh),
        cmds.subscribe(),
        results.clone(),
    ));
    set.spawn(run_scraper(
        scrapers::se::gbg::majorna::MajornaScraper::new(Uuid::new_v4()), // TODO: use uuid fetched from DB for this specific site
        cmds.subscribe(),
        results.clone(),
    ));

    Ok(set)
}

async fn stop_scrapers(
    cmd_tx: broadcast::Sender<ScrapeCommand>,
    mut tasks: task::JoinSet<()>,
) -> Result<()> {
    cmd_tx.send(ScrapeCommand::Shutdown)?;
    // drop(cmd_tx); // this works just as well as sending Shutdown, so might switch...

    // this might pose a problem if there are many scrapers running slow jobs, but I just want to
    // see that they finish as they should for now. Might later skip this and just call shutdown
    // right away.
    // while tasks.join_next().await.is_some() {
    //     trace!("Scraper sub-task finished");
    // }
    tasks.shutdown().await; // likely redundant if also doing join_next
    Ok(())
}

async fn run_scraper(
    scraper: impl RestaurantScraper,
    mut cmds: broadcast::Receiver<ScrapeCommand>,
    results: mpsc::Sender<Result<ScrapeResult>>,
) {
    let name = scraper.name();
    loop {
        match cmds.recv().await {
            Ok(c) => match c {
                ScrapeCommand::Run => {
                    trace!(scraper = name, "Starting scrape...");
                    if let Err(e) = results.send(scraper.run().await).await {
                        error!(scraper = name, err = %e, "Results channel closed, quitting");
                        break;
                    }
                }
                ScrapeCommand::Shutdown => {
                    trace!(scraper = name, "Stopping due to shutdown command");
                    break;
                }
            },
            Err(e) => match e {
                broadcast::error::RecvError::Lagged(_) => {
                    trace!(scraper = name, "Lagging behind, retrying receive...");
                    continue;
                }
                broadcast::error::RecvError::Closed => {
                    trace!(scraper = name, "Stopping due to closed channel");
                    break;
                }
            },
        }
    }
}
