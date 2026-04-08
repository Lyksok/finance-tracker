use crate::AppState;
use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::models::User;
use crate::templates::ProfileTemplate;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[derive(Deserialize)]
pub struct UpdateProfileForm {
    pub current_password: String,
    pub new_password: String,
}

pub async fn render_profile(auth_user: AuthUser) -> impl IntoResponse {
    let tpl = ProfileTemplate {
        logged_in: true,
        username: auth_user.0.username,
        error: None,
        success: None,
    };
    Html(tpl.render().unwrap())
}

pub async fn update_profile(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<UpdateProfileForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let parsed_hash = PasswordHash::new(&auth_user.0.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let valid = Argon2::default()
        .verify_password(form.current_password.as_bytes(), &parsed_hash)
        .is_ok();

    if !valid {
        tracing::warn!(
            "User {} failed to update password: incorrect current password",
            auth_user.0.username
        );
        let tpl = ProfileTemplate {
            logged_in: true,
            username: auth_user.0.username.clone(),
            error: Some("Incorrect current password".to_string()),
            success: None,
        };
        return Ok(Html(
            tpl.render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .into_response());
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(form.new_password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    User::update_password(&state.pool, auth_user.0.id, &hash)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to update password hashing for user {}: {:?}",
                auth_user.0.username,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "User {} updated their password successfully",
        auth_user.0.username
    );

    let tpl = ProfileTemplate {
        logged_in: true,
        username: auth_user.0.username,
        error: None,
        success: Some("Password updated successfully".to_string()),
    };
    Ok(Html(
        tpl.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response())
}
