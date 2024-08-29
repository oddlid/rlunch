use anyhow::Result;
use sqlx::PgPool;
use tracing::trace;
use uuid::Uuid;

pub async fn get_site_uuid(
    db: &PgPool,
    country_url_id: &str,
    city_url_id: &str,
    site_url_id: &str,
) -> Result<Uuid> {
    trace!(
        country_url_id,
        city_url_id,
        site_url_id,
        "Searching for site ID..."
    );
    let id = sqlx::query_scalar!(
        r#"
            with co as (
                select country_id from country where url_id = $1
            ), ci as (
                select city_id from city, co where city.country_id = co.country_id and url_id = $2
            )
            select site_id from site, ci where site.city_id = ci.city_id and url_id = $3;
        "#,
        country_url_id,
        city_url_id,
        site_url_id
    )
    .fetch_one(db)
    .await?;

    trace!(%id, "ID  found");

    Ok(id)
}
