// this file is only a scratchpad for testing new scrapers without including them in
// the scraping framework that updates the DB

use anyhow::Result;
use rlunch::cli;

#[tokio::main]
async fn main() -> Result<()> {
    // anyhow::bail!("Not implemented")
    // Test why fetch sometimes fail
    cli::Cli::parse_args().init_logger()?;
    // let c = get_client()?;
    // match get(
    //     &c,
    //     "https://lindholmen.uit.se/omradet/dagens-lunch?embed-mode=iframe",
    // )
    // .await
    // {
    //     Ok(_) => Ok(()),
    //     Err(e) => {
    //         dbg!(e);
    //         Ok(())
    //     }
    // }
    Ok(())
}
