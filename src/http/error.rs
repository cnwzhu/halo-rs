use std::borrow::Cow;
use std::collections::HashMap;

use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sqlx::error::DatabaseError;

/// 异常类型
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// 返回 `401 Unauthorized`
    #[error("authentication required")]
    Unauthorized,

    /// 返回 `403 Forbidden`
    #[error("user may not perform that action")]
    Forbidden,

    /// 返回 `404 Not Found`
    #[error("request path not found")]
    NotFound,

    /// 返回 `422 Unprocessable Entity`
    #[error("error in the request body")]
    UnprocessableEntity {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    /// 返回 `500 Internal Server Error` on a `sqlx::Error`.
    #[error("数据库错误")]
    Sqlx(#[from] sqlx::Error),

    /// 返回 `500 Internal Server Error` on a `anyhow::Error`.
    #[error("服务器异常")]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    pub fn unprocessable_entity<K, V>(errors: impl IntoIterator<Item=(K, V)>) -> Self
        where
            K: Into<Cow<'static, str>>,
            V: Into<Cow<'static, str>>,
    {
        let mut error_map = HashMap::new();

        for (key, val) in errors {
            error_map
                .entry(key.into())
                .or_insert_with(Vec::new)
                .push(val.into());
        }

        Self::UnprocessableEntity { errors: error_map }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Sqlx(_) | Self::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// error 转换成 response
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::UnprocessableEntity { errors } => {
                #[derive(serde::Serialize)]
                struct Errors {
                    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
                }

                return (StatusCode::UNPROCESSABLE_ENTITY, Json(Errors { errors })).into_response();
            }
            Self::Unauthorized => {
                return (
                    self.status_code(),
                    [(WWW_AUTHENTICATE, HeaderValue::from_static("Token"))]
                        .into_iter()
                        .collect::<HeaderMap>(),
                    self.to_string(),
                )
                    .into_response();
            }

            Self::Sqlx(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("SQLx error: {:?}", e);
            }

            Self::Anyhow(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("Generic error: {:?}", e);
            }

            // 其他异常忽略
            _ => (),
        }

        (self.status_code(), self.to_string()).into_response()
    }
}


pub trait ResultExt<T> {
    fn on_constraint(self,
                     name: &str,
                     map_err: impl FnOnce(Box<dyn DatabaseError>) -> Error) -> Result<T, Error>;
}

impl<T, E> ResultExt<T> for Result<T, E>
    where
        E: Into<Error>,
{
    fn on_constraint(
        self,
        name: &str,
        map_err: impl FnOnce(Box<dyn DatabaseError>) -> Error) -> Result<T, Error> {
        self.map_err(|e| match e.into() {
            Error::Sqlx(sqlx::Error::Database(dbe)) if dbe.constraint() == Some(name) => {
                map_err(dbe)
            }
            e => e,
        })
    }
}

