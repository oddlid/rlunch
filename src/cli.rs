use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use compact_str::{CompactString, ToCompactString};
use tokio::io::AsyncWriteExt as _;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Log level verbosity
    #[command(flatten)]
    verbosity: Verbosity<InfoLevel>,

    /// Subcommand to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Perform a one-shot scrape and output the results
    Scrape {},
    /// Start the full server
    Serve {
        /// Listen address
        #[arg(short, long, default_value_t = CompactString::from(":20666"))]
        listen: CompactString,

        /// Cron spec for running scrapers
        #[arg(short, long)]
        cron: Option<CompactString>,
    },
}

impl Cli {
    /// Wrapper for clap::Parser::try_parse_from
    pub fn parse_opts<I, T>(itr: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::try_parse_from(itr).map_err(Error::from)
    }

    // might not use this in the end, but keeping it for signature reference, for now
    pub async fn run<W>(self, w: &mut W) -> Result<()>
    where
        W: tokio::io::AsyncWrite,
        W: std::marker::Unpin,
    {
        w.write_all(b"Cli::run was called\n").await?;

        match self.command {
            Commands::Scrape {} => {}
            Commands::Serve { listen, cron } => {
                let cron = cron.unwrap_or("UNDEF".to_compact_string());
                let msg = format!("Listening on {} witch schedule {}\n", listen, cron);
                w.write_all(msg.as_bytes()).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_and_run() {
        let cli =
            Cli::parse_opts(["test", "-v", "serve", "--listen", ":1234", "--cron", "* *"]).unwrap();
        cli.run(&mut tokio::io::stdout()).await.unwrap();
    }
}
