use anyhow::{Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{ErrorLevel, LevelFilter, Verbosity};
use compact_str::{CompactString, ToCompactString};
use tracing::{debug, instrument};
use tracing_subscriber::filter::LevelFilter as TFilter;

#[derive(Debug, Clone, Default, ValueEnum)]
pub enum LogFormat {
    Normal,
    Compact,
    Pretty,
    #[default]
    Json,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Log level verbosity
    #[command(flatten)]
    pub verbosity: Verbosity<ErrorLevel>,

    /// Which log formatter to use
    #[arg(short = 'f', long, default_value_t, value_enum)]
    pub log_format: LogFormat,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,
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

    /// Maps clap_verbosity_flag::LevelFilter values to tracing_subscriber::filter::LevelFilter
    /// values
    pub fn tracing_level_filter(&self) -> TFilter {
        match self.verbosity.log_level_filter() {
            LevelFilter::Off => TFilter::OFF,
            LevelFilter::Error => TFilter::ERROR,
            LevelFilter::Warn => TFilter::WARN,
            LevelFilter::Info => TFilter::INFO,
            LevelFilter::Debug => TFilter::DEBUG,
            LevelFilter::Trace => TFilter::TRACE,
        }
    }

    // might not use this in the end, but keeping it for signature reference, for now
    #[instrument]
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Scrape {} => {}
            Commands::Serve { listen, cron } => {
                let cron = cron.unwrap_or("UNDEF".to_compact_string());
                // let msg = format!("Listening on {} with schedule {}\n", listen, cron);
                // w.write_all(msg.as_bytes()).await?;
                debug!("Listening on {} with cron schedule: {}", listen, cron);
            }
        }
        Ok(())
    }

    // pub fn schedule<T>(&self, run: T) -> Result<Job, JobSchedulerError> {
    //     Job::new(self.cron)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_and_run() {
        let cli =
            Cli::parse_opts(["test", "-v", "serve", "--listen", ":1234", "--cron", "* *"]).unwrap();
        cli.run().await.unwrap();
    }
}
