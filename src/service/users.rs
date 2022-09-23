use anyhow::Context;
use argon2::Argon2;
use axum::{
    Extension,
    Json, Router, routing::{get, post},
};
use password_hash::{PasswordHash, SaltString};

use crate::http::{ApiContext, Error};
use crate::http::Result;

pub fn router() -> Router {
    Router::new()
        .route("/api/users", post(create_user))
        .route("/api/users/login", post(login_user))
        .route("/api/user", get(get_current_user))
}

pub struct AuthUser {
    pub user_id: i64,
}

pub struct MaybeAuthUser(pub Option<AuthUser>);

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub description: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct UserDO {
    pub id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub password_hash: String,
    pub email: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub expired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_login_time: Option<chrono::DateTime<chrono::Utc>>,
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
    let now = chrono::Utc::now();
    let password_hash = hash_password(req.user.password).await?;

    let user = sqlx::query_scalar!(
        r#"
        INSERT INTO user (username, email, password_hash, created_at, updated_at, mfa_type)
        VALUES  (?, ?, ?, ?, ?, ?)
        "#,req.user.username,req.user.email,password_hash,now,now, 0u8)
        .fetch_one(&ctx.db)
        .await?;
    todo!()
}

async fn login_user(
    ctx: Extension<ApiContext>,
    Json(req): Json<UserBody<LoginUser>>,
) -> Result<Json<UserBody<User>>> {
    let user = sqlx::query!(
        r#"
            select id, username, nickname, avatar, description, password_hash
            from user where email = $1 or username = $1
        "#,
        req.user.email
    )
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::unprocessable_entity([(
            "email or username",
            "does not exist",
        )]))?;

    verify_password(req.user.password, user.password_hash).await?;
    todo!()
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
            from user where id = $1
        "#,
        auth_user.user_id,
    )
        .fetch_optional(&ctx.db)
        .await?
        .ok_or(Error::unprocessable_entity([("user id", "does not exist")]))?;
    todo!()
}
