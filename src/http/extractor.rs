use crate::http::error::Error;
use axum::body::Body;
use axum::extract::{Extension, FromRequest, RequestParts};

use crate::http::ApiContext;
use async_trait::async_trait;
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderValue;

use crate::service::users::AuthUser;
use crate::service::users::MaybeAuthUser;
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha384;

use hmac::{Hmac, Mac};

const SCHEME_PREFIX: &str = "Token ";

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthUserClaims {
    user_id: i64,
    /// Standard JWT `exp` claim.
    exp: i64,
}

impl AuthUser {
    pub(in crate::http) fn to_jwt(&self, ctx: &ApiContext) -> String {
        let hmac = Hmac::<Sha384>::new_from_slice(ctx.config.hmac_key.as_bytes())
            .expect("HMAC-SHA-384 can accept any key length");

        AuthUserClaims {
            user_id: self.user_id,
            exp: (chrono::Utc::now() +  chrono::Duration::weeks(2)).timestamp(),
        }
        .sign_with_key(&hmac)
        .expect("HMAC signing should be infallible")
    }

    /// Attempt to parse `Self` from an `Authorization` header.
    fn from_authorization(ctx: &ApiContext, auth_header: &HeaderValue) -> Result<Self, Error> {
        let auth_header = auth_header.to_str().map_err(|_| {
            log::debug!("Authorization header is not UTF-8");
            Error::Unauthorized
        })?;

        if !auth_header.starts_with(SCHEME_PREFIX) {
            log::debug!(
                "Authorization header is using the wrong scheme: {:?}",
                auth_header
            );
            return Err(Error::Unauthorized);
        }

        let token = &auth_header[SCHEME_PREFIX.len()..];

        let jwt =
            jwt::Token::<jwt::Header, AuthUserClaims, _>::parse_unverified(token).map_err(|e| {
                log::debug!(
                    "failed to parse Authorization header {:?}: {}",
                    auth_header,
                    e
                );
                Error::Unauthorized
            })?;

        let hmac = Hmac::<Sha384>::new_from_slice(ctx.config.hmac_key.as_bytes())
            .expect("HMAC-SHA-384 can accept any key length");

        let jwt = jwt.verify_with_key(&hmac).map_err(|e| {
            log::debug!("JWT failed to verify: {}", e);
            Error::Unauthorized
        })?;

        let (_header, claims) = jwt.into();
        if claims.exp < chrono::Utc::now().timestamp() {
            log::debug!("token expired");
            return Err(Error::Unauthorized);
        }

        Ok(Self {
            user_id: claims.user_id,
        })
    }
}

impl MaybeAuthUser {
    /// If this is `Self(Some(AuthUser))`, return `AuthUser::user_id`
    pub fn user_id(&self) -> Option<i64> {
        self.0.as_ref().map(|auth_user| auth_user.user_id)
    }
}

#[async_trait]
impl FromRequest<Body> for AuthUser {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let ctx: Extension<ApiContext> = Extension::from_request(req)
            .await
            .expect("BUG: ApiContext was not added as an extension");

        // Get the value of the `Authorization` header, if it was sent at all.
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .ok_or(Error::Unauthorized)?;

        Self::from_authorization(&ctx, auth_header)
    }
}

#[async_trait]
impl FromRequest<Body> for MaybeAuthUser {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let ctx: Extension<ApiContext> = Extension::from_request(req)
            .await
            .expect("BUG: ApiContext was not added as an extension");

        Ok(Self(
            // Get the value of the `Authorization` header, if it was sent at all.
            req.headers()
                .get(AUTHORIZATION)
                .and_then(|auth_header| Some(AuthUser::from_authorization(&ctx, auth_header)))
                .transpose()?,
        ))
    }
}
