// All DB operations that are more than a single query, should be done in a transaction, even read
// operations, since the view could be inconsistent if a scraper updates a site while we're in the
// middle of several queries for that site, we could be referencing data that is not there anymore,
// since a site update will delete all restaurants and dishes before inserting it again with new
// uuids.
// This means to possibly write some repeated SQL instead of reusing smaller functions, unless the
// smaller functions can work within a transaction.

use crate::{
    models::{City, Country, Dish, LunchData, Restaurant, RestaurantRows, Site},
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

pub async fn get_site_by_id(pg: &PgPool, site_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let mut site = sqlx::query_as::<_, Site>(
        r#"
            select * from site where site_id = $1
        "#,
    )
    .bind(site_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut city = sqlx::query_as::<_, City>(
        r#"
            select * from city where city_id = $1
        "#,
    )
    .bind(site.city_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut country = sqlx::query_as::<_, Country>(
        r#"
            select * from country where country_id = $1
        "#,
    )
    .bind(city.country_id)
    .fetch_one(&mut *tx)
    .await?;

    let restaurants: Vec<Restaurant> = sqlx::query_as(
        r#"
            select * from restaurant where site_id = $1
        "#,
    )
    .bind(site.site_id)
    .fetch_all(&mut *tx)
    .await?;

    let mut restaurant_ids = Vec::new();
    for r in &restaurants {
        restaurant_ids.push(r.restaurant_id);
    }
    let dishes: Vec<Dish> = sqlx::query_as(
        r#"
            select
                dish_id,
                restaurant_id,
                dish_name,
                description,
                comment,
                string_to_array(tags, ',') as tags,
                price,
                created_at
                from dish where restaurant_id in (select unnest($1::uuid[]))
                group by dish_id
        "#,
    )
    .bind(restaurant_ids)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    for r in restaurants {
        site.restaurants.insert(r.restaurant_id, r);
    }

    for d in dishes {
        if let Some(r) = site.restaurants.get_mut(&d.restaurant_id) {
            r.dishes.insert(d.dish_id, d);
        }
    }

    city.sites.insert(site.site_id, site);
    country.cities.insert(city.city_id, city);

    let mut ld = LunchData::new();
    ld.countries.insert(country.country_id, country);

    Ok(ld)
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
