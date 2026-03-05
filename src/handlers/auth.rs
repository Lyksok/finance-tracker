use crate::AppState;
use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;

use crate::models::User;
use crate::templates::{LoginTemplate, RegisterTemplate};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn render_login() -> impl IntoResponse {
    Html(
        LoginTemplate {
            logged_in: false,
            error: None,
        }
        .render()
        .unwrap(),
    )
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = User::find_by_username(&state.pool, &form.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let valid = match user {
        Some(ref u) => {
            let parsed_hash = PasswordHash::new(&u.password_hash)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Argon2::default()
                .verify_password(form.password.as_bytes(), &parsed_hash)
                .is_ok()
        }
        None => false,
    };

    if valid {
        let u = user.unwrap();
        session.insert("user_id", u.id).await.map_err(|e| {
            tracing::error!("Failed to save session for user {}: {:?}", u.username, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        tracing::info!("User {} logged in successfully", u.username);
        return Ok(Redirect::to("/").into_response());
    }

    tracing::warn!("Failed login attempt for username: {}", form.username);

    let tpl = LoginTemplate {
        logged_in: false,
        error: Some("Invalid username or password".to_string()),
    };
    Ok(Html(
        tpl.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response())
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub password: String,
}

pub async fn render_register() -> impl IntoResponse {
    Html(
        RegisterTemplate {
            logged_in: false,
            error: None,
        }
        .render()
        .unwrap(),
    )
}

pub async fn register(
    State(state): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(form.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let res = User::create(&state.pool, &form.username, &hash).await;

    match res {
        Ok(_) => {
            tracing::info!("New user registered successfully: {}", form.username);
            Ok(Redirect::to("/login").into_response())
        }
        Err(e) => {
            tracing::warn!(
                "Failed registration attempt (username likely exists): {} - {:?}",
                form.username,
                e
            );
            let tpl = RegisterTemplate {
                logged_in: false,
                error: Some("Username already exists".to_string()),
            };
            Ok(Html(
                tpl.render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            )
            .into_response())
        }
    }
}

pub async fn logout(session: Session) -> Result<impl IntoResponse, StatusCode> {
    session.delete().await.map_err(|e| {
        tracing::error!("Failed to delete session on logout: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("A user logged out successfully");
    Ok(Redirect::to("/login").into_response())
}
