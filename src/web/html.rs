use super::{ApiContext, Result};
use crate::{
    db::{self},
    models::api::{LunchData, Site},
    signals::shutdown_signal,
};
use anyhow::Context;
use axum::{
    Router,
    extract::{Path, State},
    response::{Html, Redirect},
    routing::get,
};
use axum_embed::ServeEmbed;
use compact_str::CompactString;
use minijinja::{Environment, context};
use minijinja_autoreload::AutoReloader;
use reqwest::StatusCode;
use rust_decimal::prelude::*;
use rust_embed::RustEmbed;
use serde::Serialize;
use shadow_rs::shadow;
use sqlx::PgPool;
use std::{borrow::Cow, time::Duration};
use std::{path::PathBuf, sync::LazyLock};
use tokio::net::TcpListener;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::trace;
use uuid::Uuid;

shadow!(build);

#[derive(serde::Serialize)]
struct BuildInfo<'a> {
    build_date: Cow<'a, str>,
    commit_date: Cow<'a, str>,
    commit_hash: Cow<'a, str>,
    commit_author: Cow<'a, str>,
    pkg_version: Cow<'a, str>,
}

impl BuildInfo<'_> {
    fn new() -> Self {
        Self {
            build_date: Cow::from(build::BUILD_TIME),
            commit_date: Cow::from(build::COMMIT_DATE),
            commit_hash: Cow::from(build::COMMIT_HASH),
            commit_author: Cow::from(build::COMMIT_AUTHOR),
            pkg_version: Cow::from(build::PKG_VERSION),
        }
    }
}

#[derive(RustEmbed, Clone)]
#[folder = "static/"]
struct Assets;

// filter function for template to display price in a more normal human format
fn strip_zeros(v: f32) -> String {
    if let Some(d) = Decimal::from_f32(v) {
        return d.normalize().to_string();
    }
    format!("{:.2}", v)
}

static LOADER: LazyLock<AutoReloader> = LazyLock::new(|| {
    #[allow(unused_variables)]
    AutoReloader::new(move |notifier| {
        let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
        let mut env = Environment::new();
        minijinja_contrib::add_to_environment(&mut env);
        env.set_trim_blocks(true);
        env.set_lstrip_blocks(true);
        env.add_filter("stripz", strip_zeros);

        #[cfg(feature = "bundled")]
        {
            minijinja_embed::load_templates!(&mut env);
        }

        #[cfg(not(feature = "bundled"))]
        {
            env.set_loader(minijinja::path_loader(&template_path));
            notifier.set_fast_reload(true);
            notifier.watch_path(&template_path, true);
        }

        Ok(env)
    })
});

pub async fn serve(pg: PgPool, addr: &str) -> anyhow::Result<()> {
    trace!(addr, "Starting HTTP server...");
    axum::serve(
        TcpListener::bind(addr).await?,
        html_router(ApiContext { db: pg }),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("failed to start HTTP server")
}

fn router() -> Router<ApiContext> {
    Router::new()
        .route("/", get(list_sites))
        .route("/site/{site_id}", get(list_dishes_for_site))
        // I found out that I had solved this in the Go version by letting the Caddy
        // frontend handle the rewrite. But it doesn't hurt to have this here as well, so I know
        // how to do it in just Rust.
        .route(
            "/favicon.ico",
            get(|| async { Redirect::permanent("/static/favicon.ico") }),
        )
}

fn html_router(ctx: ApiContext) -> Router {
    Router::new()
        .nest_service("/static", ServeEmbed::<Assets>::new())
        .merge(router())
        .layer((
            TraceLayer::new_for_http().on_failure(()),
            TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)),
            CatchPanicLayer::new(),
            CompressionLayer::new(),
        ))
        .with_state(ctx)
}

fn render<S: Serialize>(name: &str, ctx: S) -> Result<String> {
    let env = LOADER.acquire_env().map_err(anyhow::Error::from)?;
    let tmpl = env.get_template(name).map_err(anyhow::Error::from)?;
    let content = tmpl.render(ctx).map_err(anyhow::Error::from)?;
    Ok(content)
}

async fn list_sites(ctx: State<ApiContext>) -> Result<Html<String>> {
    let data: LunchData = db::list_all_sites(&mut ctx.get_tx().await?).await?.into();

    Ok(Html(render(
        "sites.html",
        context!(data, build => BuildInfo::new()),
    )?))
}

async fn list_dishes_for_site(
    ctx: State<ApiContext>,
    Path(site_id): Path<Uuid>,
) -> Result<Html<String>> {
    super::check_id(site_id)?;
    let data = db::list_dishes_for_site_by_id(&mut ctx.get_tx().await?, site_id).await?;
    let currency_suffix = || -> CompactString {
        for country in data.countries.values() {
            if let Some(ref v) = country.currency_suffix {
                return CompactString::from(v);
            }
        }
        CompactString::from("")
    }();
    // TODO: Consider if we should extract all useful info from the chain of ancestors,
    // to use as a bread crumb back in the template, before we lose all parent info here.
    let site: Site = data.into_site(site_id)?.into();

    Ok(Html(render(
        "dishes_for_site.html",
        context!(currency_suffix, site, build => BuildInfo::new()),
    )?))
}
