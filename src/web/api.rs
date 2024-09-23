use super::{ApiContext, Result};
use crate::{db, models::LunchData, signals::shutdown_signal};
use anyhow::Context;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
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
        api_router(ApiContext { db: pg }),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("failed to start HTTP API server")
}

fn api_router(ctx: ApiContext) -> Router {
    Router::new()
        .merge(router())
        .layer((
            CompressionLayer::new(),
            TraceLayer::new_for_http().on_failure(()),
            TimeoutLayer::new(Duration::from_secs(30)),
            CatchPanicLayer::new(),
        ))
        .with_state(ctx)
}

fn router() -> Router<ApiContext> {
    Router::new()
        .route("/", get(list_countries))
        .route("/cities/:country_id", get(list_cities))
        .route("/sites/:city_id", get(list_sites))
        .route("/restaurants/:site_id", get(list_restaurants))
        .route(
            "/dishes/restaurant/:restaurant_id",
            get(list_dishes_for_restaurant),
        )
        .route("/dishes/site/:site_id", get(list_dishes_for_site))
}

async fn list_countries(ctx: State<ApiContext>) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_countries(&ctx.db).await?;
    let duration = start.elapsed();
    trace!("Fetched country list in {:?}", duration);
    Ok(Json(res))
}

async fn list_cities(ctx: State<ApiContext>, Path(id): Path<Uuid>) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_cities(&ctx.db, id).await?;
    let duration = start.elapsed();
    trace!("Fetched city list in {:?}", duration);
    Ok(Json(res))
}

async fn list_sites(ctx: State<ApiContext>, Path(id): Path<Uuid>) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_sites(&ctx.db, id).await?;
    let duration = start.elapsed();
    trace!("Fetched site list in {:?}", duration);
    Ok(Json(res))
}

async fn list_restaurants(ctx: State<ApiContext>, Path(id): Path<Uuid>) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_restaurants(&ctx.db, id).await?;
    let duration = start.elapsed();
    trace!("Fetched restaurant list in {:?}", duration);
    Ok(Json(res))
}

async fn list_dishes_for_restaurant(
    ctx: State<ApiContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_dishes_for_restaurant(&ctx.db, id).await?;
    let duration = start.elapsed();
    trace!("Fetched dishes for restaurant list in {:?}", duration);
    Ok(Json(res))
}

async fn list_dishes_for_site(
    ctx: State<ApiContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<LunchData>> {
    let start = Instant::now();
    let res = db::list_dishes_for_site(&ctx.db, id).await?;
    let duration = start.elapsed();
    trace!("Fetched dishes for site list in {:?}", duration);
    Ok(Json(res))
}
