// All DB operations that are more than a single query, should be done in a transaction, even read
// operations, since the view could be inconsistent if a scraper updates a site while we're in the
// middle of several queries for that site, we could be referencing data that is not there anymore,
// since a site update will delete all restaurants and dishes before inserting it again with new
// uuids.
// This means to possibly write some repeated SQL instead of reusing smaller functions, unless the
// smaller functions can work within a transaction.

use crate::{
    models::{RestaurantRows, Site, SiteWithCurrency},
    scrape::ScrapeResult,
};
use anyhow::{Error, Result};
use sqlx::{Executor, PgPool, Postgres};
use std::time::Instant;
use tracing::trace;
use uuid::Uuid;

#[derive(Debug)]
pub struct SiteKey<'a> {
    pub country_url_id: &'a str,
    pub city_url_id: &'a str,
    pub site_url_id: &'a str,
}

#[derive(Debug, Clone, Default, PartialEq, sqlx::FromRow)]
#[sqlx(default)]
pub struct SiteRelation {
    pub country_id: Uuid,
    pub city_id: Uuid,
    pub site_id: Uuid,
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

// pub async fn get_site_uuid(pg: &PgPool, key: SiteKey<'_>) -> Result<Uuid> {
//     trace!(?key, "Searching for site ID...");
//
//     let id = sqlx::query_scalar!(
//         r#"
//             with co as (
//                 select country_id from country where url_id = $1
//             ), ci as (
//                 select city_id from city, co where city.country_id = co.country_id and url_id = $2
//             )
//             select site_id from site, ci where site.city_id = ci.city_id and url_id = $3;
//         "#,
//         key.country_url_id,
//         key.city_url_id,
//         key.site_url_id
//     )
//     .fetch_one(pg)
//     .await?;
//
//     trace!(%id, "ID  found");
//
//     Ok(id)
// }

pub async fn get_site_relation<'e, E>(executor: E, key: SiteKey<'_>) -> Result<SiteRelation>
where
    E: Executor<'e, Database = Postgres>,
{
    trace!(?key, "Searching for site relation...");

    let rel = sqlx::query_as::<_, SiteRelation>(
        r#"
            with co as (
                select country_id from country where url_id = $1
            ), ci as (
                select city_id from city, co where city.country_id = co.country_id and url_id = $2
            )
            select co.country_id, ci.city_id, site_id from co, ci, site where site.city_id = ci.city_id and url_id = $3;
        "#,
    )
    .bind(key.country_url_id)
    .bind(key.city_url_id)
    .bind(key.site_url_id)
    .fetch_one(executor)
    .await?;

    trace!(?rel, "Relation  found");

    Ok(rel)
}

pub async fn get_site_by_id(pg: &PgPool, site_id: Uuid) -> Result<SiteWithCurrency> {
    let site = sqlx::query_as::<_, Site>(
        r#"
            select * from site where site_id = $1
        "#,
    )
    .bind(site_id)
    .fetch_one(pg)
    .await?;

    todo!("implement")
}

pub async fn update_site(pg: &PgPool, update: ScrapeResult) -> Result<()> {
    trace!(site_id = %update.site_id, "Adding {} restaurants and {} dishes to DB", update.num_restaurants(), update.num_dishes());

    let start = Instant::now();
    // convert to format suitable for use with unnest
    let rs = RestaurantRows::from(update.restaurants);
    let duration = start.elapsed();
    trace!("Conversion to DB format done in {:?}", duration);

    // we need a transaction to ensure these operations are done atomically
    let mut tx = pg.begin().await?;

    let start = Instant::now();
    // first, clear out all restaurants and their dishes, so that we don't have any stale data
    // lingering. We have "on delete cascade" for dishes, so we just need to delete the parent
    // restaurants to get rid of all.
    sqlx::query!("delete from restaurant where site_id = $1", update.site_id)
        .execute(&mut *tx)
        .await?;

    // insert all restaurants
    sqlx::query!(
        r#"
            insert into restaurant (site_id, restaurant_id, restaurant_name, comment, address, url, map_url, created_at)
            select * from unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::timestamptz[])
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
