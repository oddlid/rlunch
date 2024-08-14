use anyhow::Result;

pub trait RestaurantScraper {
    fn run(&self) -> Result<()>;
}
