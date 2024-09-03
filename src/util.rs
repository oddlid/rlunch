use compact_str::{CompactString, ToCompactString};
use nom::number::complete;
use scraper::{ElementRef, Selector};

// use rand::prelude::*;
// use tokio::time::{sleep, Duration};

pub fn sel(selector: &str) -> Selector {
    Selector::parse(selector).unwrap()
}

pub fn get_text(e: &ElementRef, sel: &Selector) -> Option<CompactString> {
    match e.select(sel).next() {
        None => None,
        Some(v) => v.text().next().map(|v| v.trim().to_compact_string()),
    }
}

pub fn parse_float(s: &str) -> f32 {
    match complete::float::<_, ()>(s) {
        Ok((_, v)) => v,
        _ => 0.0,
    }
}

pub fn reduce_whitespace(s: &str) -> CompactString {
    s.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_compact_string()
}

// we need to have this split into a separate function, so that thread_rng is dropped before the
// call to sleep, since ThreadRng is not Send
// fn get_random_ms(min: u64, max: u64) -> u64 {
//     thread_rng().gen_range(min..=max)
// }
//
// pub async fn wait_random_range_ms(min: u64, max: u64) {
//     sleep(Duration::from_millis(get_random_ms(min, max))).await
// }
