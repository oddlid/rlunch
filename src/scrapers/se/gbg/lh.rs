use crate::{
    data::{Dish, Restaurant},
    scrape::{RestaurantScraper, ScrapeResult},
    util::*,
};
use anyhow::{anyhow, bail, Result};
use compact_str::{CompactString, ToCompactString};
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};
use std::collections::hash_map::HashMap;

const SCRAPE_URL: &str = "http://localhost:8080";
const ATTR_CLASS: &str = "class";
const ATTR_TITLE: &str = "title";

// For constructing ScrapeResult. Values subject to change.
const COUNTRY_ID: &str = "se";
const CITY_ID: &str = "gbg";
const SITE_ID: &str = "lh";

// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

lazy_static! {
    static ref SEL_VIEW_CONTENT: Selector = sel("div.view-content");
    static ref SEL_DISH: Selector = sel("span.dish-name");
    static ref SEL_DISH_TYPE: Selector = sel("div.icon-dish");
    static ref SEL_DISH_PRICE: Selector = sel("div.table-list__column--price");
}

#[derive(Default, Clone, Debug)]
pub struct LHScraper {
    client: Client,
    url: &'static str,
}

impl LHScraper {
    pub fn new() -> Self {
        Self {
            url: SCRAPE_URL,
            client: Client::builder()
                .user_agent(APP_USER_AGENT)
                .build()
                .unwrap(),
        }
    }

    async fn get(&self) -> Result<String> {
        self.client
            .get(self.url)
            .send()
            .await?
            .text()
            .await
            .map_err(anyhow::Error::from)
    }
}

impl RestaurantScraper for LHScraper {
    fn name(&self) -> &'static str {
        "SE::GBG::LH::Scraper"
    }

    async fn run(&self) -> Result<ScrapeResult> {
        let html = Html::parse_document(&self.get().await?);
        let vc = match html.select(&SEL_VIEW_CONTENT).next() {
            Some(vc) => vc,
            None => bail!("Invalid HTML"),
        };

        let mut restaurants = HashMap::new();
        let mut cur_restaurant_name = CompactString::new("");

        for e in vc.child_elements() {
            match e.attr(ATTR_CLASS) {
                None => continue,
                Some(v) => {
                    if v == ATTR_TITLE {
                        if let Some(name) = e.text().next().map(|v| v.trim().to_compact_string()) {
                            cur_restaurant_name = name;
                        }
                    } else if let Some(d) = parse_dish(&e) {
                        if cur_restaurant_name.is_empty() {
                            continue;
                        }
                        let entry = restaurants
                            .entry(cur_restaurant_name.clone())
                            .or_insert_with(|| Restaurant::new(&cur_restaurant_name));
                        entry.dishes.push(d);
                    }
                }
            }
        }

        // TODO: Fetch details about each restaurant, as in the original

        Ok(ScrapeResult {
            country_id: COUNTRY_ID.to_compact_string(),
            city_id: CITY_ID.to_compact_string(),
            site_id: SITE_ID.to_compact_string(),
            restaurants: restaurants.into_values().collect(),
        })
    }
}

fn parse_dish(e: &ElementRef) -> Option<Dish> {
    let (name, description) = get_dish_name_and_desc(e);
    let price = match get_text(e, &SEL_DISH_PRICE) {
        None => 0.0,
        Some(v) => parse_float(v.trim()),
    };
    let mut dish = Dish {
        name: name?,
        description,
        price,
        ..Default::default()
    };
    if let Some(t) = get_text(e, &SEL_DISH_TYPE) {
        dish.tags.insert(t);
    }
    Some(dish)
}

fn get_dish_name_and_desc(e: &ElementRef) -> (Option<CompactString>, Option<CompactString>) {
    match e.select(&SEL_DISH).next() {
        None => (None, None),
        Some(v) => {
            let mut t = v.text();
            let name = t.next().map(|v| v.trim().to_compact_string());
            let desc = t.next().map(reduce_whitespace);
            (name, desc)
        }
    }
}
