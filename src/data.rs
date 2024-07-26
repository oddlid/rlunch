use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::HashMap, hash_set::HashSet},
    fmt::Display,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Dish {
    /// Name of the dish, e.g. "meatballs"
    #[serde(default)]
    pub name: String,
    /// More details about the dish, e.g. "with spaghetti"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Extra info, e.g. "contains nuts"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optionals tags for filtering, e.g. "vego,gluten,lactose"
    #[serde(default)]
    pub tags: HashSet<String>,
    /// Price, in whatever currency is in use
    #[serde(default)]
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
            name: name.to_owned(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Restaurant {
    /// Name of restaurant
    #[serde(default)]
    pub name: String,
    /// Extra info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Street address
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Homepage
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Google maps URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_url: Option<String>,
    /// When the scraping was last done
    #[serde(default)]
    pub parsed_at: DateTime<Local>,
    /// List of current dishes
    #[serde(default)]
    pub dishes: Vec<Dish>,
}

impl Restaurant {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            parsed_at: Local::now(),
            ..Default::default()
        }
    }

    // unsure if I want to have methods like this
    // pub fn add_dish(&mut self, dish: Dish) {
    //     self.dishes.push(dish);
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Site {
    /// Name of site/area
    #[serde(default)]
    pub name: String,
    /// Extra info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// List of current restaurants
    #[serde(default)]
    pub restaurants: HashMap<String, Restaurant>,
}

impl Site {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    // unsure if I want to have methods like this
    // pub fn add_restaurant(&mut self, restaurant: Restaurant) {
    //     self.restaurants.insert(restaurant.name.clone(), restaurant);
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct City {
    /// Name of city
    #[serde(default)]
    pub name: String,
    /// List of current sites
    #[serde(default)]
    pub sites: HashMap<String, Site>,
}

impl City {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    // unsure if I want to have methods like this
    // pub fn add_site(&mut self, site: Site) {
    //     self.sites.insert(site.name.clone(), site);
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Country {
    /// Name of country
    #[serde(default)]
    pub name: String,
    /// Currency abbreviation to use as suffix for dish prices
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_suffix: Option<String>,
    /// List of current cities
    #[serde(default)]
    pub cities: HashMap<String, City>,
}

impl Country {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Default::default()
        }
    }

    // unsure if I want to have methods like this
    // pub fn add_city(&mut self, city: City) {
    //     self.cities.insert(city.name.clone(), city);
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct LunchData {
    /// List of current countries
    #[serde(default)]
    pub countries: HashMap<String, Country>,
}

impl LunchData {
    pub fn new() -> Self {
        Default::default()
    }

    // unsure if I want to have methods like this
    // pub fn add_country(&mut self, country: Country) {
    //     self.countries.insert(country.name.clone(), country);
    // }
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
        d.tags.insert(String::from("carnivora"));
        d.tags.insert(String::from("yummy"));

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
