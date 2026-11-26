use std::cell::Cell;
use std::future::Future;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use crate::i18n::{keys::ErrorKey, Lang};

tokio::task_local! {
    static REQUEST_LANG: Cell<Lang>;
}

/// Bind a request-scoped fallback language for errors that arise outside handlers
/// with explicit `CurrentUser` context (e.g. `sqlx::Error::RowNotFound`, 500s).
pub async fn scope_lang<F: Future>(lang: Lang, fut: F) -> F::Output {
    REQUEST_LANG.scope(Cell::new(lang), fut).await
}

pub fn set_request_lang(lang: Lang) {
    let _ = REQUEST_LANG.try_with(|c| c.set(lang));
}

pub fn current_request_lang() -> Lang {
    REQUEST_LANG.try_with(|c| c.get()).unwrap_or(Lang::Zh)
}

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
    #[error("locked")]
    Locked { lang: Lang, key: ErrorKey },
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

pub fn bad_request(lang: Lang, key: ErrorKey) -> AppError {
    AppError::BadRequest { lang, key }
}

pub use bad_request as bad;

pub fn forbidden(lang: Lang, key: ErrorKey) -> AppError {
    AppError::Forbidden { lang, key }
}

pub fn conflict(lang: Lang, key: ErrorKey) -> AppError {
    AppError::Conflict { lang, key }
}

pub fn too_many_requests(lang: Lang, key: ErrorKey) -> AppError {
    AppError::TooManyRequests { lang, key }
}

pub fn locked(lang: Lang, key: ErrorKey) -> AppError {
    AppError::Locked { lang, key }
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
            AppError::Locked { lang, key } => (StatusCode::LOCKED, *lang, key),
            // sqlx `?`-propagated RowNotFound arrives here without user context;
            // render using the request-scoped locale (Accept-Language or user's saved lang).
            AppError::Db(sqlx::Error::RowNotFound) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(error_body(current_request_lang(), &ErrorKey::NotFound)),
                )
                    .into_response();
            }
            AppError::Db(sqlx::Error::Database(db_err)) => {
                tracing::error!(
                    code = ?db_err.code(),
                    constraint = ?db_err.constraint(),
                    table = ?db_err.table(),
                    "database error"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_body(current_request_lang(), &ErrorKey::ServerError)),
                )
                    .into_response();
            }
            AppError::Db(e) => {
                tracing::error!(error = %e, "db error");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_body(current_request_lang(), &ErrorKey::ServerError)),
                )
                    .into_response();
            }
            AppError::Other(e) => {
                tracing::error!("error: {e:?}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_body(current_request_lang(), &ErrorKey::ServerError)),
                )
                    .into_response();
            }
        };
        (status, Json(error_body(lang, key))).into_response()
    }
}
