use anyhow::{bail, Result};
use lazy_static::lazy_static;
use rlunch::data::*;
use scraper::{selectable::Selectable, ElementRef, Html, Selector};

// This is just a test bed for the first attempt at parsing lindholmen.se

// Name your user agent after your app?
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);
const UNDEF: &str = "UNDEFINED";

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).unwrap()
}

lazy_static! {
    static ref SEL_VIEW_CONTENT: Selector = sel("div.view-content");
    static ref SEL_CONTENT: Selector = sel("div.content");
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
    // let vc = doc.select(&SEL_VIEW_CONTENT).next().unwrap();
    let vc = match doc.select(&SEL_VIEW_CONTENT).next() {
        Some(vc) => vc,
        None => bail!("Invalid HTML"),
    };
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

    // let mut site = Site::new("Lindholmen");
    // This does absolutely not work!
    // for titles in vc.select(&SEL_TITLE) {
    //     let restaurant_name = titles.text().next().unwrap_or(UNDEF).trim();
    //     println!("{restaurant_name}");
    //     while let Some(s) = titles.next_sibling() {
    //         println!("{:?}", s.value());
    //     }
    // }

    for e in vc.child_elements() {
        let dish_type = get_text(&e, &SEL_DISH_TYPE);
        let (dish_name, dish_desc) = get_dish_name_and_desc(&e);
        let dish_price = get_text(&e, &SEL_DISH_PRICE);
        println!(
            "=> Dish:\nType: {dish_type}\nName: {dish_name}\nDesc: {dish_desc}\nPrice: {dish_price}\n\n"
        );
    }

    Ok(())
}

fn get_text(e: &ElementRef, sel: &Selector) -> String {
    match e.select(sel).next() {
        None => UNDEF.to_owned(),
        Some(v) => match v.text().next() {
            None => UNDEF.to_owned(),
            Some(v) => v.trim().to_owned(),
        },
    }
}

fn reduce_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    s.split_whitespace().for_each(|w| {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(w);
    });
    result
}

fn get_dish_name_and_desc(e: &ElementRef) -> (String, String) {
    match e.select(&SEL_DISH).next() {
        None => (UNDEF.to_owned(), UNDEF.to_owned()),
        Some(v) => {
            let mut t = v.text();
            let name = match t.next() {
                None => UNDEF.to_owned(),
                Some(v) => v.trim().to_owned(),
            };
            let desc = match t.next() {
                None => UNDEF.to_owned(),
                Some(v) => reduce_whitespace(v),
            };
            (name, desc)
        }
    }
}
