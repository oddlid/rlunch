use crate::{models::RestaurantRows, scrape::ScrapeResult};
use anyhow::{Error, Result};
use sqlx::PgPool;
use std::time::Instant;
use tracing::trace;
use uuid::Uuid;

#[derive(Debug)]
pub struct SiteKey<'a> {
    pub country_url_id: &'a str,
    pub city_url_id: &'a str,
    pub site_url_id: &'a str,
}

impl<'a> SiteKey<'a> {
    pub fn new(country_url_id: &'a str, city_url_id: &'a str, site_url_id: &'a str) -> Self {
        Self {
            country_url_id,
            city_url_id,
            site_url_id,
        }
    }
}

pub async fn get_site_uuid(pg: &PgPool, key: SiteKey<'_>) -> Result<Uuid> {
    trace!(?key, "Searching for site ID...");

    let id = sqlx::query_scalar!(
        r#"
            with co as (
                select country_id from country where url_id = $1
            ), ci as (
                select city_id from city, co where city.country_id = co.country_id and url_id = $2
            )
            select site_id from site, ci where site.city_id = ci.city_id and url_id = $3;
        "#,
        key.country_url_id,
        key.city_url_id,
        key.site_url_id
    )
    .fetch_one(pg)
    .await?;

    trace!(%id, "ID  found");

    Ok(id)
}

pub async fn update_site(pg: &PgPool, update: ScrapeResult) -> Result<()> {
    trace!(site_id = %update.site_id, "Adding {} restaurants and {} dishes to DB", update.num_restaurants(), update.num_dishes());

    let start = Instant::now();
    // convert to format suitable for use with unnest
    let rs = RestaurantRows::from(update.restaurants);
    let duration = start.elapsed();
    trace!("Conversion to DB format done in {:?}", duration);

    let mut tx = pg.begin().await?;

    let start = Instant::now();
    // first, clear out all restaurants and their dishes, so that we don't have any stale data
    // lingering
    sqlx::query!("delete from restaurant where site_id = $1", update.site_id)
        .execute(&mut *tx)
        .await?;

    // insert all restaurants
    sqlx::query!(
        r#"
            insert into restaurant (site_id, restaurant_id, restaurant_name, comment, address, url, map_url, created_at)
            select * from    unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::timestamptz[])
        "#,
        &rs.site_ids[..],
        &rs.restaurant_ids[..],
        &rs.names[..],
        &rs.comments as &[Option<String>],
        &rs.addresses as &[Option<String>],
        &rs.urls as &[Option<String>],
        &rs.map_urls as &[Option<String>],
        &rs.parsed_ats[..],
    )
    .execute(&mut *tx)
    .await?;

    // insert all dishes
    sqlx::query!(
        r#"
            insert into dish (restaurant_id, dish_id, dish_name, description, comment, price, tags)
            select * from unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::real[], $7::text[])
        "#,
        &rs.dishes.restaurant_ids[..],
        &rs.dishes.dish_ids[..],
        &rs.dishes.names[..],
        &rs.dishes.descriptions as &[Option<String>],
        &rs.dishes.comments as &[Option<String>],
        &rs.dishes.prices[..],
        &rs.dishes.tags[..],
    ).execute(&mut *tx).await?;
    let duration = start.elapsed();

    trace!("DB update done in {:?}", duration);

    tx.commit().await.map_err(Error::from)
}
