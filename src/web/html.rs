use super::{ApiContext, Result};
use crate::{
    db::{self},
    models::api::{LunchData, Site},
    signals::shutdown_signal,
};
use anyhow::Context;
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Router,
};
use compact_str::CompactString;
use minijinja::{context, Environment};
use minijinja_autoreload::AutoReloader;
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration;
use std::{path::PathBuf, sync::LazyLock};
use tokio::net::TcpListener;
use tower_http::{
    catch_panic::CatchPanicLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::trace;
use uuid::Uuid;

static LOADER: LazyLock<AutoReloader> = LazyLock::new(|| {
    #[allow(unused_variables)]
    AutoReloader::new(move |notifier| {
        let template_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
        let mut env = Environment::new();
        minijinja_contrib::add_to_environment(&mut env);
        env.set_trim_blocks(true);
        env.set_lstrip_blocks(true);

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

pub async fn serve(pg: PgPool, addr: &str, gtag: CompactString) -> anyhow::Result<()> {
    trace!(addr, "Starting HTTP server...");
    axum::serve(
        TcpListener::bind(addr).await?,
        html_router(ApiContext { db: pg, gtag }),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("failed to start HTTP server")
}

fn router() -> Router<ApiContext> {
    Router::new()
        .route("/", get(list_sites))
        .route("/site/:site_id", get(list_dishes_for_site))
}

fn html_router(ctx: ApiContext) -> Router {
    Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/favicon.ico", ServeFile::new("static/favicon.ico"))
        .merge(router())
        .layer((
            TraceLayer::new_for_http().on_failure(()),
            TimeoutLayer::new(Duration::from_secs(30)),
            CatchPanicLayer::new(),
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
        context!(gtag => &ctx.gtag, data),
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
        CompactString::from(",-")
    }();
    let site: Site = data.into_site(site_id)?.into();

    Ok(Html(render(
        "dishes_for_site.html",
        context!(gtag => &ctx.gtag, currency_suffix, site),
    )?))
}
