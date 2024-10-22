// scraper for oldtown.se

use crate::{
    models::{Dish, Restaurant},
    scrape::{self, RestaurantScraper, ScrapeResult},
    util::*,
};
use anyhow::{bail, Result};
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{selectable::Selectable, ElementRef, Html, Selector};
use std::time::Duration;
use tracing::trace;
use uuid::Uuid;

static URL_PREFIX: &str = "http://localhost:8080";
static EP_GREEK: &str = "/ot_pasta.html"; // "/pasta" irl
static EP_PIZZA: &str = "/ot_meny.html"; // "/meny/" irl
static EP_TALLRIK: &str = "/ot_tallrik.html"; // "/tandoori-kitchen/" irl
static EP_PITA: &str = "/ot_pita.html"; // "/chicken-dishes/" irl
static ERR_INVALID_HTML: &str = "Invalid HTML";

//
lazy_static! {
    static ref SEL_DISH_CONTAINER: Selector = sel("div.mt-i-c.cf.mt-border.line-color");
    static ref SEL_DISH_NAME: Selector = sel("h3");
    static ref SEL_DISH_PRICE: Selector = sel("h3 > strong");
    static ref SEL_DISH_DESC_P: Selector = sel("h3 + p");
    static ref SEL_DISH_DESC_D: Selector = sel("h3 + div");
}

#[derive(Clone, Debug)]
pub struct OldTownScraper {
    client: Client,
    site_id: Uuid,
    request_delay: Duration,
}

impl OldTownScraper {
    pub fn new(client: Client, site_id: Uuid, request_delay: Duration) -> Self {
        Self {
            client,
            site_id,
            request_delay,
        }
    }

    async fn get(&self, url: &str) -> Result<String> {
        trace!(?url, "Fetching URL...");
        scrape::get(&self.client, url).await
    }

    // this far, this seems to mostly work for both pita and tallrik pages...
    async fn parse_overview_page(&self, url: &str) -> Result<Vec<Dish>> {
        let html = Html::parse_document(&self.get(url).await?);

        let mut dishes = Vec::new();
        for dc in html.select(&SEL_DISH_CONTAINER) {
            if let Some(dish) = parse_dish(&dc) {
                dishes.push(dish);
            }
        }
        Ok(dishes)
    }
}

impl RestaurantScraper for OldTownScraper {
    fn name(&self) -> &'static str {
        "SE::GBG::Majorna::OldTown::Scraper"
    }

    async fn run(&self) -> Result<ScrapeResult> {
        let ot = Restaurant::new_for_site("Old Town", self.site_id);
        let mut dishes = Vec::new();
        let mut res = self
            .parse_overview_page(&format!("{}/{}", URL_PREFIX, EP_PITA))
            .await?;
        dishes.append(&mut res);
        let mut res = self
            .parse_overview_page(&format!("{}/{}", URL_PREFIX, EP_TALLRIK))
            .await?;
        dishes.append(&mut res);
        Ok(ScrapeResult {
            site_id: self.site_id,
            restaurants: vec![ot.with_dishes(dishes)],
        })
    }
}

fn parse_dish(e: &ElementRef) -> Option<Dish> {
    // this pulls out the wrong data for some dishes, since oldtown.se is not
    // consistent with their (already crappy) html.
    // It's just too bothersome to try to cater for all their weirdness,
    // so I'll just let it be. People can visit the site directly if they
    // find this annoying.
    let dish_name = e
        .select(&SEL_DISH_NAME)
        .next()
        .map(|dn| dn.text().next().map(|v| v.trim()).unwrap_or_default());

    let dish_desc = match e.select(&SEL_DISH_DESC_P).next() {
        None => match e.select(&SEL_DISH_DESC_D).next() {
            None => None,
            Some(dd) => dd.text().next().map(reduce_whitespace),
        },
        Some(dd) => dd.text().next().map(reduce_whitespace),
    };

    let dish_price = e
        .select(&SEL_DISH_PRICE)
        .next()
        .map(|dp| dp.text().next().map(|v| v.trim()).unwrap_or_default());

    if let Some((dn, dp)) = dish_name.zip(dish_price) {
        return Some(Dish {
            dish_id: Uuid::new_v4(),
            name: dn.into(),
            description: dish_desc,
            price: parse_float(dp),
            ..Default::default()
        });
    }

    None
}
