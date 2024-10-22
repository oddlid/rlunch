// this file is only a scratchpad for testing new scrapers without including them in
// the scraping framework that updates the DB

use std::time::Duration;

use anyhow::Result;
use rlunch::{
    scrape::{get_client, RestaurantScraper},
    scrapers::se::gbg::majorna::MajornaScraper,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let ms = MajornaScraper::new(get_client()?, Uuid::new_v4(), Duration::from_millis(1));
    let res = ms.run().await?;
    dbg!(res);
    Ok(())
}
