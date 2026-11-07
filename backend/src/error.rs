use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("请先登入")]
    Unauthorized,
    #[error("{0}")]
    Forbidden(String),
    #[error("找不到资料")]
    NotFound,
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::TooManyRequests(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            AppError::Db(sqlx::Error::RowNotFound) => (StatusCode::NOT_FOUND, "找不到资料".into()),
            AppError::Db(e) => {
                tracing::error!("db error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "系统错误，请稍后再试".into(),
                )
            }
            AppError::Other(e) => {
                tracing::error!("error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "系统错误，请稍后再试".into(),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
