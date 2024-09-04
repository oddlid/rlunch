use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::HashMap;
use std::fmt::Display;

// I'm evaluating if I should move away from having all these nested structs, and rather have them
// decoupled, or at least from `Site` and upwards.
// My original idea was to implement this in the same way as the original `go2lunch`, where there's
// a set of threads responsible for scraping and updating the global data structure, while other threads
// take care of the http serving. That worked well enough in the original, with a quite small data
// set, but I think this would become problematic if we're to actually expand into many more sites,
// cities, and countries.
//
// A better approcah could be to have all data in some DB, and to have wrapper structs that
// contains the necessary references / IDs, to be able to resolve where in the hierarchy the item
// belongs.
// I should then separate the logic of serving, from scraping and updating.
// There are several ways this could be done:
// - We could have an http server that only serves GET requests
//   * Scraping is done in a separate binary, directly updating the same DB, either with each
//     scraper directly updating the DB, or via a central manager thread receiving scrape results
//     and updating the DB from the results
// - We could have an http server that accepts GET/POST/DELETE etc to update the DB from external
//   scrape results. This would require authentication.
//   * Separate scraper binary/(ies) that post results to the http server. Most flexible, since
//     scrapers could be run anywhere and written in any language.
// - A combination/variant of the above, where serving GET is separate, and we have some sort of
//   manager responsible for receiving results and updating the DB. Could be with authentication,
//   or without, if all scrapers are local and trusted.
//
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Dish {
    /// Name of the dish, e.g. "meatballs"
    #[sqlx(rename = "dish_name")]
    pub name: String,
    /// More details about the dish, e.g. "with spaghetti"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Extra info, e.g. "contains nuts"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optionals tags for filtering, e.g. "vego,gluten,lactose"
    pub tags: Vec<String>,
    /// Price, in whatever currency is in use
    pub price: f32,
}

impl Display for Dish {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.description {
            Some(d) => write!(f, "{} {}", self.name, d),
            None => write!(f, "{}", self.name),
        }
    }
}

impl Dish {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Restaurant {
    /// Name of restaurant
    #[sqlx(rename = "restaurant_name")]
    pub name: String,
    /// Extra info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Street address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Homepage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Google maps URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_url: Option<String>,
    /// When the scraping was last done
    #[sqlx(rename = "created_at")]
    pub parsed_at: DateTime<Local>,
    /// List of current dishes
    pub dishes: Vec<Dish>,
}

impl Restaurant {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            parsed_at: Local::now(),
            ..Default::default()
        }
    }

    // pub fn opt_comment(&self) -> Option<&str> {
    //     // match self.comment.as_ref() {
    //     //     Some(v) => Some(v),
    //     //     None => None,
    //     // }
    //
    //     self.comment.as_ref().map(|v| v.as_str())
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Site {
    /// Name of site/area
    pub name: String,
    /// Extra info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// List of current restaurants
    pub restaurants: HashMap<String, Restaurant>,
}

impl Site {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct City {
    /// Name of city
    pub name: String,
    /// List of current sites
    pub sites: HashMap<String, Site>,
}

impl City {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Country {
    /// Name of country
    pub name: String,
    /// Currency abbreviation to use as suffix for dish prices
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_suffix: Option<String>,
    /// List of current cities
    pub cities: HashMap<String, City>,
}

impl Country {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct LunchData {
    /// List of current countries
    pub countries: HashMap<String, Country>,
}

impl LunchData {
    pub fn new() -> Self {
        Default::default()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn dish_display() {
        let d = Dish {
            name: String::from("meat"),
            description: Some(String::from("balls")),
            ..Default::default()
        };
        assert_eq!("meat balls", format!("{d}"));
    }

    #[test]
    fn dish_deserialize() {
        let j = serde_json::json!({
            "name": "Test",
            "description": "with sauce",
            "price": 32
        });
        let d: Dish = serde_json::from_value(j).unwrap();
        assert_eq!(32.0, d.price);
        println!("{d:#?}");
    }

    #[test]
    #[ignore = "Visual inspection"]
    fn show_structure() {
        let mut d = Dish::new("meat");
        d.description = Some(String::from("balls"));
        d.price = 120.0;
        d.tags.push(String::from("carnivora"));
        d.tags.push(String::from("yummy"));

        let mut r = Restaurant::new("Pasta House");
        r.dishes.push(d);

        let mut s = Site::new("SomeSite");
        s.restaurants.insert(r.name.clone(), r);

        let mut city = City::new("Göteborg");
        city.sites.insert(s.name.clone(), s);

        let mut country = Country::new("Sweden");
        country.cities.insert(city.name.clone(), city);

        let mut ld = LunchData::new();
        ld.countries.insert(country.name.clone(), country);

        println!("{}", serde_json::to_string_pretty(&ld).unwrap());
    }
}
