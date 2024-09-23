use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use tracing::error;

pub mod api;
pub mod html;

#[derive(Debug, Clone)]
pub struct ApiContext {
    pub db: PgPool,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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
