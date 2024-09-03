// Currently just a dummy implementation to see how it all works with several scrapers

use crate::{
    data::{Dish, Restaurant},
    scrape::{RestaurantScraper, ScrapeResult},
};
use anyhow::{bail, Result};
use chrono::Local;
use compact_str::CompactString;
use std::{collections::HashSet, time::Duration};
use tracing::trace;
use uuid::Uuid;

#[derive(Default, Clone, Debug)]
pub struct MajornaScraper {
    site_id: Uuid,
    request_delay: Duration,
}

impl MajornaScraper {
    pub fn new(site_id: Uuid, request_delay: Duration) -> Self {
        Self {
            site_id,
            request_delay,
        }
    }
}

impl RestaurantScraper for MajornaScraper {
    fn name(&self) -> &'static str {
        "SE::GBG::Majorna::Scraper"
    }

    async fn run(&self) -> Result<ScrapeResult> {
        trace!(?self.request_delay, "Faking taking time to do work...");
        tokio::time::sleep(self.request_delay).await;

        if rand::random() {
            bail!("{}: Randomly generated error", self.name());
        }
        Ok(ScrapeResult {
            site_id: self.site_id,
            restaurants: vec![Restaurant {
                name: CompactString::from("Old Town"),
                comment: Some(CompactString::from("Second home")),
                address: Some(CompactString::from("Godhemsgatan 7, 414 68 Göteborg")),
                url: Some(CompactString::from("https://www.oldtown.se/")),
                map_url: Some(CompactString::from(
                    "https://www.google.se/maps/place/Godhemsgatan+7,+414+68+G%C3%B6teborg",
                )),
                parsed_at: Local::now(),
                dishes: vec![
                    Dish {
                        name: CompactString::from("Grekiskt"),
                        description: Some(CompactString::from("med stor stark")),
                        comment: Some(CompactString::from("kan innehålla grävling")),
                        tags: HashSet::new(),
                        price: 149.0,
                    },
                    Dish {
                        name: CompactString::from("Pizza"),
                        description: Some(CompactString::from("med saker")),
                        comment: Some(CompactString::from("kan innehålla rotta")),
                        tags: HashSet::new(),
                        price: 89.0,
                    },
                ],
            }],
        })
    }
}
