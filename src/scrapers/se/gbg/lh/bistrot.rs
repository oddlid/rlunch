// Scraper for bistrot.se

use anyhow::Result;
use chrono::Local;
use uuid::Uuid;

use crate::{
    cache::Client,
    models::{Dish, Restaurant},
    scrape::RestaurantScraper,
};

// The menu on the site is very understable for a human to read,
// but parsing it might be a problem, due to how it's presented...
// The html tag structure is not fully predictable, so one might end up extracting from
// the empty tags used for spacing between the actual content.
// I think I'll start with something easier to parse than this site.
//
//
// Pris dagens lunch:
// #shapely_home_parallax-5 > section > div > div > div > div > div > div > p:nth-child(3)
// Veckans - titel:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(2) > strong
// Veckans - desc:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(5)
// Veg, mån-tis, titel:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(8) > strong
// Veg, mån-tis, desc:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(11)
// Veg, ons-tors, titel:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(14) > strong
// Veg, ons-tors, desc:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(17)
// Veg, fre, titel:
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(20) > strong
// Veg, fre, titel (sallad):
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(23) > strong
// Veg, fre, desc (sallad):
// #fdm-menu-1 > li > ul > li:nth-child(2) > div > div.fdm-item-content > p:nth-child(26)

static SCRAPE_URL: &str = "https://bistrot.se";

#[derive(Clone)]
pub struct Bistrot {
    client: Client,
    site_id: Uuid,
}

impl Bistrot {
    pub fn new(client: Client, site_id: Uuid) -> Self {
        Self { client, site_id }
    }

    async fn get(&self, url: &str) -> Result<String> {
        self.client.get_as_string(url).await
    }
}

impl RestaurantScraper for Bistrot {
    async fn run(&self) -> Result<Restaurant> {
        // Set stuff that rarely changes statically
        let mut r = Restaurant {
            restaurant_id: Uuid::new_v4(),
            site_id: self.site_id,
            name: "Bistrot".into(),
            address: Some("Diagonalen 8, 41756 Göteborg".into()),
            url: Some(SCRAPE_URL.into()),
            map_url: Some("https://www.google.com/maps/search/?api=1&query=BISTROT+Diagonalen+8++417+56+G%C3%B6teborg".into()),
            comment: Some("I alla rätter ingår salladsbuffé, hembakt surdegsbröd, kaffe och kaka.".into()),
            parsed_at: Local::now(),
            ..Default::default()
        };

        // price is the same for all dishes, so we use a separate variable, since it's not
        // specified along the individual dish on the page.
        // Static for now, while testing
        let price: f32 = 130.0;

        // add some dishes manually for test
        r.add(Dish {
            dish_id: Uuid::new_v4(),
            restaurant_id: r.restaurant_id,
            name: "Matjessill".into(),
            description: Some(
                "Kokt Färskpotatis – Brynt smör – Citroncreme – Ägg – Gräslök & Rödlök".into(),
            ),
            comment: Some("Veckans".into()),
            price,
            ..Default::default()
        });

        r.add(Dish {
            dish_id: Uuid::new_v4(),
            restaurant_id: r.restaurant_id,
            name: "Panerad Feta ost sallad".into(),
            description: Some("Tomat – Oliver – Tzatziki – Rostade kikärtor".into()),
            comment: Some("Mån-Tis".into()),
            price,
            tags: vec![String::from("Vegetarisk")],
        });

        Ok(r)
    }
}
