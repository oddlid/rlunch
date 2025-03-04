use anyhow::{Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{log::LevelFilter, ErrorLevel, Verbosity};
use compact_str::CompactString;
use shadow_rs::shadow;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{io, path::PathBuf};
use tracing_subscriber::{
    filter::LevelFilter as TFilter,
    fmt::{self, time::ChronoLocal},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

shadow!(build);

#[derive(Debug, Clone, Default, ValueEnum)]
pub enum LogFormat {
    Normal,
    Compact,
    Pretty,
    #[default]
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_version = build::CLAP_LONG_VERSION, propagate_version = true)]
pub struct Cli {
    /// Log level verbosity
    #[command(flatten)]
    pub verbosity: Verbosity<ErrorLevel>,

    /// Which log formatter to use.
    /// The value can also be picked up from env if the key in uppercase has a valid value.
    #[arg(short = 'f', long, env, default_value_t, value_enum)]
    pub log_format: LogFormat,

    /// URL for Postgres database backend.
    /// The value can also be picked up from env if the key in uppercase has a valid value.
    #[arg(short, long, env)]
    pub database_url: String,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Start scraper manager
    Scrape {
        /// Cron spec for running scrapers.
        /// Leave unset to run a one-off scrape.
        #[arg(long)]
        cron: Option<CompactString>,

        /// How long to wait between requests to the same site.
        /// Useful to not get blocked for DDoS'ing target sites.
        #[arg(short = 'd', long, default_value = "1500ms")]
        request_delay: humantime::Duration,

        /// How long to wait before timing out a request
        #[arg(short = 't', long, default_value = "5s")]
        request_timeout: humantime::Duration,

        /// Time To Live for a cached request.
        /// Set to 0 to disable caching alltogether.
        /// If set, adjust in relation to cron schedule, to get the desired behavior.
        #[arg(short = 'l', long, default_value = "20m")]
        cache_ttl: humantime::Duration,

        /// Max items in cache.
        /// Adjust according to how many scrapers, and how many different page requests they make
        /// combined.
        #[arg(short = 'c', long, default_value_t = 64)]
        cache_capacity: usize,

        /// Path for saving cache between runs.
        /// Leave unset to disable saving/loading from file.
        /// Useful to set for local development, in order to not hammer the target scraping sites too
        /// much.
        #[arg(short = 'p', long)]
        cache_path: Option<PathBuf>,
    },
    /// Start a server
    Serve {
        /// Listen address
        #[arg(short, long, default_value_t = CompactString::from("[::]:20666"))]
        listen: CompactString,

        /// What kind of server to start
        #[command(subcommand)]
        commands: ServeCommands,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServeCommands {
    /// Start a REST API JSON server
    Json,
    /// Start HTML web server
    Html,
    /// Unimplemented
    Admin,
}

impl Cli {
    // This one turned out to not be so nice when supplying help or version flags in combination
    // with returning a Result from main, since it will then print "Error: <app description>",
    // which is a bit misleading.
    // The idea with this wrapper was to make the parsing testable, but I guess that's overkill
    // anyways.
    /// Wrapper for clap::Parser::try_parse_from
    pub fn try_parse_opts<I, T>(itr: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::try_parse_from(itr).map_err(Error::from)
    }

    // this thin wrapper makes it possible to do the parsing without importing clap::Parser at the
    // call site
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Maps clap_verbosity_flag::LevelFilter values to tracing_subscriber::filter::LevelFilter
    /// values
    fn tracing_level_filter(&self) -> TFilter {
        match self.verbosity.log_level_filter() {
            LevelFilter::Off => TFilter::OFF,
            LevelFilter::Error => TFilter::ERROR,
            LevelFilter::Warn => TFilter::WARN,
            LevelFilter::Info => TFilter::INFO,
            LevelFilter::Debug => TFilter::DEBUG,
            LevelFilter::Trace => TFilter::TRACE,
        }
    }

    /// Initialize logging via the tracing crate
    pub fn init_logger(&self) -> Result<()> {
        let layer = match self.log_format {
            LogFormat::Json => fmt::layer()
                .json()
                .with_writer(io::stderr)
                .with_timer(ChronoLocal::rfc_3339())
                .boxed(),
            LogFormat::Pretty => fmt::layer()
                .pretty()
                .with_writer(io::stderr)
                .with_timer(ChronoLocal::rfc_3339())
                .boxed(),
            LogFormat::Compact => fmt::layer()
                .without_time()
                .compact()
                .with_writer(io::stderr)
                .boxed(),
            LogFormat::Normal => fmt::layer()
                .with_writer(io::stderr)
                .with_timer(ChronoLocal::rfc_3339())
                .boxed(),
        };
        tracing_subscriber::registry()
            .with(
                EnvFilter::builder()
                    .with_default_directive(self.tracing_level_filter().into())
                    .from_env()?,
            )
            .with(layer)
            .init();
        Ok(())
    }

    pub async fn get_pg_pool(&self) -> Result<PgPool> {
        PgPoolOptions::new()
            .max_connections(20) // TODO: evaluate this value
            .connect(&self.database_url)
            .await
            .map_err(Error::from)
    }
}
