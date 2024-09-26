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
// use compact_str::CompactString;
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

// #[derive(Debug, Clone, Default, PartialEq)]
// struct PathElement {
//     id: Option<Uuid>,
//     path: Option<CompactString>,
// }
//
// impl PathElement {
//     // fn set_id(&mut self, id: Uuid) {
//     //     self.id = Some(id);
//     // }
//
//     fn set_path(&mut self, path: Option<&str>) {
//         if let Some(p) = path {
//             self.path = Some(p.into())
//         }
//     }
//
//     fn has_path(&self) -> bool {
//         self.path.as_ref().is_some_and(|s| !s.is_empty())
//     }
// }
//
// #[derive(Debug, Clone, Default, PartialEq)]
// pub struct DBPath {
//     country: PathElement,
//     city: PathElement,
//     site: PathElement,
//     restaurant: PathElement,
//     dish: PathElement,
// }
//
// impl DBPath {
//     pub fn new() -> Self {
//         Self {
//             ..Default::default()
//         }
//     }
//
//     pub fn with_country(mut self, country: Option<&str>) -> Self {
//         self.country.set_path(country);
//         self
//     }
//
//     pub fn with_city(mut self, path: Option<&str>) -> Self {
//         self.city.set_path(path);
//         self
//     }
//
//     pub fn with_site(mut self, path: Option<&str>) -> Self {
//         self.site.set_path(path);
//         self
//     }
//
//     pub fn with_restaurant(mut self, path: Option<&str>) -> Self {
//         self.restaurant.set_path(path);
//         self
//     }
//
//     pub fn with_dish(mut self, path: Option<&str>) -> Self {
//         self.dish.set_path(path);
//         self
//     }
//
//     /// `path` should be a string separated by /, with no leading slash.
//     /// The expected format is "country/city/site/restaurant/dish", where
//     /// country, city and site should be the url_id field in the DB, and
//     /// restaurant and dish should be the respective names.
//     /// The parts will be parsed in that order, as found.
//     /// All parts are optional, but at least the first part for country should be given,
//     /// for the result to be of any use.
//     pub fn parse(path: &str) -> Self {
//         let mut parts = path.split('/').collect::<Vec<&str>>();
//         parts.reverse();
//         Self::new()
//             .with_country(parts.pop())
//             .with_city(parts.pop())
//             .with_site(parts.pop())
//             .with_restaurant(parts.pop())
//             .with_dish(parts.pop())
//     }
//
//     fn has_country(&self) -> bool {
//         self.country.has_path()
//     }
//
//     fn has_city(&self) -> bool {
//         self.city.has_path()
//     }
//
//     fn has_site(&self) -> bool {
//         self.site.has_path()
//     }
//
//     fn has_restaurant(&self) -> bool {
//         self.restaurant.has_path()
//     }
//
//     fn has_dish(&self) -> bool {
//         self.dish.has_path()
//     }
//
//     fn is_empty(&self) -> bool {
//         !self.has_country()
//             && !self.has_city()
//             && !self.has_site()
//             && !self.has_restaurant()
//             && !self.has_dish()
//     }
//
//     fn is_full(&self) -> bool {
//         self.has_country()
//             && self.has_city()
//             && self.has_site()
//             && self.has_restaurant()
//             && self.has_dish()
//     }
//
//     fn has_up_to_site(&self) -> bool {
//         self.has_country() && self.has_city() && self.has_site()
//     }
// }

// this signature is taken from https://github.com/launchbadge/sqlx/issues/419
// Unfortunately it doesn't work to use the executor more than once within the same
// function, since the value is moved.
// So in reality, this has no value other than saving it as an example of a parameter
// that can accept both &PgPool or &mut *Transaction.
// This might be of use later though, if I refactor so that each function only use the executor
// once, and I call several small functions with the same executor.
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

    Ok(LunchData::new().with_countries(countries))
}

// template
pub async fn get_x<'e, E>(_ex: E) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    Ok(())
}

pub async fn get_country<'e, E>(ex: E, country_id: Uuid) -> Result<Country>
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
    .map_err(Error::from)
}

pub async fn get_city<'e, E>(ex: E, city_id: Uuid) -> Result<City>
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
    .map_err(Error::from)
}

pub async fn get_cities<'e, E>(ex: E, country_id: Uuid) -> Result<Vec<City>>
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
    .map_err(Error::from)
}

pub async fn get_site<'e, E>(ex: E, site_id: Uuid) -> Result<Site>
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
    .map_err(Error::from)
}

pub async fn get_sites<'e, E>(ex: E, city_id: Uuid) -> Result<Vec<Site>>
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
    .map_err(Error::from)
}

pub async fn get_restaurant<'e, E>(ex: E, restaurant_id: Uuid) -> Result<Restaurant>
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
    .map_err(Error::from)
}

pub async fn get_restaurants<'e, E>(ex: E, site_id: Uuid) -> Result<Vec<Restaurant>>
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
    .map_err(Error::from)
}

pub async fn get_dish<'e, E>(ex: E, dish_id: Uuid) -> Result<Dish>
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
    .map_err(Error::from)
}

pub async fn get_dishes_for_restaurant<'e, E>(ex: E, restaurant_id: Uuid) -> Result<Vec<Dish>>
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
    .map_err(Error::from)
}

pub fn get_restaurant_ids(restaurants: &[Restaurant]) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(restaurants.len());
    for r in restaurants {
        ids.push(r.restaurant_id);
    }
    ids
}

pub async fn get_dishes_for_site<'e, E>(ex: E, restaurant_ids: Vec<Uuid>) -> Result<Vec<Dish>>
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
    .map_err(Error::from)
}

pub async fn list_cities(pg: &PgPool, country_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let country = get_country(&mut *tx, country_id).await?;
    let cities = get_cities(&mut *tx, country_id).await?;

    tx.rollback().await?;

    Ok(LunchData::new().with_country(country.with_cities(cities)))
}

pub async fn list_sites(pg: &PgPool, city_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let city = get_city(&mut *tx, city_id).await?;
    let country = get_country(&mut *tx, city.country_id).await?;
    let sites = get_sites(&mut *tx, city_id).await?;

    tx.rollback().await?;

    Ok(LunchData::new().with_country(country.with_city(city.with_sites(sites))))
}

pub async fn list_restaurants(pg: &PgPool, site_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let site = get_site(&mut *tx, site_id).await?;
    let city = get_city(&mut *tx, site.city_id).await?;
    let country = get_country(&mut *tx, city.country_id).await?;
    let restaurants = get_restaurants(&mut *tx, site_id).await?;

    tx.rollback().await?;

    Ok(LunchData::new()
        .with_country(country.with_city(city.with_site(site.with_restaurants(restaurants)))))
}

pub async fn list_dishes_for_restaurant(pg: &PgPool, restaurant_id: Uuid) -> Result<LunchData> {
    let mut tx = pg.begin().await?;

    let restaurant = get_restaurant(&mut *tx, restaurant_id).await?;
    let site = get_site(&mut *tx, restaurant.site_id).await?;
    let city = get_city(&mut *tx, site.city_id).await?;
    let country = get_country(&mut *tx, city.country_id).await?;
    let dishes = get_dishes_for_restaurant(&mut *tx, restaurant_id).await?;

    tx.rollback().await?;

    Ok(LunchData::new().with_country(
        country.with_city(city.with_site(site.with_restaurant(restaurant.with_dishes(dishes)))),
    ))
}

pub async fn list_dishes_for_site(pg: &PgPool, site_id: Uuid) -> Result<LunchData> {
    // we use a transaction only to get a consistent view between several queries,
    // as it could happen that an update comes in in between, deleting restaurants
    // and dishes, rendering uuid fk references invalid.
    let mut tx = pg.begin().await?;

    let site = get_site(&mut *tx, site_id).await?;
    let city = get_city(&mut *tx, site.city_id).await?;
    let country = get_country(&mut *tx, city.country_id).await?;
    let restaurants = get_restaurants(&mut *tx, site_id).await?;
    let dishes = get_dishes_for_site(&mut *tx, get_restaurant_ids(&restaurants)).await?;

    // since we haven't done any modifications in our queries,
    // we just rollback now that we're done selecting
    tx.rollback().await?;

    Ok(LunchData::new().with_country(
        country.with_city(city.with_site(site.with_restaurants(restaurants).with_dishes(dishes))),
    ))
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
