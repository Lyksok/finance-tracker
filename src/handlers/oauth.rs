use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{models::OAuthAccount, AppState};

#[derive(Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    login: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUser {
    id: String,
    email: Option<String>,
}

fn oauth_client(provider: &str) -> Option<BasicClient> {
    let client_id = std::env::var(format!("{}_CLIENT_ID", provider.to_uppercase())).ok()?;
    let client_secret = std::env::var(format!("{}_CLIENT_SECRET", provider.to_uppercase())).ok()?;

    let (auth_url, token_url) = match provider {
        "github" => (
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        ),
        "google" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
        ),
        _ => return None,
    };

    let redirect_url = std::env::var(format!("{}_REDIRECT_URL", provider.to_uppercase()))
        .unwrap_or_else(|_| format!("http://localhost:8000/auth/{}/callback", provider));

    Some(
        BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(auth_url.to_string()).unwrap(),
            Some(TokenUrl::new(token_url.to_string()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap()),
    )
}

pub async fn start_oauth_flow(
    Path(provider): Path<String>,
    session: Session,
) -> Result<impl IntoResponse, StatusCode> {
    let client = oauth_client(&provider).ok_or_else(|| {
        tracing::error!("OAuth client not configured for {}", provider);
        StatusCode::NOT_FOUND
    })?;

    let mut req = client.authorize_url(CsrfToken::new_random);
    if provider == "google" {
        req = req
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()));
    } else if provider == "github" {
        req = req.add_scope(Scope::new("user:email".to_string()));
    }

    let (auth_url, csrf_token) = req.url();

    session
        .insert("oauth_state", csrf_token.secret().to_string())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    session
        .insert("oauth_provider", provider)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn handle_oauth_callback(
    State(state): State<AppState>,
    session: Session,
    Path(provider): Path<String>,
    Query(query): Query<AuthRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let session_state: Option<String> = session.get("oauth_state").await.unwrap_or(None);
    let session_provider: Option<String> = session.get("oauth_provider").await.unwrap_or(None);

    if session_state != Some(query.state) || session_provider != Some(provider.clone()) {
        tracing::warn!("OAuth state mismatch, potential CSRF");
        return Err(StatusCode::BAD_REQUEST);
    }

    let client = oauth_client(&provider).ok_or(StatusCode::NOT_FOUND)?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            tracing::error!("OAuth token exchange failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let access_token = token_result.access_token().secret();

    let client = reqwest::Client::new();
    let (provider_id, username_hint) = match provider.as_str() {
        "github" => {
            let res = client
                .get("https://api.github.com/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "lyksos-finance-app")
                .send()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let user: GitHubUser = res
                .json()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            (
                user.id.to_string(),
                user.login.unwrap_or_else(|| user.id.to_string()),
            )
        }
        "google" => {
            let res = client
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let user: GoogleUser = res
                .json()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            (
                user.id.clone(),
                user.email.unwrap_or_else(|| user.id.clone()),
            )
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let user = OAuthAccount::find_user_by_oauth(&state.pool, &provider, &provider_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(u) => {
            session
                .insert("user_id", u.id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Redirect::to("/"))
        }
        None => {
            // Unbound user - Create them!
            use crate::models::User;

            // Generate a fake password hash since they use oauth natively
            let fake_hash = "OAUTH_MANAGED_ACCOUNT_NO_PASSWORD";

            // Create user with the hint, if user exists fallback to hint + provider ID
            let actual_username = match User::create(&state.pool, &username_hint, fake_hash).await {
                Ok(_) => username_hint,
                Err(_) => {
                    let fallback = format!("{}_{}", username_hint, provider_id);
                    User::create(&state.pool, &fallback, fake_hash)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    fallback
                }
            };

            // Retrieve the newly created user ID
            let new_user = User::find_by_username(&state.pool, &actual_username)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

            // Bind the OAuth provider ID directly to their new account profile
            sqlx::query(
                "INSERT INTO oauth_accounts (user_id, provider, provider_id) VALUES (?, ?, ?)",
            )
            .bind(new_user.id)
            .bind(&provider)
            .bind(&provider_id)
            .execute(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Establish Session ID and send to dashboard
            session
                .insert("user_id", new_user.id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            tracing::info!(
                "Created new OAuth user: {} via {}",
                actual_username,
                provider
            );
            Ok(Redirect::to("/"))
        }
    }
}
