// This one pulls data from https://github.com/Fawenah/lindholmen_lunch/blob/main/data/lunch_data_<day-of-week>.json
// Permission is granted to use this data.

use crate::{
    cache::Client,
    models::{
        Dish, Restaurant, Site,
        fawenah::{DayMenus, RestaurantLink},
    },
    scrape::{SiteScrapeResult, SiteScraper},
    util,
};
use anyhow::Result;
use chrono::Weekday;
use std::{collections::hash_map::HashMap, time::Instant};
use tracing::{debug, trace};
use uuid::Uuid;

static RESTAURANT_LINKS_URL: &str = "https://raw.githubusercontent.com/Fawenah/lindholmen_lunch/refs/heads/main/data/restaurant_links.json";

pub struct LHScraper {
    client: Client,
    site_id: Uuid,
}

impl LHScraper {
    pub fn new(client: Client, site_id: Uuid) -> Self {
        Self { client, site_id }
    }

    async fn parse_site(&self) -> Result<Site> {
        let start = Instant::now();

        let mut site = self.construct_initial_site(
            self.client
                .get(RESTAURANT_LINKS_URL)
                .send()
                .await?
                .json::<HashMap<String, RestaurantLink>>()
                .await?,
        );

        trace!("Constructed initial site in {:?}", start.elapsed());

        let mut menus = self
            .client
            .get(get_json_url_for_day(util::get_weekday()))
            .send()
            .await?
            .json::<DayMenus>()
            .await?
            .strip_key_suffix("Scraper");

        // Convert the map of restaurants temporarily, so we can do faster lookups
        // and not iterate through the whole map on each outer iteration
        let mut rs: HashMap<String, Restaurant> = site.restaurants.clone().into();
        for (k, v) in menus.drain() {
            if let Some(r) = rs.get_mut(&k) {
                for item in v.items.into_iter() {
                    let mut dish: Dish = item.into();
                    dish.restaurant_id = r.restaurant_id;
                    r.add(dish);
                }
            } else {
                debug!("No match for {}", &k);
            }
        }
        // set updated restaurants
        site.restaurants = rs.into();

        // In the source JSON, there are some restaurants that have no scrapers / results, so we
        // get items with only links to site and maps, but no dishes.
        // We could loop through here and remove those, but for now I'll leave those in.

        trace!("Scrape done in {:?}", start.elapsed());

        Ok(site)
    }

    fn construct_initial_site(&self, mut items: HashMap<String, RestaurantLink>) -> Site {
        // we only need the site_id and the restaurant map, since this site is not
        // to be inserted in the DB in full. We just need a container for the restaurants and
        // dishes.
        let mut site = Site {
            site_id: self.site_id,
            ..Default::default()
        };
        for (k, v) in items.drain() {
            let mut r = Restaurant::new_for_site(&k, site.site_id);
            r.url = Some(v.url);
            r.map_url = Some(v.map);
            site.add(r);
        }
        site
    }
}

impl SiteScraper for LHScraper {
    async fn run(&self) -> Result<SiteScrapeResult> {
        let s = self.parse_site().await?;
        Ok(SiteScrapeResult {
            site_id: s.site_id,
            restaurants: s.restaurants.into_vec(),
        })
    }

    fn name(&self) -> &'static str {
        "se::gbg::lh::Fawenah::LHScraper"
    }
}

fn get_json_url_for_day(wd: Weekday) -> String {
    // There are only files for mon-fri, but we'll do the error handling elsewhere if the url is
    // not valid
    format!(
        "https://github.com/Fawenah/lindholmen_lunch/raw/refs/heads/main/data/lunch_data_{}.json",
        util::get_weekday_name(wd)
    )
}
