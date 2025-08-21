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
    models::{City, Country, Dish, DishRows, LunchData, Restaurant, Site},
    scrape::SiteScrapeResult,
};
use anyhow::Result;
use sqlx::{Error, Executor, PgPool, Postgres};
use std::time::Instant;
use tracing::trace;
use uuid::Uuid;

pub type Transaction<'a> = sqlx::Transaction<'a, Postgres>;

enum SiteKeyLevel {
    Empty,
    Country,
    City,
    Site,
}

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

    fn level(&self) -> SiteKeyLevel {
        if !self.country_url_id.is_empty()
            && !self.city_url_id.is_empty()
            && !self.site_url_id.is_empty()
        {
            return SiteKeyLevel::Site;
        } else if !self.country_url_id.is_empty() && !self.city_url_id.is_empty() {
            return SiteKeyLevel::City;
        } else if !self.country_url_id.is_empty() {
            return SiteKeyLevel::Country;
        }
        SiteKeyLevel::Empty
    }
}

#[derive(Debug, Clone, Default, PartialEq, sqlx::FromRow)]
#[sqlx(default)]
pub struct SiteRelation {
    pub country_id: Uuid,
    pub city_id: Uuid,
    pub site_id: Uuid,
}

impl SiteRelation {
    fn empty(&self) -> bool {
        self == &Self::default()
    }
}

// this signature is taken from https://github.com/launchbadge/sqlx/issues/419
// Unfortunately it doesn't work to use the executor more than once within the same
// function, since the value is moved.
// This might be of use later, if I refactor so that each function only use the executor
// once, and I call several small functions with the same executor.
pub async fn get_site_relation<'e, E>(executor: E, key: SiteKey<'_>) -> Result<SiteRelation, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    trace!(?key, "Searching for site relation...");

    let rel: SiteRelation = match key.level() {
        SiteKeyLevel::Site => {
            sqlx::query_as(
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
            .await?
        }
        SiteKeyLevel::City => {
            sqlx::query_as(
                r#"
                    with co as (
                        select country_id from country where url_id = $1
                    )
                    select co.country_id, city_id, '00000000-0000-0000-0000-000000000000' from co, city where city.country_id = co.country_id and url_id = $2
                "#,
            )
            .bind(key.country_url_id)
            .bind(key.city_url_id)
            .fetch_one(executor)
            .await?
        }
        SiteKeyLevel::Country => {
            sqlx::query_as(
                r#"
                    select country_id, '00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000' from country where url_id = $1
                "#,
            )
            .bind(key.country_url_id)
            .fetch_one(executor)
            .await?
        }
        SiteKeyLevel::Empty => {
           SiteRelation::default()
        }
    };

    if rel.empty() {
        return Err(sqlx::Error::RowNotFound);
    }

    trace!(?rel, "Relation  found");

    Ok(rel)
}

pub async fn get_countries<'e, E>(ex: E) -> Result<Vec<Country>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from country
        "#,
    )
    .fetch_all(ex)
    .await
}

pub async fn get_country<'e, E>(ex: E, country_id: Uuid) -> Result<Country, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from country where country_id = $1
        "#,
    )
    .bind(country_id)
    .fetch_one(ex)
    .await
}

pub async fn get_city<'e, E>(ex: E, city_id: Uuid) -> Result<City, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from city where city_id = $1
        "#,
    )
    .bind(city_id)
    .fetch_one(ex)
    .await
}

pub async fn get_cities_for_country<'e, E>(ex: E, country_id: Uuid) -> Result<Vec<City>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from city where country_id = $1
        "#,
    )
    .bind(country_id)
    .fetch_all(ex)
    .await
}

pub async fn get_site<'e, E>(ex: E, site_id: Uuid) -> Result<Site, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from site where site_id = $1
        "#,
    )
    .bind(site_id)
    .fetch_one(ex)
    .await
}

pub async fn get_sites_for_city<'e, E>(ex: E, city_id: Uuid) -> Result<Vec<Site>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from site where city_id = $1
        "#,
    )
    .bind(city_id)
    .fetch_all(ex)
    .await
}

pub async fn get_restaurant<'e, E>(ex: E, restaurant_id: Uuid) -> Result<Restaurant, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from restaurant where restaurant_id = $1
        "#,
    )
    .bind(restaurant_id)
    .fetch_one(ex)
    .await
}

pub async fn get_restaurants_for_site<'e, E>(ex: E, site_id: Uuid) -> Result<Vec<Restaurant>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from restaurant where site_id = $1
        "#,
    )
    .bind(site_id)
    .fetch_all(ex)
    .await
}

pub async fn get_dish<'e, E>(ex: E, dish_id: Uuid) -> Result<Dish, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        r#"
            select * from dish where dish_id = $1
        "#,
    )
    .bind(dish_id)
    .fetch_one(ex)
    .await
}

pub async fn get_dishes_for_restaurant<'e, E>(
    ex: E,
    restaurant_id: Uuid,
) -> Result<Vec<Dish>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
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
    .bind(restaurant_id)
    .fetch_all(ex)
    .await
}

pub fn get_restaurant_ids(restaurants: &[Restaurant]) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(restaurants.len());
    for r in restaurants {
        ids.push(r.restaurant_id);
    }
    ids
}

pub async fn get_dishes_for_site<'e, E>(
    ex: E,
    restaurant_ids: Vec<Uuid>,
) -> Result<Vec<Dish>, Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
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
    .fetch_all(ex)
    .await
}

pub async fn list_countries(pg: &PgPool) -> Result<LunchData, Error> {
    // we don't need a transaction here, since we only make a single query
    Ok(LunchData::new().with_countries(get_countries(pg).await?))
}

pub async fn list_cities_for_country_by_id(
    tx: &mut Transaction<'_>,
    country_id: Uuid,
) -> Result<LunchData, Error> {
    let country = get_country(&mut **tx, country_id).await?;
    let cities = get_cities_for_country(&mut **tx, country_id).await?;

    Ok(LunchData::new().with_country(country.with_cities(cities)))
}

pub async fn list_cities_for_country_by_key(
    tx: &mut Transaction<'_>,
    key: SiteKey<'_>,
) -> Result<LunchData, Error> {
    let country_id = get_site_relation(&mut **tx, key).await?.country_id;
    list_cities_for_country_by_id(tx, country_id).await
}

pub async fn list_sites_for_city_by_id(
    tx: &mut Transaction<'_>,
    city_id: Uuid,
) -> Result<LunchData, Error> {
    let city = get_city(&mut **tx, city_id).await?;
    let country = get_country(&mut **tx, city.country_id).await?;
    let sites = get_sites_for_city(&mut **tx, city_id).await?;

    Ok(LunchData::new().with_country(country.with_city(city.with_sites(sites))))
}

pub async fn list_sites_for_city_by_key(
    tx: &mut Transaction<'_>,
    key: SiteKey<'_>,
) -> Result<LunchData, Error> {
    let city_id = get_site_relation(&mut **tx, key).await?.city_id;
    list_sites_for_city_by_id(tx, city_id).await
}

pub async fn list_all_sites(tx: &mut Transaction<'_>) -> Result<LunchData, Error> {
    let sites: Vec<Site> = sqlx::query_as(
        r#"
            select * from site
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;

    let cities: Vec<City> = sqlx::query_as(
        r#"
            select * from city
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;

    let countries: Vec<Country> = sqlx::query_as(
        r#"
            select * from country
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(LunchData::build(
        countries,
        cities,
        sites,
        Vec::new(),
        Vec::new(),
    ))
}

pub async fn list_restaurants_for_site_by_id(
    tx: &mut Transaction<'_>,
    site_id: Uuid,
) -> Result<LunchData, Error> {
    let site = get_site(&mut **tx, site_id).await?;
    let city = get_city(&mut **tx, site.city_id).await?;
    let country = get_country(&mut **tx, city.country_id).await?;
    let restaurants = get_restaurants_for_site(&mut **tx, site_id).await?;

    Ok(LunchData::new()
        .with_country(country.with_city(city.with_site(site.with_restaurants(restaurants)))))
}

pub async fn list_restaurants_for_site_by_key(
    tx: &mut Transaction<'_>,
    key: SiteKey<'_>,
) -> Result<LunchData, Error> {
    let site_id = get_site_relation(&mut **tx, key).await?.site_id;
    list_restaurants_for_site_by_id(tx, site_id).await
}

pub async fn list_dishes_for_restaurant_by_id(
    tx: &mut Transaction<'_>,
    restaurant_id: Uuid,
) -> Result<LunchData, Error> {
    let restaurant = get_restaurant(&mut **tx, restaurant_id).await?;
    let site = get_site(&mut **tx, restaurant.site_id).await?;
    let city = get_city(&mut **tx, site.city_id).await?;
    let country = get_country(&mut **tx, city.country_id).await?;
    let dishes = get_dishes_for_restaurant(&mut **tx, restaurant_id).await?;

    Ok(LunchData::new().with_country(
        country.with_city(city.with_site(site.with_restaurant(restaurant.with_dishes(dishes)))),
    ))
}

// We skip this implementation for now, since there's currently no support for levels below site in
// SiteKey or SiteRelation
// pub async fn list_dishes_for_restaurant_by_key(
//     tx: &mut Transaction<'_>,
//     key: SiteKey<'_>,
// ) -> Result<LunchData> {
// }

pub async fn list_dishes_for_site_by_id(
    tx: &mut Transaction<'_>,
    site_id: Uuid,
) -> Result<LunchData, Error> {
    let site = get_site(&mut **tx, site_id).await?;
    let city = get_city(&mut **tx, site.city_id).await?;
    let country = get_country(&mut **tx, city.country_id).await?;
    let restaurants = get_restaurants_for_site(&mut **tx, site_id).await?;
    let dishes = get_dishes_for_site(&mut **tx, get_restaurant_ids(&restaurants)).await?;

    Ok(LunchData::new().with_country(
        country.with_city(city.with_site(site.with_restaurants(restaurants).with_dishes(dishes))),
    ))
}

pub async fn list_dishes_for_site_by_key(
    tx: &mut Transaction<'_>,
    key: SiteKey<'_>,
) -> Result<LunchData, Error> {
    let site_id = get_site_relation(&mut **tx, key).await?.site_id;
    list_dishes_for_site_by_id(tx, site_id).await
}

// I'm evaluating if I should write a "list_all" function as well, to get everything in the DB into a
// LunchData instance, but that might be a bad idea if the DB gets big.
// Let's wait and see of there's any need for it at some point.

// Keeping this for reference, due to the quirky queries
// pub async fn update_site(pg: &PgPool, update: SiteScrapeResult) -> Result<(), Error> {
//     trace!(site_id = %update.site_id, "Adding {} restaurants and {} dishes to DB", update.num_restaurants(), update.num_dishes());
//
//     let start = Instant::now();
//     // convert to format suitable for use with unnest
//     let rs = RestaurantRows::from(update.restaurants);
//     let duration = start.elapsed();
//     trace!("Conversion to DB format done in {:?}", duration);
//
//     // we need a transaction to ensure these operations are done atomically
//     let mut tx = pg.begin().await?;
//
//     let start = Instant::now();
//     // first, clear out all restaurants and their dishes, so that we don't have any stale data
//     // lingering. We have "on delete cascade" for dishes, so we just need to delete the parent
//     // restaurants to get rid of all.
//     sqlx::query!("delete from restaurant where site_id = $1", update.site_id)
//         .execute(&mut *tx)
//         .await?;
//
//     // insert all restaurants
//     sqlx::query!(
//         r#"
//             insert into restaurant (site_id, restaurant_id, restaurant_name, comment, address, url, map_url, created_at)
//             select * from unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::timestamptz[])
//         "#,
//         &rs.site_ids[..],
//         &rs.restaurant_ids[..],
//         &rs.names[..],
//         &rs.comments as &[Option<String>],
//         &rs.addresses as &[Option<String>],
//         &rs.urls as &[Option<String>],
//         &rs.map_urls as &[Option<String>],
//         &rs.parsed_ats[..],
//     )
//     .execute(&mut *tx)
//     .await?;
//
//     // insert all dishes
//     sqlx::query!(
//         r#"
//             insert into dish (restaurant_id, dish_id, dish_name, description, comment, price, tags)
//             select * from unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::real[], $7::text[])
//         "#,
//         &rs.dishes.restaurant_ids[..],
//         &rs.dishes.dish_ids[..],
//         &rs.dishes.names[..],
//         &rs.dishes.descriptions as &[Option<String>],
//         &rs.dishes.comments as &[Option<String>],
//         &rs.dishes.prices[..],
//         &rs.dishes.tags[..],
//     ).execute(&mut *tx).await?;
//     let duration = start.elapsed();
//
//     trace!("DB update done in {:?}", duration);
//
//     tx.commit().await
// }

// Add/replace one restaurant at a time, to allow for leaving manually inserted restaurants
// untouched, and not needing to scrape stuff that rarely changes
pub async fn update_restaurants(pg: &PgPool, update: SiteScrapeResult) -> Result<(), Error> {
    trace!(site_id = %update.site_id, "Adding {} restaurants and {} dishes to DB", update.num_restaurants(), update.num_dishes());

    // we need a transaction to ensure these operations are done atomically
    let mut tx = pg.begin().await?;

    let start = Instant::now();

    for restaurant in update.restaurants.into_iter() {
        sqlx::query!(
            "delete from restaurant where site_id = $1 and restaurant_name = $2",
            restaurant.site_id,
            restaurant.name
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
        "insert into restaurant (site_id, restaurant_id, restaurant_name, comment, address, url, map_url, created_at) values($1, $2, $3, $4, $5, $6, $7, $8)",
        restaurant.site_id,
        restaurant.restaurant_id,
        restaurant.name,
        restaurant.comment,
        restaurant.address,
        restaurant.url,
        restaurant.map_url,
        restaurant.parsed_at,
    ).execute(&mut *tx).await?;

        let drs = DishRows::from(restaurant.dishes);
        // insert all dishes
        sqlx::query!(
        r#"
            insert into dish (restaurant_id, dish_id, dish_name, description, comment, price, tags)
            select * from unnest($1::uuid[], $2::uuid[], $3::text[], $4::text[], $5::text[], $6::real[], $7::text[])
        "#,
        &drs.restaurant_ids[..],
        &drs.dish_ids[..],
        &drs.names[..],
        &drs.descriptions as &[Option<String>],
        &drs.comments as &[Option<String>],
        &drs.prices[..],
        &drs.tags[..],
    ).execute(&mut *tx).await?;
    }

    let duration = start.elapsed();
    trace!("DB update done in {:?}", duration);

    tx.commit().await
}
