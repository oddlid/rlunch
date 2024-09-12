// This module might replace some of the stuff I first put in data.
// The structs in this module are a direct mapping of the DB structure,
// while the structs in the api sub-module are stripped versions of those intended for use in API
// output, and similar, where uuids and mappings are not needed.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::HashMap,
    convert::From,
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, sqlx::FromRow)]
pub struct UuidMap<T>(pub HashMap<Uuid, T>);

impl<T, U: std::convert::From<T>> From<Vec<T>> for UuidMap<U> {
    fn from(value: Vec<T>) -> Self {
        Self(
            value
                .into_iter()
                .map(|v| (Uuid::new_v4(), v.into()))
                .collect(),
        )
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
    pub tags: Vec<String>,
    /// Price, in whatever currency is in use
    pub price: f32,
}

impl Dish {
    pub fn new(name: &str) -> Self {
        Self {
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

impl From<api::Dish> for Dish {
    fn from(dish: api::Dish) -> Self {
        Self {
            name: dish.name,
            description: dish.description,
            comment: dish.comment,
            tags: dish.tags,
            price: dish.price,
            ..Default::default()
        }
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
    pub dishes: UuidMap<Dish>,
}

impl Restaurant {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            parsed_at: Local::now(),
            ..Default::default()
        }
    }

    pub fn new_for_site(name: &str, site_id: Uuid) -> Self {
        Self {
            name: name.into(),
            parsed_at: Local::now(),
            site_id,
            ..Default::default()
        }
    }

    // pub fn add(&mut self, d: Dish) -> Option<Dish> {
    //     self.dishes.insert(Uuid::new_v4(), d)
    // }
    //
    // pub fn into_dishes(self) -> UuidMap<Dish> {
    //     self.dishes
    // }
}

impl From<api::Restaurant> for Restaurant {
    fn from(restaurant: api::Restaurant) -> Self {
        Self {
            name: restaurant.name,
            comment: restaurant.comment,
            address: restaurant.address,
            url: restaurant.url,
            map_url: restaurant.map_url,
            parsed_at: restaurant.parsed_at,
            dishes: restaurant.dishes.into(),
            ..Default::default()
        }
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
    pub restaurants: UuidMap<Restaurant>,
}

impl Site {
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
pub struct City {
    #[serde(skip_serializing)]
    pub city_id: Uuid,
    #[serde(skip_serializing)]
    pub country_id: Uuid, // parent country
    pub name: String,
    pub url_id: String,
    #[sqlx(skip)]
    pub sites: UuidMap<Site>,
}

impl City {
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
pub struct Country {
    #[serde(skip_serializing)]
    pub country_id: Uuid,
    pub name: String,
    pub url_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_suffix: Option<String>,
    #[sqlx(skip)]
    pub cities: UuidMap<City>,
}

impl Country {
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
pub struct LunchData {
    /// List of current countries
    #[sqlx(skip)]
    pub countries: UuidMap<Country>,
}

impl LunchData {
    pub fn new() -> Self {
        Default::default()
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
    /// This module has versions of structs from the parent, stripped for use during scraping and
    /// for presentation in JSON API
    use chrono::{DateTime, Local};
    use serde::{Deserialize, Serialize};
    use std::convert::From;

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Dish {
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

    impl Dish {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.into(),
                ..Default::default()
            }
        }
    }

    impl From<super::Dish> for Dish {
        fn from(dish: super::Dish) -> Self {
            Self {
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

    impl Restaurant {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.into(),
                parsed_at: Local::now(),
                ..Default::default()
            }
        }
    }

    impl From<super::Restaurant> for Restaurant {
        fn from(mut restaurant: super::Restaurant) -> Self {
            Self {
                name: restaurant.name,
                comment: restaurant.comment,
                address: restaurant.address,
                url: restaurant.url,
                map_url: restaurant.map_url,
                parsed_at: restaurant.parsed_at,
                dishes: restaurant.dishes.drain().map(|(_, v)| v.into()).collect(),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
    #[serde(default)]
    pub struct Site {
        pub name: String,
        pub url_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub comment: Option<String>,
        pub restaurants: Vec<Restaurant>,
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
        pub name: String,
        pub url_id: String,
        pub sites: Vec<Site>,
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
        pub name: String,
        pub url_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub currency_suffix: Option<String>,
        pub cities: Vec<City>,
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
        pub countries: Vec<Country>,
    }

    impl LunchData {
        pub fn new() -> Self {
            Default::default()
        }
    }
}
