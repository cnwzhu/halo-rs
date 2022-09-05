use anyhow::Context;
use async_session::SessionStore;
use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::http::{ApiContext, Error};
use crate::http::Result;

pub fn router() -> Router {
    Router::new()
        .route("/api/users", post(create_user))
        .route("/api/users/login", post(login_user))
        .route("/api/user", get(get_current_user))
}

pub struct AuthUser {
    pub user_id: Uuid,
}

pub struct MaybeAuthUser(pub Option<AuthUser>);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UserDO {
    pub id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub password_hash: String,
    pub email: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub expire_time: Option<time::OffsetDateTime>,
    pub create_time: time::OffsetDateTime,
    pub update_time: time::OffsetDateTime,
    pub last_login_time: Option<time::OffsetDateTime>,
    pub last_login_ip: Option<String>,
    pub mfa_type: u8,
    pub mfa_key: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct UserBody<T> {
    user: T,
}

#[derive(serde::Deserialize)]
struct NewUser {
    username: String,
    email: String,
    password: String,
}

#[derive(serde::Deserialize)]
struct LoginUser {
    email: String,
    password: String,
}

async fn create_user(
    ctx: Extension<ApiContext>,
    Json(req): Json<UserBody<NewUser>>,
) -> Result<Json<UserBody<User>>> {
    let now = time::OffsetDateTime::now_utc();
    let password_hash = hash_password(req.user.password).await?;
    let user = sqlx::query_scalar!(r#"
        INSERT INTO  "user" (username, email, password_hash, create_time, update_time, mfa_type)
        VALUES  ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, username, nickname, avatar, description
    "#,req.user.username,req.user.email,password_hash,&now,&now,0).fetch_one(&ctx.db)
        .await
        .on_constraint("user_username_key", |_| {
            Error::unprocessable_entity([("username", "username taken")])
        })
        .on_constraint("user_email_key", |_| {
            Error::unprocessable_entity([("email", "email taken")])
        })?;

    Ok(Json(UserBody {
        user
    }))
}

async fn login_user(
    ctx: Extension<ApiContext>,
    Json(req): Json<UserBody<LoginUser>>,
) -> Result<Json<UserBody<User>>> {
    let user = sqlx::query!(
        r#"
            select id, username, nickname, avatar, description
            from "user" where email = $1 or username = $1
        "#,
        req.user.email,
    )
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::unprocessable_entity([("email or username", "does not exist")]))?;

    verify_password(req.user.password, user.password_hash).await?;

    Ok(Json(UserBody {
        user,
    }))
}

async fn verify_password(password: String, password_hash: String) -> Result<()> {
    Ok(tokio::task::spawn_blocking(move || -> Result<()> {
        let hash = PasswordHash::new(&password_hash)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password)
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => Error::Unauthorized,
                _ => anyhow::anyhow!("failed to verify password hash: {}", e).into(),
            })
    })
        .await
        .context("panic in verifying password hash")??)
}

async fn hash_password(password: String) -> Result<String> {
    Ok(tokio::task::spawn_blocking(move || -> Result<String> {
        let salt = SaltString::generate(rand::thread_rng());
        Ok(
            PasswordHash::generate(Argon2::default(), password, salt.as_str())
                .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
                .to_string(),
        )
    })
        .await
        .context("panic in generating password hash")??)
}

async fn get_current_user(
    auth_user: AuthUser,
    ctx: Extension<ApiContext>,
) -> Result<Json<UserBody<User>>> {
    let user = sqlx::query!(
        r#"
            select id, username, nickname, avatar, description
            from "user" where id = $1
        "#,
        auth_user.user_id,
    )
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::unprocessable_entity([("user id", "does not exist")]))?;
    Ok(Json(UserBody {
        user,
    }))
}
