// The structs in this module are a direct mapping of the DB structure,
// while the structs in the api sub-module are stripped versions of those intended for use in API
// output, and similar, where uuids and mappings are not needed.

use crate::util;
use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::HashMap,
    convert::From,
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

pub trait Id {
    fn id(&self) -> Uuid;
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
pub struct UuidMap<T>(pub HashMap<Uuid, T>);

impl<T: Id, U: From<T>> From<Vec<T>> for UuidMap<U> {
    fn from(value: Vec<T>) -> Self {
        Self(value.into_iter().map(|v| (v.id(), v.into())).collect())
    }
}

impl From<UuidMap<Restaurant>> for HashMap<String, Restaurant> {
    fn from(mut value: UuidMap<Restaurant>) -> Self {
        value.drain().map(|(_, v)| (v.name.clone(), v)).collect()
    }
}

impl From<HashMap<String, Restaurant>> for UuidMap<Restaurant> {
    fn from(mut value: HashMap<String, Restaurant>) -> Self {
        Self(value.drain().map(|(_, v)| (v.id(), v)).collect())
    }
}

impl<T> Deref for UuidMap<T> {
    type Target = HashMap<Uuid, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UuidMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Id> UuidMap<T> {
    pub fn into_vec<U: From<T>>(mut self) -> Vec<U> {
        self.drain().map(|(_, v)| v.into()).collect()
    }

    pub fn add(&mut self, v: T) -> Option<T> {
        self.insert(v.id(), v)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Dish {
    #[serde(skip_serializing)]
    pub dish_id: Uuid,
    #[serde(skip_serializing)]
    pub restaurant_id: Uuid, // parent restaurant
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Price, in whatever currency is in use
    pub price: f32,
}

impl Dish {
    pub fn new(name: &str) -> Self {
        Self {
            dish_id: Uuid::new_v4(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn for_restaurant(self, restaurant_id: Uuid) -> Self {
        Self {
            restaurant_id,
            ..self
        }
    }
}

impl Id for Dish {
    fn id(&self) -> Uuid {
        self.dish_id
    }
}

impl From<fawenah::MenuItem> for Dish {
    fn from(item: fawenah::MenuItem) -> Self {
        let mut s = Self {
            dish_id: Uuid::new_v4(),
            name: item.name,
            tags: item.tags,
            ..Default::default()
        };
        if !item.category.is_empty() {
            s.comment = Some(item.category);
        }
        if !item.description.is_empty() {
            s.description = Some(item.description);
        }
        if !item.price.is_empty() {
            s.price = util::parse_float(&item.price);
        }
        s
    }
}

// impl From<api::Dish> for Dish {
//     fn from(dish: api::Dish) -> Self {
//         Self {
//             name: dish.name,
//             description: dish.description,
//             comment: dish.comment,
//             tags: dish.tags,
//             price: dish.price,
//             ..Default::default()
//         }
//     }
// }

/// DishRows maps a list of Dish into lists of all its fields.
/// The intended use is together with Postgres' UNNEST, to be able to do batch insert of many
/// Dishes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DishRows {
    pub dish_ids: Vec<Uuid>,
    pub restaurant_ids: Vec<Uuid>,
    pub names: Vec<String>,
    pub descriptions: Vec<Option<String>>,
    pub comments: Vec<Option<String>>,
    pub tags: Vec<String>, // comma separated list
    pub prices: Vec<f32>,
}

impl DishRows {
    fn with_capacity(cap: usize) -> Self {
        Self {
            dish_ids: Vec::with_capacity(cap),
            restaurant_ids: Vec::with_capacity(cap),
            names: Vec::with_capacity(cap),
            descriptions: Vec::with_capacity(cap),
            comments: Vec::with_capacity(cap),
            tags: Vec::with_capacity(cap),
            prices: Vec::with_capacity(cap),
        }
    }

    fn extend(&mut self, other: DishRows) {
        self.dish_ids.extend(other.dish_ids);
        self.restaurant_ids.extend(other.restaurant_ids);
        self.names.extend(other.names);
        self.descriptions.extend(other.descriptions);
        self.comments.extend(other.comments);
        self.tags.extend(other.tags);
        self.prices.extend(other.prices);
    }
}

impl From<UuidMap<Dish>> for DishRows {
    fn from(mut m: UuidMap<Dish>) -> Self {
        let mut dr = Self::with_capacity(m.len());

        for (_, v) in m.drain() {
            dr.dish_ids.push(v.dish_id);
            dr.restaurant_ids.push(v.restaurant_id);
            dr.names.push(v.name);
            dr.descriptions.push(v.description);
            dr.comments.push(v.comment);
            dr.tags.push(v.tags.join(",")); // flatten the list to comma separated values
            dr.prices.push(v.price);
        }

        dr
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Restaurant {
    #[serde(skip_serializing)]
    pub restaurant_id: Uuid,
    #[serde(skip_serializing)]
    pub site_id: Uuid, // parent site
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
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub dishes: UuidMap<Dish>,
}

impl Restaurant {
    pub fn new(name: &str) -> Self {
        Self {
            restaurant_id: Uuid::new_v4(),
            name: name.into(),
            parsed_at: Local::now(),
            ..Default::default()
        }
    }

    pub fn new_for_site(name: &str, site_id: Uuid) -> Self {
        Self {
            site_id,
            ..Self::new(name)
        }
    }

    pub fn add(&mut self, dish: Dish) -> Option<Dish> {
        self.dishes.add(dish)
    }

    pub fn set_dishes(&mut self, dishes: Vec<Dish>) {
        self.dishes = dishes.into()
    }

    pub fn with_id(self, restaurant_id: Uuid) -> Self {
        Self {
            restaurant_id,
            ..self
        }
    }

    pub fn with_dish(mut self, dish: Dish) -> Self {
        self.add(dish);
        self
    }

    pub fn with_dishes(mut self, dishes: Vec<Dish>) -> Self {
        self.set_dishes(dishes);
        self
    }
}

impl Id for Restaurant {
    fn id(&self) -> Uuid {
        self.restaurant_id
    }
}

// impl From<api::Restaurant> for Restaurant {
//     fn from(restaurant: api::Restaurant) -> Self {
//         Self {
//             name: restaurant.name,
//             comment: restaurant.comment,
//             address: restaurant.address,
//             url: restaurant.url,
//             map_url: restaurant.map_url,
//             parsed_at: restaurant.parsed_at,
//             dishes: restaurant.dishes.into(),
//             ..Default::default()
//         }
//     }
// }

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RestaurantRows {
    pub restaurant_ids: Vec<Uuid>,
    pub site_ids: Vec<Uuid>,
    pub names: Vec<String>,
    pub comments: Vec<Option<String>>,
    pub addresses: Vec<Option<String>>,
    pub urls: Vec<Option<String>>,
    pub map_urls: Vec<Option<String>>,
    pub parsed_ats: Vec<DateTime<Local>>,
    pub dishes: DishRows,
}

impl RestaurantRows {
    fn with_capacity(cap: usize) -> Self {
        Self {
            restaurant_ids: Vec::with_capacity(cap),
            site_ids: Vec::with_capacity(cap),
            names: Vec::with_capacity(cap),
            comments: Vec::with_capacity(cap),
            addresses: Vec::with_capacity(cap),
            urls: Vec::with_capacity(cap),
            map_urls: Vec::with_capacity(cap),
            parsed_ats: Vec::with_capacity(cap),
            dishes: DishRows::with_capacity(cap), // might be good to use a larger size here
        }
    }
}

impl From<Vec<Restaurant>> for RestaurantRows {
    fn from(v: Vec<Restaurant>) -> Self {
        let mut rr = Self::with_capacity(v.len());

        for r in v {
            rr.restaurant_ids.push(r.restaurant_id);
            rr.site_ids.push(r.site_id);
            rr.names.push(r.name);
            rr.comments.push(r.comment);
            rr.addresses.push(r.address);
            rr.urls.push(r.url);
            rr.map_urls.push(r.map_url);
            rr.parsed_ats.push(r.parsed_at);
            rr.dishes.extend(r.dishes.into());
        }

        rr
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Site {
    #[serde(skip_serializing)]
    pub site_id: Uuid,
    #[serde(skip_serializing)]
    pub city_id: Uuid, // parent city
    pub name: String,
    pub url_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub restaurants: UuidMap<Restaurant>,
}

impl Site {
    pub fn new(name: &str) -> Self {
        Self {
            site_id: Uuid::new_v4(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn new_for_city(name: &str, city_id: Uuid) -> Self {
        Self {
            city_id,
            ..Self::new(name)
        }
    }

    pub fn add(&mut self, restaurant: Restaurant) -> Option<Restaurant> {
        self.restaurants.add(restaurant)
    }

    pub fn set_restaurants(&mut self, restaurants: Vec<Restaurant>) {
        self.restaurants = restaurants.into()
    }

    /// Add dishes to any restaurant in this site
    pub fn add_dishes(&mut self, dishes: Vec<Dish>) {
        for d in dishes {
            if let Some(r) = self.restaurants.get_mut(&d.restaurant_id) {
                r.add(d);
            }
        }
    }

    pub fn with_restaurant(mut self, restaurant: Restaurant) -> Self {
        self.add(restaurant);
        self
    }

    pub fn with_restaurants(mut self, restaurants: Vec<Restaurant>) -> Self {
        self.set_restaurants(restaurants);
        self
    }

    pub fn with_dishes(mut self, dishes: Vec<Dish>) -> Self {
        self.add_dishes(dishes);
        self
    }
}

impl Id for Site {
    fn id(&self) -> Uuid {
        self.site_id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct City {
    #[serde(skip_serializing)]
    pub city_id: Uuid,
    #[serde(skip_serializing)]
    pub country_id: Uuid, // parent country
    pub name: String,
    pub url_id: String,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub sites: UuidMap<Site>,
}

impl City {
    pub fn new(name: &str) -> Self {
        Self {
            city_id: Uuid::new_v4(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn new_for_country(name: &str, country_id: Uuid) -> Self {
        Self {
            country_id,
            ..Self::new(name)
        }
    }

    pub fn add(&mut self, site: Site) -> Option<Site> {
        self.sites.add(site)
    }

    pub fn set_sites(&mut self, sites: Vec<Site>) {
        self.sites = sites.into()
    }

    pub fn with_site(mut self, site: Site) -> Self {
        self.add(site);
        self
    }

    pub fn with_sites(mut self, sites: Vec<Site>) -> Self {
        self.set_sites(sites);
        self
    }
}

impl Id for City {
    fn id(&self) -> Uuid {
        self.city_id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct Country {
    #[serde(skip_serializing)]
    pub country_id: Uuid,
    pub name: String,
    pub url_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_suffix: Option<String>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub cities: UuidMap<City>,
}

impl Country {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn add(&mut self, city: City) -> Option<City> {
        self.cities.add(city)
    }

    pub fn set_cities(&mut self, cities: Vec<City>) {
        self.cities = cities.into()
    }

    pub fn with_city(mut self, city: City) -> Self {
        self.add(city);
        self
    }

    pub fn with_cities(mut self, cities: Vec<City>) -> Self {
        self.set_cities(cities);
        self
    }
}

impl Id for Country {
    fn id(&self) -> Uuid {
        self.country_id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
#[serde(default)]
#[sqlx(default)]
pub struct LunchData {
    /// List of current countries
    #[sqlx(skip)]
    pub countries: UuidMap<Country>,
}

impl LunchData {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, country: Country) -> Option<Country> {
        self.countries.add(country)
    }

    pub fn set_countries(&mut self, countries: Vec<Country>) {
        self.countries = countries.into()
    }

    pub fn with_country(mut self, country: Country) -> Self {
        self.add(country);
        self
    }

    pub fn with_countries(mut self, countries: Vec<Country>) -> Self {
        self.set_countries(countries);
        self
    }

    pub fn build(
        countries: Vec<Country>,
        cities: Vec<City>,
        sites: Vec<Site>,
        restaurants: Vec<Restaurant>,
        dishes: Vec<Dish>,
    ) -> Self {
        let mut restaurants: UuidMap<Restaurant> = restaurants.into();
        for dish in dishes {
            restaurants
                .entry(dish.restaurant_id)
                .and_modify(|restaurant| {
                    restaurant.add(dish);
                });
        }

        let mut sites: UuidMap<Site> = sites.into();
        for (_, restaurant) in restaurants.drain() {
            sites.entry(restaurant.site_id).and_modify(|site| {
                site.add(restaurant);
            });
        }

        let mut cities: UuidMap<City> = cities.into();
        for (_, site) in sites.drain() {
            cities.entry(site.city_id).and_modify(|city| {
                city.add(site);
            });
        }

        let mut countries: UuidMap<Country> = countries.into();
        for (_, city) in cities.drain() {
            countries.entry(city.country_id).and_modify(|country| {
                country.add(city);
            });
        }

        Self { countries }
    }

    pub fn get_site(&self, site_id: Uuid) -> Option<&Site> {
        for country in self.countries.values() {
            for city in country.cities.values() {
                if let Some(site) = city.sites.get(&site_id) {
                    return Some(site);
                }
            }
        }
        None
    }

    pub fn into_site(mut self, site_id: Uuid) -> Result<Site> {
        for (_, mut country) in self.countries.drain() {
            for (_, mut city) in country.cities.drain() {
                if let Some(site) = city.sites.remove(&site_id) {
                    return Ok(site);
                }
            }
        }
        Err(anyhow::format_err!("site_id {site_id} not found"))
    }
}

pub mod fawenah {
    use serde::{Deserialize, Serialize};
    use std::{
        collections::hash_map::HashMap,
        ops::{Deref, DerefMut},
    };

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct RestaurantLink {
        pub url: String,
        pub map: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct MenuItem {
        pub name: String,
        pub category: String,
        pub description: String,
        pub price: String,
        pub tags: Vec<String>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct DayMenu {
        pub day: String,
        pub items: Vec<MenuItem>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct DayMenus(pub HashMap<String, DayMenu>);

    impl Deref for DayMenus {
        type Target = HashMap<String, DayMenu>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for DayMenus {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl DayMenus {
        pub fn add(&mut self, k: String, v: DayMenu) -> Option<DayMenu> {
            self.insert(k, v)
        }

        pub fn strip_key_suffix(mut self, suffix: &str) -> Self {
            let mut s = Self {
                ..Default::default()
            };

            for (k, v) in self.drain() {
                if let Some(k) = k.strip_suffix(suffix) {
                    s.add(k.into(), v);
                } else {
                    s.add(k, v);
                }
            }
            s
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn uuidmap_from() {
//         let m: UuidMap<u32> = vec![1, 2, 3].into();
//         assert_eq!(3, m.len());
//
//         let v: Vec<u32> = m.into_vec();
//         assert_eq!([1u32, 2u32, 3u32], v[..]);
//     }
// }

pub mod api {
    // This module contains the same structs as the parent,
    // where maps have been converted to vecs, for easier use in templates
    // and possibly elsewhere
    use chrono::{DateTime, Local};
    use serde::{Deserialize, Serialize};
    use std::convert::From;
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Dish {
        pub dish_id: Uuid,
        pub restaurant_id: Uuid,
        /// Name of the dish, e.g. "meatballs"
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

    impl super::Id for Dish {
        fn id(&self) -> Uuid {
            self.dish_id
        }
    }

    impl From<super::Dish> for Dish {
        fn from(dish: super::Dish) -> Self {
            Self {
                dish_id: dish.dish_id,
                restaurant_id: dish.restaurant_id,
                name: dish.name,
                description: dish.description,
                comment: dish.comment,
                tags: dish.tags,
                price: dish.price,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Restaurant {
        pub restaurant_id: Uuid,
        pub site_id: Uuid,
        /// Name of restaurant
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
        pub parsed_at: DateTime<Local>,
        /// List of current dishes
        pub dishes: Vec<Dish>,
    }

    impl super::Id for Restaurant {
        fn id(&self) -> Uuid {
            self.restaurant_id
        }
    }

    impl From<super::Restaurant> for Restaurant {
        fn from(restaurant: super::Restaurant) -> Self {
            let mut dishes: Vec<Dish> = restaurant.dishes.into_vec();
            dishes.sort_by(|a, b| a.name.cmp(&b.name));
            Self {
                restaurant_id: restaurant.restaurant_id,
                site_id: restaurant.site_id,
                name: restaurant.name,
                comment: restaurant.comment,
                address: restaurant.address,
                url: restaurant.url,
                map_url: restaurant.map_url,
                parsed_at: restaurant.parsed_at,
                dishes,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Site {
        pub site_id: Uuid,
        pub city_id: Uuid,
        pub name: String,
        pub url_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub comment: Option<String>,
        pub restaurants: Vec<Restaurant>,
    }

    impl super::Id for Site {
        fn id(&self) -> Uuid {
            self.site_id
        }
    }

    impl From<super::Site> for Site {
        fn from(s: super::Site) -> Self {
            let mut restaurants: Vec<Restaurant> = s.restaurants.into_vec();
            restaurants.sort_by(|a, b| a.name.cmp(&b.name));
            Self {
                site_id: s.site_id,
                city_id: s.city_id,
                name: s.name,
                url_id: s.url_id,
                comment: s.comment,
                restaurants,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct City {
        pub city_id: Uuid,
        pub country_id: Uuid,
        pub name: String,
        pub url_id: String,
        pub sites: Vec<Site>,
    }

    impl super::Id for City {
        fn id(&self) -> Uuid {
            self.city_id
        }
    }

    impl From<super::City> for City {
        fn from(c: super::City) -> Self {
            let mut sites: Vec<Site> = c.sites.into_vec();
            sites.sort_by(|a, b| a.name.cmp(&b.name));
            Self {
                city_id: c.city_id,
                country_id: c.country_id,
                name: c.name,
                url_id: c.url_id,
                sites,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Country {
        pub country_id: Uuid,
        pub name: String,
        pub url_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub currency_suffix: Option<String>,
        pub cities: Vec<City>,
    }

    impl super::Id for Country {
        fn id(&self) -> Uuid {
            self.country_id
        }
    }

    impl From<super::Country> for Country {
        fn from(c: super::Country) -> Self {
            let mut cities: Vec<City> = c.cities.into_vec();
            cities.sort_by(|a, b| a.name.cmp(&b.name));
            Self {
                country_id: c.country_id,
                name: c.name,
                url_id: c.url_id,
                currency_suffix: c.currency_suffix,
                cities,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct LunchData {
        pub countries: Vec<Country>,
    }

    impl LunchData {
        pub fn new() -> Self {
            Default::default()
        }
    }

    impl From<super::LunchData> for LunchData {
        fn from(l: super::LunchData) -> Self {
            let mut countries: Vec<Country> = l.countries.into_vec();
            countries.sort_by(|a, b| a.name.cmp(&b.name));
            Self { countries }
        }
    }
}
