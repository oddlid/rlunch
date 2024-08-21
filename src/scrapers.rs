pub mod se {
    pub mod gbg {
        pub mod lh {
            // use crate::data;
            use crate::scrape::{RestaurantScraper, ScrapeResult};
            use anyhow::{anyhow, Result};

            #[derive(Default, Clone, Debug)]
            pub struct LHScraper {}

            impl RestaurantScraper for LHScraper {
                async fn run(&self) -> Result<ScrapeResult> {
                    Err(anyhow!("LHScraper not yet implemented"))
                }

                fn name(&self) -> &'static str {
                    "LHScraper"
                }
            }
        }
    }
}
