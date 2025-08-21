/// This scraper gets all info for the whole site from a single website,
/// so we do everything in a single module.
/// Other scrapers might need to have a submodule for each restaurant,
/// to not make the code too unwieldy.
///
/// This module is now deprecated, since lindholmen.se is no longer providing a lunch menu,
/// but just links to restaurants, as of 2025-06-20.
/// Just keeping it around a little whille for reference.
///
use crate::{
    cache::Client,
    models::{Dish, Restaurant},
    scrape::{SiteScrapeResult, SiteScraper},
    util::*,
};
use anyhow::{Result, anyhow, bail};
use lazy_static::lazy_static;
use scraper::{ElementRef, Html, Selector, selectable::Selectable};
use slugify::slugify;
use std::collections::hash_map::HashMap;
use tracing::{error, trace};
use url::Url;
use uuid::Uuid;

// static SCRAPE_URL: &str = "http://localhost:8080";
static URL_PREFIX: &str = "https://www.lindholmen.se/sv/";
static SCRAPE_URL: &str = "https://lindholmen.uit.se/omradet/dagens-lunch?embed-mode=iframe";
static ATTR_CLASS: &str = "class";
static ATTR_TITLE: &str = "title";
static ATTR_HREF: &str = "href";
static MAPS_DOMAIN: &str = "maps.google.com";
static ERR_INVALID_HTML: &str = "Invalid HTML";

lazy_static! {
    static ref SEL_CONTENT: Selector = sel("div.content");
    static ref SEL_VIEW_CONTENT: Selector = sel("div.view-content");
    static ref SEL_DISH: Selector = sel("span.dish-name");
    static ref SEL_DISH_TYPE: Selector = sel("div.icon-dish");
    static ref SEL_DISH_PRICE: Selector = sel("div.table-list__column--price");
    static ref SEL_LINK: Selector = sel("p > a");
    static ref SEL_ADDR: Selector = sel("div > h3 + p");
}

#[derive(Clone)]
pub struct LHScraper {
    client: Client,
    url: &'static str,
    site_id: Uuid,
}

#[derive(Default, Clone, Debug)]
struct AddrInfo {
    /// Street addres
    address: Option<String>,
    /// Google maps url
    map_url: Option<String>,
}

impl LHScraper {
    pub fn new(client: Client, site_id: Uuid) -> Self {
        Self {
            url: SCRAPE_URL, // TODO: evaluate if this should rather be passed in
            client,
            site_id,
        }
    }

    async fn get(&self, url: &str) -> Result<String> {
        self.client.get_as_string(url).await
    }

    async fn get_addr_info(&self, url: &str) -> Result<AddrInfo> {
        trace!(url = %url, "Fetching address info...");
        let html = Html::parse_document(&self.get(url).await?);

        let content = match html.select(&SEL_CONTENT).next() {
            Some(c) => c,
            None => bail!(ERR_INVALID_HTML),
        };

        // first search for map links, since they'll contain all we need
        trace!("Trying to find map link with address....");
        for anchor in content.select(&SEL_LINK) {
            if let Some(href) = anchor.attr(ATTR_HREF) {
                if href.contains(MAPS_DOMAIN) {
                    let map_url = Url::parse(href)?;
                    if let Some(q) = map_url.query_pairs().into_owned().next() {
                        let addr = urlencoding::decode(&q.1)?.into_owned();
                        return Ok(AddrInfo {
                            address: Some(addr.trim().into()),
                            map_url: Some(map_url.as_str().into()),
                        });
                    }
                }
            }
        }

        // try to just find an address, if no links were found, as in the case of Pier 11
        trace!("No map link found, trying to find just address...");
        if let Some(p) = content.select(&SEL_ADDR).next() {
            if let Some(addr) = p.text().next().map(|v| v.trim().into()) {
                return Ok(AddrInfo {
                    address: Some(addr),
                    map_url: None,
                });
            }
        }

        Err(anyhow!("No address found"))
    }

    async fn update_restaurant_addresses(
        &self,
        mut restaurants: HashMap<String, Restaurant>,
    ) -> HashMap<String, Restaurant> {
        for (k, v) in restaurants.iter_mut() {
            // Throttle requests to not get blocked
            tokio::time::sleep(self.client.request_delay()).await;

            let info = self.get_addr_info(k).await;
            if info.is_err() {
                let e = info.unwrap_err();
                error!(err = %e, url = k, "Failed to get address info");
                continue;
            }
            let info = info.unwrap();
            v.address = info.address;
            v.map_url = info.map_url;
        }
        restaurants
    }
}

impl SiteScraper for LHScraper {
    fn name(&self) -> &'static str {
        "SE::GBG::LH::Scraper"
    }

    async fn run(&self) -> Result<SiteScrapeResult> {
        let mut restaurants = HashMap::new();

        // Due to some rust bug/weirdness, we need to wrap this in a scope, otherwise the compiler
        // will complain about the selection being non-Send, held across an await point
        {
            let html = Html::parse_document(&self.get(self.url).await?);
            let vc = match html.select(&SEL_VIEW_CONTENT).next() {
                Some(vc) => vc,
                None => bail!(ERR_INVALID_HTML),
            };

            let mut cur_restaurant_name = String::new();

            for e in vc.child_elements() {
                match e.attr(ATTR_CLASS) {
                    None => continue,
                    Some(v) => {
                        if v == ATTR_TITLE {
                            if let Some(name) = e.text().next().map(|v| v.trim().into()) {
                                cur_restaurant_name = name;
                            }
                        } else if let Some(d) = parse_dish(&e) {
                            if cur_restaurant_name.is_empty() {
                                continue;
                            }
                            let restaurant = restaurants
                                .entry(get_restaurant_link(&cur_restaurant_name))
                                .or_insert_with(|| {
                                    Restaurant::new_for_site(&cur_restaurant_name, self.site_id)
                                });
                            restaurant
                                .dishes
                                .insert(d.dish_id, d.for_restaurant(restaurant.restaurant_id));
                        }
                    }
                }
            }
        }

        let restaurants = self
            .update_restaurant_addresses(update_restaurant_links(restaurants))
            .await;

        Ok(SiteScrapeResult {
            site_id: self.site_id,
            restaurants: restaurants.into_values().collect(),
        })
    }
}

/// Set the url field of each restaurant to the key under which it's stored in the given map
fn update_restaurant_links(mut r: HashMap<String, Restaurant>) -> HashMap<String, Restaurant> {
    r.iter_mut().for_each(|(k, v)| v.url = Some(k.clone()));
    r
}

fn parse_dish(e: &ElementRef) -> Option<Dish> {
    let (name, description) = get_dish_name_and_desc(e);
    let price = match get_text(e, &SEL_DISH_PRICE) {
        None => 0.0,
        Some(v) => parse_float(v.trim()),
    };
    let mut dish = Dish {
        dish_id: Uuid::new_v4(), // very important when creating a Dish manually!
        name: name?,
        description,
        price,
        ..Default::default()
    };
    if let Some(t) = get_text(e, &SEL_DISH_TYPE) {
        dish.tags.push(t);
    }
    Some(dish)
}

fn get_dish_name_and_desc(e: &ElementRef) -> (Option<String>, Option<String>) {
    match e.select(&SEL_DISH).next() {
        None => (None, None),
        Some(v) => {
            let mut t = v.text();
            let name = t.next().map(|v| v.trim().into());
            let desc = t.next().map(reduce_whitespace);
            (name, desc)
        }
    }
}

fn get_restaurant_link(name: &str) -> String {
    // Local dev version
    // format!(
    //     "{}/{}",
    //     SCRAPE_URL,
    //     slugify!(&str::replace(name, "'", ""), stop_words = "by,of")
    // )

    // slugify will replace apostrophes with dashes, so we need to strip them out first in order to
    // get the same slugs as lindholmen.se uses.
    // They also seem to remove certain words, like "by" and "of", so we strip those as well.
    format!(
        "{}{}",
        URL_PREFIX,
        slugify!(&str::replace(name, "'", ""), stop_words = "by,of")
    )
}
