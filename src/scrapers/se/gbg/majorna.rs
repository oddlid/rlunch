// Currently just a dummy implementation to see how it all works with several scrapers

use std::collections::HashSet;

use crate::{
    data::{Dish, Restaurant},
    scrape::{RestaurantScraper, ScrapeResult},
    util::wait_random_range_ms,
};
use anyhow::{bail, Result};
use chrono::Local;
use compact_str::CompactString;
use tracing::trace;
use uuid::Uuid;

#[derive(Default, Clone, Debug)]
pub struct MajornaScraper {
    site_id: Uuid,
}

impl MajornaScraper {
    pub fn new(site_id: Uuid) -> Self {
        Self { site_id }
    }
}

impl RestaurantScraper for MajornaScraper {
    fn name(&self) -> &'static str {
        "SE::GBG::Majorna::Scraper"
    }

    async fn run(&self) -> Result<ScrapeResult> {
        trace!("Faking taking time to do work...");
        wait_random_range_ms(500, 1000).await;

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
