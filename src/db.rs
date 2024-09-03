use anyhow::Result;
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

pub async fn get_site_uuid(db: &PgPool, key: SiteKey<'_>) -> Result<Uuid> {
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
    .fetch_one(db)
    .await?;

    trace!(%id, "ID  found");

    Ok(id)
}
