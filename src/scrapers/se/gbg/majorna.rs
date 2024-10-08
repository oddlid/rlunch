// Currently just a dummy implementation to see how it all works with several scrapers

use crate::{
    models::{Dish, Restaurant},
    scrape::{RestaurantScraper, ScrapeResult},
};
use anyhow::Result;
use chrono::Local;
use std::time::Duration;
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

        let mut r = Restaurant {
            site_id: self.site_id,
            restaurant_id: Uuid::new_v4(),
            name: String::from("Old Town"),
            comment: Some(String::from("Second home")),
            address: Some(String::from("Godhemsgatan 7, 414 68 Göteborg")),
            url: Some(String::from("https://www.oldtown.se/")),
            map_url: Some(String::from(
                "https://www.google.se/maps/place/Godhemsgatan+7,+414+68+G%C3%B6teborg",
            )),
            parsed_at: Local::now(),
            ..Default::default()
        };

        let id = Uuid::new_v4();
        r.dishes.insert(
            id,
            Dish {
                dish_id: id,
                restaurant_id: r.restaurant_id,
                name: String::from("Grekiskt"),
                description: Some(String::from("med stor stark")),
                comment: Some(String::from("kan innehålla grävling")),
                tags: ["kött", "gris"].map(String::from).to_vec(),
                price: 149.0,
            },
        );
        let id = Uuid::new_v4();
        r.dishes.insert(
            id,
            Dish {
                dish_id: id,
                restaurant_id: r.restaurant_id,
                name: String::from("Pizza"),
                description: Some(String::from("med saker")),
                comment: Some(String::from("kan innehålla rotta")),
                tags: Vec::new(),
                price: 89.0,
            },
        );

        Ok(ScrapeResult {
            site_id: self.site_id,
            restaurants: vec![r],
        })
    }
}
