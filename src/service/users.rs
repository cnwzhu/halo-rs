use axum::{
    routing::{get, post},
    Extension, Json, Router,
};
use uuid::Uuid;

use crate::http::ApiContext;
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
    pub name: String,
    pub email: String,
    pub password_hash: String,
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
    Ok(Json(UserBody {
        user: User {
            id: 1,
            name: "".into(),
            email: "".into(),
            password_hash: "".into(),
        },
    }))
}

async fn login_user(
    ctx: Extension<ApiContext>,
    Json(req): Json<UserBody<LoginUser>>,
) -> Result<Json<UserBody<User>>> {
    todo!()
}

async fn get_current_user(
    auth_user: AuthUser,
    ctx: Extension<ApiContext>,
) -> Result<Json<UserBody<User>>> {
    todo!()
}
