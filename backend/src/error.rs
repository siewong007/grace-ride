use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use crate::i18n::{keys::ErrorKey, Lang};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request")]
    BadRequest { lang: Lang, key: ErrorKey },
    #[error("unauthorized")]
    Unauthorized { lang: Lang, key: ErrorKey },
    #[error("forbidden")]
    Forbidden { lang: Lang, key: ErrorKey },
    #[error("not found")]
    NotFound { lang: Lang, key: ErrorKey },
    #[error("conflict")]
    Conflict { lang: Lang, key: ErrorKey },
    #[error("too many requests")]
    TooManyRequests { lang: Lang, key: ErrorKey },
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

pub fn bad(lang: Lang, key: ErrorKey) -> AppError {
    AppError::BadRequest { lang, key }
}

pub fn forbidden(lang: Lang, key: ErrorKey) -> AppError {
    AppError::Forbidden { lang, key }
}

pub fn conflict(lang: Lang, key: ErrorKey) -> AppError {
    AppError::Conflict { lang, key }
}

pub fn too_many_requests(lang: Lang, key: ErrorKey) -> AppError {
    AppError::TooManyRequests { lang, key }
}

/// 401 in the requester's language: Accept-Language for unauthenticated
/// requests, or the saved `users.lang` once a session exists.
pub fn unauthorized(lang: Lang) -> AppError {
    AppError::Unauthorized {
        lang,
        key: ErrorKey::Unauthorized,
    }
}

/// 404 in the given language.
pub fn not_found(lang: Lang) -> AppError {
    AppError::NotFound {
        lang,
        key: ErrorKey::NotFound,
    }
}

fn error_body(lang: Lang, key: &ErrorKey) -> Value {
    let mut body = json!({
        "error": key.render(lang),
        "code": key.code(),
    });
    if let Some(params) = key.params() {
        body["params"] = params;
    }
    body
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, lang, key) = match &self {
            AppError::BadRequest { lang, key } => (StatusCode::BAD_REQUEST, *lang, key),
            AppError::Unauthorized { lang, key } => (StatusCode::UNAUTHORIZED, *lang, key),
            AppError::Forbidden { lang, key } => (StatusCode::FORBIDDEN, *lang, key),
            AppError::NotFound { lang, key } => (StatusCode::NOT_FOUND, *lang, key),
            AppError::Conflict { lang, key } => (StatusCode::CONFLICT, *lang, key),
            AppError::TooManyRequests { lang, key } => (StatusCode::TOO_MANY_REQUESTS, *lang, key),
            // sqlx `?`-propagated RowNotFound arrives here without user context;
            // render the app default locale (zh) for those infrastructure paths.
            AppError::Db(sqlx::Error::RowNotFound) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(error_body(Lang::Zh, &ErrorKey::NotFound)),
                )
                    .into_response();
            }
            AppError::Db(e) => {
                tracing::error!("db error: {e:?}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_body(Lang::Zh, &ErrorKey::ServerError)),
                )
                    .into_response();
            }
            AppError::Other(e) => {
                tracing::error!("error: {e:?}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_body(Lang::Zh, &ErrorKey::ServerError)),
                )
                    .into_response();
            }
        };
        (status, Json(error_body(lang, key))).into_response()
    }
}
