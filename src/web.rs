use crate::db;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

pub mod api;
pub mod html;

#[derive(Debug, Clone)]
pub struct ApiContext {
    pub db: PgPool,
}

impl ApiContext {
    pub async fn get_tx(&self) -> Result<db::Transaction<'_>> {
        self.db.begin().await.map_err(Error::from)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum ListQueryLevel {
    Empty,
    Country,
    City,
    Site,
    Restaurant,
    // Dish,
}

#[serde_as]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ListQuery {
    #[serde_as(as = "NoneAsEmptyString")]
    pub country: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub city: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub site: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub restaurant: Option<String>,
}

impl ListQuery {
    pub fn level(&self) -> ListQueryLevel {
        if self.country.is_some()
            && self.city.is_some()
            && self.site.is_some()
            && self.restaurant.is_some()
        {
            return ListQueryLevel::Restaurant;
        } else if self.country.is_some() && self.city.is_some() && self.site.is_some() {
            return ListQueryLevel::Site;
        } else if self.country.is_some() && self.city.is_some() {
            return ListQueryLevel::City;
        } else if self.country.is_some() {
            return ListQueryLevel::Country;
        }
        ListQueryLevel::Empty
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// 404 Not Found
    #[error("request path not found")]
    NotFound,
    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),
    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Sqlx(ref e) => {
                error!(err = %e, "SQLx error");
            }
            Self::Anyhow(ref e) => {
                error!(err = %e, "Internal error");
            }
            _ => (),
        }
        (self.status_code(), self.to_string()).into_response()
    }
}

fn check_id(id: Uuid) -> Result<()> {
    if id.is_nil() {
        return Err(Error::NotFound);
    }
    Ok(())
}
