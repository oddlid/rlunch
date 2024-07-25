use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::HashMap, hash_set::HashSet};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Dish {
    /// Name of the dish, e.g. "meatballs"
    pub name: String,
    /// More details about the dish, e.g. "with spaghetti"
    pub description: Option<String>,
    // Extra info, e.g. "contains nuts"
    pub comment: Option<String>,
    /// Optionals tags for filtering, e.g. "vego,gluten,lactose"
    pub tags: HashSet<String>,
    /// Price, in whatever currency is in use
    pub price: f32,
}

impl Default for Dish {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            comment: None,
            tags: HashSet::new(),
            price: 0.0,
        }
    }
}

#[allow(dead_code)]
impl Dish {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Restaurant {
    /// Name of restaurant
    pub name: String,
    /// Extra info
    pub comment: Option<String>,
    /// Street address
    pub address: Option<String>,
    /// Homepage
    pub url: Option<String>,
    /// Google maps URL
    pub map_url: Option<String>,
    /// When the scraping was last done
    pub parsed_at: DateTime<Local>,
    /// List of current dishes
    pub dishes: Vec<Dish>,
}

impl Default for Restaurant {
    fn default() -> Self {
        Self {
            name: String::new(),
            comment: None,
            address: None,
            url: None,
            map_url: None,
            parsed_at: Local::now(),
            dishes: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Site {
    /// Name of site/area
    pub name: String,
    /// Extra info
    pub comment: Option<String>,
    /// Homepage
    pub url: Option<String>,
    /// List of current restaurants
    pub restaurants: HashMap<String, Restaurant>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct City {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Country {
    pub name: String,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LunchData {}
