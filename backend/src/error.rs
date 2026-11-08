use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::util::{t, Lang};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    TooManyRequests(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

pub fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}
pub fn forbidden(msg: impl Into<String>) -> AppError {
    AppError::Forbidden(msg.into())
}
pub fn conflict(msg: impl Into<String>) -> AppError {
    AppError::Conflict(msg.into())
}

/// 401 in the requester's language: Accept-Language for unauthenticated
/// requests, or the saved `users.lang` once a session exists.
pub fn unauthorized(lang: Lang) -> AppError {
    AppError::Unauthorized(t(lang, "请先登入", "Please log in first").to_string())
}

/// 404 in the given language.
pub fn not_found(lang: Lang) -> AppError {
    AppError::NotFound(t(lang, "找不到资料", "Not found").to_string())
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::TooManyRequests(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            // sqlx `?`-propagated RowNotFound arrives here without user context;
            // render the app default locale (zh) for those infrastructure paths.
            AppError::Db(sqlx::Error::RowNotFound) => (
                StatusCode::NOT_FOUND,
                t(Lang::Zh, "找不到资料", "Not found").to_string(),
            ),
            AppError::Db(e) => {
                tracing::error!("db error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    t(Lang::Zh, "系统错误，请稍后再试", "Something went wrong, please try again later")
                        .to_string(),
                )
            }
            AppError::Other(e) => {
                tracing::error!("error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    t(Lang::Zh, "系统错误，请稍后再试", "Something went wrong, please try again later")
                        .to_string(),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
