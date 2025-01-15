use super::{check_id, ApiContext, ListQuery, ListQueryLevel, Result};
use crate::{
    db::{self, SiteKey},
    models::api::LunchData,
    signals::shutdown_signal,
};
use anyhow::Context;
use axum::{
    extract::{Path, Query, State},
    response::Redirect,
    routing::get,
    Json, Router,
};
use compact_str::CompactString;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::trace;
use uuid::Uuid;

pub async fn serve(pg: PgPool, addr: &str) -> anyhow::Result<()> {
    trace!(addr, "Starting HTTP API server...");
    axum::serve(
        TcpListener::bind(addr).await?,
        api_router(ApiContext {
            db: pg,
            gtag: CompactString::from(""),
        }),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("failed to start HTTP API server")
}

fn api_router(ctx: ApiContext) -> Router {
    Router::new()
        .merge(router())
        .layer((
            TraceLayer::new_for_http().on_failure(()),
            TimeoutLayer::new(Duration::from_secs(30)),
            CatchPanicLayer::new(),
            CompressionLayer::new(),
        ))
        .with_state(ctx)
}

fn router() -> Router<ApiContext> {
    Router::new()
        .route("/", get(|| async { Redirect::permanent("/countries/") }))
        .route("/countries/", get(list_countries))
        .route("/cities/{country_id}", get(list_cities))
        .route("/sites/{city_id}", get(list_sites))
        .route("/restaurants/{site_id}", get(list_restaurants))
        .route(
            "/dishes/restaurant/{restaurant_id}",
            get(list_dishes_for_restaurant),
        )
        .route("/dishes/site/{site_id}", get(list_dishes_for_site))
        .route("/list/", get(list))
}

async fn list(ctx: State<ApiContext>, Query(q): Query<ListQuery>) -> Result<Json<LunchData>> {
    match q.level() {
        // Until we have support for a restaurant level for SiteKey, we do the same for
        // both restaurant and site level here
        lvl @ ListQueryLevel::Site | lvl @ ListQueryLevel::Restaurant => {
            trace!("Level: {:?}", lvl);
            let start = Instant::now();
            let res = db::list_dishes_for_site_by_key(
                &mut ctx.get_tx().await?,
                SiteKey::new(
                    &q.country.unwrap_or_default(),
                    &q.city.unwrap_or_default(),
                    &q.site.unwrap_or_default(),
                ),
            )
            .await?;
            trace!("Fetched restaurant list in {:?}", start.elapsed());
            Ok(Json(res.into()))
        }
        lvl @ ListQueryLevel::City => {
            trace!("Level: {:?}", lvl);
            let start = Instant::now();
            let res = db::list_sites_for_city_by_key(
                &mut ctx.get_tx().await?,
                SiteKey::new(
                    &q.country.unwrap_or_default(),
                    &q.city.unwrap_or_default(),
                    "",
                ),
            )
            .await?;
            trace!("Fetched site list in {:?}", start.elapsed());
            Ok(Json(res.into()))
        }
        lvl @ ListQueryLevel::Country => {
            trace!("Level: {:?}", lvl);
            let start = Instant::now();
            let res = db::list_cities_for_country_by_key(
                &mut ctx.get_tx().await?,
                SiteKey::new(&q.country.unwrap_or_default(), "", ""),
            )
            .await?;
            trace!("Fetched city list in {:?}", start.elapsed());
            Ok(Json(res.into()))
        }
        lvl @ ListQueryLevel::Empty => {
            trace!("Level: {:?}", lvl);
            list_countries(ctx).await
        }
    }
}

async fn list_countries(ctx: State<ApiContext>) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_countries(&ctx.db).await?;
    let duration = start.elapsed();
    trace!("Fetched country list in {:?}", duration);
    Ok(Json(res.into()))
}

async fn list_cities(
    ctx: State<ApiContext>,
    Path(country_id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    check_id(country_id)?;
    let start = Instant::now();
    let res = db::list_cities_for_country_by_id(&mut ctx.get_tx().await?, country_id).await?;
    let duration = start.elapsed();
    trace!("Fetched city list in {:?}", duration);
    Ok(Json(res.into()))
}

async fn list_sites(ctx: State<ApiContext>, Path(city_id): Path<Uuid>) -> Result<Json<LunchData>> {
    check_id(city_id)?;
    let start = Instant::now();
    let res = db::list_sites_for_city_by_id(&mut ctx.get_tx().await?, city_id).await?;
    let duration = start.elapsed();
    trace!("Fetched site list in {:?}", duration);
    Ok(Json(res.into()))
}

async fn list_restaurants(
    ctx: State<ApiContext>,
    Path(site_id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    check_id(site_id)?;
    let start = Instant::now();
    let res = db::list_restaurants_for_site_by_id(&mut ctx.get_tx().await?, site_id).await?;
    let duration = start.elapsed();
    trace!("Fetched restaurant list in {:?}", duration);
    Ok(Json(res.into()))
}

async fn list_dishes_for_restaurant(
    ctx: State<ApiContext>,
    Path(restaurant_id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    check_id(restaurant_id)?;
    let start = Instant::now();
    let res = db::list_dishes_for_restaurant_by_id(&mut ctx.get_tx().await?, restaurant_id).await?;
    let duration = start.elapsed();
    trace!("Fetched dishes for restaurant list in {:?}", duration);
    Ok(Json(res.into()))
}

async fn list_dishes_for_site(
    ctx: State<ApiContext>,
    Path(site_id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    check_id(site_id)?;
    let start = Instant::now();
    let res = db::list_dishes_for_site_by_id(&mut ctx.get_tx().await?, site_id).await?;
    let duration = start.elapsed();
    trace!("Fetched dishes for site list in {:?}", duration);
    Ok(Json(res.into()))
}
