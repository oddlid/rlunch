use compact_str::{CompactString, ToCompactString};
use nom::number::complete;
use scraper::{ElementRef, Selector};

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
