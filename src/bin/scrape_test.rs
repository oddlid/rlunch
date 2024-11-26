// this file is only a scratchpad for testing new scrapers without including them in
// the scraping framework that updates the DB

use anyhow::Result;
use rlunch::{cache, cli, scrape::RestaurantScraper, scrapers};
use std::time::Duration;
use tracing::{debug, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::parse_args().init_logger()?;

    // Running this against a local server, starting and stopping it during the runs,
    // it actually seems to work as intended!
    // If the cache file has been saved with contents on a previous run, it will work to start this
    // without the server running, up until TTL expiration, and then we get an error.
    // We can also start with an empty cache file, with the local server running, then stop it in
    // the middle of the run, and start it again before TTL expires, and the cache file will be
    // saved sucessfully with fresh contents.
    // If the the server is stopped long enough for the cache to expire all its entries, and until
    // the end of the loop, the cache file will be saved empty.
    let opts = cache::Opts {
        cache_path: Some("/tmp/scrape_cache.bin".into()),
        cache_capacity: 64,
        cache_ttl: Duration::from_secs(30),
        request_timeout: Duration::from_secs(5),
        request_delay: Duration::from_millis(1500),
    };
    let client = cache::Client::build(opts).await?;
    let scraper = scrapers::se::gbg::lh::LHScraper::new(client.clone(), Uuid::new_v4());
    let sleep_time = Duration::from_secs(5);

    for _ in 0..10 {
        if let Err(e) = scraper.run().await {
            error!(%e);
        }
        debug!("Sleeping {:?} before next scrape", sleep_time);
        tokio::time::sleep(sleep_time).await;
    }

    drop(scraper); // just to be sure nothing else is using the cache instance
    client.save().await?;

    Ok(())
}
