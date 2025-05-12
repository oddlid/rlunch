use anyhow::Result;
use http_cache_reqwest::{
    Cache, CacheMode, HttpCache, HttpCacheOptions, MokaCache, MokaCacheBuilder, MokaManager,
};
use reqwest::IntoUrl;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::RandomState,
    fs::File,
    io::{BufReader, BufWriter, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tracing::{debug, error, trace};

static APP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";

type MCache = MokaCache<String, Arc<Vec<u8>>, RandomState>;

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    key: String,
    value: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct CacheBuilder {
    store: Vec<CacheEntry>,
}

impl CacheBuilder {
    fn with_capacity(cap: usize) -> Self {
        Self {
            store: Vec::with_capacity(cap),
        }
    }

    /// Try to populate the given cache with contents of the given file.
    /// If it fails to load the file, an error will be logged, and the cache will be returned
    /// unmodified.
    /// The TTL for each cache entry will start to tick from insertion time, meaning that a very
    /// old cache file would still give "valid" results until the TTL has expired after creation.
    async fn populate_cache<P: AsRef<Path>>(path: P, cap: usize, cache: MCache) -> MCache {
        let mut this = Self::with_capacity(cap);
        if let Err(err) = this.load(path) {
            error!(%err, "Failed to load cache file");
            return cache; // unmodified
        }
        let mut cnt = 0;
        for e in this.store {
            cache.insert(e.key, Arc::new(e.value)).await;
            cnt += 1;
        }
        trace!("Loaded {} values from file into cache", cnt);
        cache
    }

    /// Consume the given cache and load its contents into the internal Vec,
    /// for saving to file.
    async fn from_cache(cache: MCache) -> Self {
        cache.run_pending_tasks().await;
        let mut this = Self::with_capacity(cache.entry_count() as usize);

        let iter = cache.iter();
        let mut cnt = 0;
        for (k, v) in iter {
            this.store.push(CacheEntry {
                key: (*k).clone(),
                value: (*v).clone(),
            });
            cnt += 1;
        }
        trace!("Loaded {} values from cache, for saving to file", cnt);

        this
    }

    /// Write whatever has been loaded in `from_cache` to the given file
    fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let mut f = BufWriter::new(File::create(path)?);
        bincode::serde::encode_into_std_write(&self.store, &mut f, bincode::config::standard())?;
        f.flush()?;
        Ok(())
    }

    /// Used by Self::populate_cache to load file contents into a new cache
    fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let mut f = BufReader::new(File::open(path)?);
        self.store = bincode::serde::decode_from_std_read(&mut f, bincode::config::standard())?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Opts {
    pub request_delay: Duration,
    pub request_timeout: Duration,
    pub cache_ttl: Duration,
    pub cache_capacity: usize,
    pub cache_path: Option<PathBuf>,
}

impl Opts {
    fn cache_mode(&self) -> CacheMode {
        if self.cache_ttl.is_zero() {
            // Disable caching alltogether if TTL is set to 0
            return CacheMode::NoStore;
        }
        // Using Default does not work when offline.
        // ForceCache works offline, and seems to work fine with TTL cache eviction as well
        CacheMode::ForceCache
    }

    fn build_cache(&self) -> MCache {
        MokaCacheBuilder::new(self.cache_capacity as u64)
            .name("LunchScraperCache")
            .time_to_live(self.cache_ttl)
            .build()
    }

    fn build_client(&self) -> reqwest::Result<reqwest::Client> {
        reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .timeout(self.request_timeout)
            .build()
    }
}

#[derive(Clone)]
pub struct Client {
    client: ClientWithMiddleware,
    cache: MCache,
    cache_path: Option<PathBuf>,
    request_delay: Duration,
}

impl Client {
    /// Build new Client from given options
    pub async fn build(opts: Opts) -> reqwest::Result<Self> {
        // if a file path is set, try to populate the cache from the file,
        // otherwise create empty cache
        let cache = match opts.cache_path.as_ref() {
            Some(p) => {
                CacheBuilder::populate_cache(p, opts.cache_capacity, opts.build_cache()).await
            }
            None => opts.build_cache(),
        };
        Ok(Self {
            client: ClientBuilder::new(opts.build_client()?)
                .with(Cache(HttpCache {
                    mode: opts.cache_mode(),
                    manager: MokaManager::new(cache.clone()),
                    options: HttpCacheOptions::default(),
                }))
                .build(),
            cache,
            cache_path: opts.cache_path,
            request_delay: opts.request_delay,
        })
    }

    pub fn request_delay(&self) -> Duration {
        self.request_delay
    }

    /// Consume self and write cache contents to file for later loading, if a file path was set at
    /// build time
    pub async fn save(self) -> Result<()> {
        // try to save to file if a path is given
        match self.cache_path {
            Some(p) => CacheBuilder::from_cache(self.cache).await.save(p),
            None => {
                debug!("No cache file path set, unable to save");
                Ok(())
            }
        }
    }

    /// Wrapper to make an HTTP GET request via the inner client instance, and get the body
    /// contents as a String
    pub async fn get_as_string<U: IntoUrl>(&self, url: U) -> Result<String> {
        self.client
            .get(url)
            .send()
            .await?
            .text()
            .await
            .map_err(anyhow::Error::from)
    }
}

/// Give access to the inner client via deref
impl Deref for Client {
    type Target = ClientWithMiddleware;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
