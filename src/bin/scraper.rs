use anyhow::{bail, Result};
use compact_str::{CompactString, ToCompactString};
use lazy_static::lazy_static;
use nom::number::complete;
use rlunch::data::*;
use scraper::{selectable::Selectable, ElementRef, Html, Selector};

// This is just a test bed for the first attempt at parsing lindholmen.se

// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
// const UNDEF: &str = "UNDEFINED";

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).unwrap()
}

lazy_static! {
    static ref SEL_VIEW_CONTENT: Selector = sel("div.view-content");
    static ref SEL_CONTENT: Selector = sel("div.content");
    static ref SEL_VIEW_LUNCH: Selector = sel("div.view-id-dagens_lunch");
    static ref SEL_TITLE: Selector = sel("h3.title");
    static ref SEL_DISH_ROW: Selector = sel("div.table-list__row");
    static ref SEL_DISH: Selector = sel("span.dish-name");
    static ref SEL_DISH_NAME: Selector = sel("strong");
    static ref SEL_DISH_TYPE: Selector = sel("div.icon-dish");
    static ref SEL_DISH_PRICE: Selector = sel("div.table-list__column--price");
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;
    let res = client
        .get("http://localhost:8080")
        .send()
        .await?
        .text()
        .await?;

    let doc = Html::parse_document(&res);
    let vc = match doc.select(&SEL_VIEW_CONTENT).next() {
        Some(vc) => vc,
        None => bail!("Invalid HTML"),
    };

    for e in vc.child_elements() {
        match e.attr("class") {
            None => continue,
            Some(v) => {
                if v == "title" {
                    if let Some(name) = e.text().next().map(|v| v.trim().to_compact_string()) {
                        println!("Restaurant: {}", name);
                    }
                } else if let Some(d) = parse_dish(&e) {
                    println!("{d:?}")
                }
            }
        }
    }

    // for restaurant_block in vc.select(&SEL_TITLE) {
    //     let name = restaurant_block
    //         .text()
    //         .map(|e| e.trim())
    //         .collect::<Vec<_>>()
    //         .join("");
    //     println!("{}", name);
    // }

    // this works, for just getting the restaurant names
    // let restaurants = vc
    //     .select(&SEL_TITLE)
    //     .map(|t| t.text().next().unwrap_or(UNDEF).trim())
    //     .map(Restaurant::new)
    //     .collect::<Vec<_>>();
    // println!("{:#?}", restaurants);

    // let mut dishes: Vec<Dish> = Vec::new();

    // if let Some(s) = doc.select(&sel("h3.title ~ div.table-list__row")).next() {
    //     println!("{:?}", s.value());
    // }
    // for e in doc.select(&sel("h3.title ~ :not(h3.title)")) {
    //     println!("{:?}", e.value());
    // }

    // for r in doc.select(&sel("h3.title")) {
    //     println!("{:?}", r.value());
    //     for d in r.select(&sel("div.table-list__row ~ :not(h3.title)")) {
    //         println!("{:?}", d.value());
    //     }
    // }

    Ok(())
}

fn parse_dish(e: &ElementRef) -> Option<Dish> {
    let (name, description) = get_dish_name_and_desc(e);
    let price = match get_text(e, &SEL_DISH_PRICE) {
        None => 0.0,
        Some(v) => parse_float(v.trim()),
    };
    let dish = Dish {
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

fn get_text(e: &ElementRef, sel: &Selector) -> Option<CompactString> {
    match e.select(sel).next() {
        None => None,
        Some(v) => v.text().next().map(|v| v.trim().to_compact_string()),
    }
}

fn parse_float(s: &str) -> f32 {
    match complete::float::<_, ()>(s) {
        Ok((_, v)) => v,
        _ => 0.0,
    }
}

fn reduce_whitespace(s: &str) -> CompactString {
    s.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_compact_string()
}

fn get_dish_name_and_desc(e: &ElementRef) -> (Option<CompactString>, Option<CompactString>) {
    // match e.select(&SEL_DISH).next() {
    //     None => (UNDEF.to_compact_string(), UNDEF.to_compact_string()),
    //     Some(v) => {
    //         let mut t = v.text();
    //         let name = match t.next() {
    //             None => UNDEF.to_compact_string(),
    //             Some(v) => v.trim().to_compact_string(),
    //         };
    //         let desc = match t.next() {
    //             None => UNDEF.to_compact_string(),
    //             Some(v) => reduce_whitespace(v),
    //         };
    //         (name, desc)
    //     }
    // }
    // match e.select(&SEL_DISH).next() {
    //     None => (None, None),
    //     Some(v) => {
    //         let mut t = v.text();
    //         let name = match t.next() {
    //             None => None,
    //             Some(v) => Some(v.trim().to_compact_string()),
    //         };
    //         let desc = match t.next() {
    //             None => None,
    //             Some(v) => Some(reduce_whitespace(v)),
    //         };
    //         (name, desc)
    //     }
    // }
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
