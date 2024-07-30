use anyhow::{bail, Result};
use compact_str::{CompactString, ToCompactString};
use lazy_static::lazy_static;
use rlunch::{data::*, util::*};
use scraper::{ElementRef, Html, Selector};

// This is just a test bed for the first attempt at parsing lindholmen.se

// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

lazy_static! {
    static ref SEL_VIEW_CONTENT: Selector = sel("div.view-content");
    static ref SEL_DISH: Selector = sel("span.dish-name");
    static ref SEL_DISH_TYPE: Selector = sel("div.icon-dish");
    static ref SEL_DISH_PRICE: Selector = sel("div.table-list__column--price");
}

// there's currently no benefit to having the async stuff in here,
// but I just want to get used to this way of coding
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

    let mut site = Site::new("Lindholmen");
    let mut cur_restaurant_name = CompactString::new("");

    for e in vc.child_elements() {
        match e.attr("class") {
            None => continue,
            Some(v) => {
                if v == "title" {
                    if let Some(name) = e.text().next().map(|v| v.trim().to_compact_string()) {
                        cur_restaurant_name = name;
                    }
                } else if let Some(d) = parse_dish(&e) {
                    if cur_restaurant_name.is_empty() {
                        continue;
                    }
                    let r = site
                        .restaurants
                        .entry(cur_restaurant_name.clone())
                        .or_insert_with(|| Restaurant::new(&cur_restaurant_name));
                    r.dishes.push(d);
                }
            }
        }
    }

    println!("{:#?}", site);

    Ok(())
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
