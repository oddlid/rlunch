use crate::scrape::ScrapeResult;
use anyhow::{Error, Result};
use sqlx::PgPool;
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
    let mut tx = pg.begin().await?;

    // first, clear out all restaurants and their dishes, so that we don't have any stale data
    // lingering
    sqlx::query!("delete from restaurant where site_id = $1", update.site_id)
        .execute(&mut *tx)
        .await?;

    // These loops will create a lot of round trips to the DB, and probably be quite slow and
    // inefficient. So it would be good to optimize this in some way, but I just don't know how,
    // yet.
    // From looking at the logs, it seems an insert of 11 restaurants with a total of 56 dishes
    // takes about 400ms. Not a huge problem given that the DB should be updated in the background,
    // but I think there's room for plenty of improvement.
    // One possibility could be to use the full structs from models in the scrapers, so we generate
    // the uuids and relations in code rather than in the DB. Then it might be possible to use
    // UNNEST here and get fewer round trips by sort of flattening the queries.
    // TODO: have a look at postgres UNNEST()
    // https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-bind-an-array-to-a-values-clause-how-can-i-do-bulk-inserts
    for r in update.restaurants {
        let r_id = sqlx::query_scalar!(
            r#"
                insert into restaurant (site_id, restaurant_name, comment, address, url, map_url, created_at)
                values ($1, $2, $3, $4, $5, $6, $7)
                returning restaurant_id;
            "#,
            update.site_id,
            r.name,
            r.comment,
            r.address,
            r.url,
            r.map_url,
            r.parsed_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        for d in r.dishes {
            sqlx::query!(
                r#"
                    insert into dish (restaurant_id, dish_name, description, comment, tags, price)
                    values ($1, $2, $3, $4, $5, $6);
                "#,
                r_id,
                d.name,
                d.description,
                d.comment,
                &d.tags[..],
                d.price,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await.map_err(Error::from)
}
