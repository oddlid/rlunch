// All DB operations that are more than a single query, should be done in a transaction, even read
// operations, since the view could be inconsistent if a scraper updates a site while we're in the
// middle of several queries for that site, we could be referencing data that is not there anymore,
// since a site update will delete all restaurants and dishes before inserting it again with new
// uuids.
// This means to possibly write some repeated SQL instead of reusing smaller functions, unless the
// smaller functions can work within a transaction.
//
// Many functions here have a lot of round trips to the DB in order to construct nested structs.
// It would be very good to cut down on those round trips, but I just don't know how to do that in
// a good way in SQL. The attempts I've made with CTEs have been bloated by all parent data for a
// dish to be repeated for every row, and I guess that's about as bad as having several round trips.
// If anyone ever reads this and have an idea of how to do this, I'd be happy to hear it!

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

pub async fn list_countries(pg: &PgPool) -> Result<LunchData> {
    let countries: Vec<Country> = sqlx::query_as(
        r#"
            select * from country
        "#,
    )
    .fetch_all(pg)
    .await?;

    let mut ld = LunchData::new();
    for c in countries {
        ld.countries.insert(c.country_id, c);
    }

    Ok(ld)
}

pub async fn list_cities(pg: &PgPool, country_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let mut country: Country = sqlx::query_as(
        r#"
            select * from country where country_id = $1
        "#,
    )
    .bind(country_id)
    .fetch_one(&mut *tx)
    .await?;

    let cities: Vec<City> = sqlx::query_as(
        r#"
            select * from city where country_id = $1
        "#,
    )
    .bind(country_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.rollback().await?;

    for c in cities {
        country.cities.insert(c.city_id, c);
    }

    let mut ld = LunchData::new();
    ld.countries.insert(country.country_id, country);

    Ok(ld)
}

pub async fn list_sites(pg: &PgPool, city_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let mut city: City = sqlx::query_as(
        r#"
            select * from city where city_id = $1
        "#,
    )
    .bind(city_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut country: Country = sqlx::query_as(
        r#"
            select * from country where country_id = $1
        "#,
    )
    .bind(city.country_id)
    .fetch_one(&mut *tx)
    .await?;

    let sites: Vec<Site> = sqlx::query_as(
        r#"
            select * from site where city_id = $1
        "#,
    )
    .bind(city_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.rollback().await?;

    for s in sites {
        city.sites.insert(s.site_id, s);
    }

    country.cities.insert(city.city_id, city);
    let mut ld = LunchData::new();
    ld.countries.insert(country.country_id, country);

    Ok(ld)
}

pub async fn list_restaurants(pg: &PgPool, site_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let mut site: Site = sqlx::query_as(
        r#"
            select * from site where site_id = $1
        "#,
    )
    .bind(site_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut city: City = sqlx::query_as(
        r#"
            select * from city where city_id = $1
        "#,
    )
    .bind(site.city_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut country: Country = sqlx::query_as(
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

    tx.rollback().await?;

    for r in restaurants {
        site.restaurants.insert(r.restaurant_id, r);
    }
    city.sites.insert(site.site_id, site);
    country.cities.insert(city.city_id, city);

    let mut ld = LunchData::new();
    ld.countries.insert(country.country_id, country);

    Ok(ld)
}

pub async fn list_dishes_for_restaurant(pg: &PgPool, restaurant_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let mut restaurant: Restaurant = sqlx::query_as(
        r#"
            select * from restaurant where restaurant_id = $1
        "#,
    )
    .bind(restaurant_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut site: Site = sqlx::query_as(
        r#"
            select * from site where site_id = $1
        "#,
    )
    .bind(restaurant.site_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut city: City = sqlx::query_as(
        r#"
            select * from city where city_id = $1
        "#,
    )
    .bind(site.city_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut country: Country = sqlx::query_as(
        r#"
            select * from country where country_id = $1
        "#,
    )
    .bind(city.country_id)
    .fetch_one(&mut *tx)
    .await?;

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
                from dish where restaurant_id = $1
                group by dish_id
        "#,
    )
    .bind(restaurant.restaurant_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.rollback().await?;

    for d in dishes {
        restaurant.dishes.insert(d.dish_id, d);
    }
    site.restaurants
        .insert(restaurant.restaurant_id, restaurant);
    city.sites.insert(site.site_id, site);
    country.cities.insert(city.city_id, city);

    let mut ld = LunchData::new();
    ld.countries.insert(country.country_id, country);

    Ok(ld)
}

pub async fn list_dishes_for_site(pg: &PgPool, site_id: Uuid) -> Result<LunchData> {
    // we use a transaction only to get a consistent view between several queries,
    // as it could happen that an update comes in in between, deleting restaurants
    // and dishes, rendering uuid fk references invalid.
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

    // since we haven't done any modifications in our queries,
    // we just rollback now that we're done selecting
    tx.rollback().await?;

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

// I'm evaluating if I should write a "list_all" function as well, to get everything in the DB into a
// LunchData instance, but that might be a bad idea if the DB gets big.
// Let's wait and see of there's any need for it at some point.

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
