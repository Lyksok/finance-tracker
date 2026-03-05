use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use axum_extra::extract::{cookie::Key, PrivateCookieJar};

use crate::{models::User, AppState};

pub struct AuthUser(pub User);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = PrivateCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse cookies"))?;

        if let Some(cookie) = jar.get("session_id") {
            let user_id_str = cookie.value();
            if let Ok(user_id) = user_id_str.parse::<i64>() {
                let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                    .bind(user_id)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

                if let Some(user) = user {
                    return Ok(AuthUser(user));
                }
            }
        }

        Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}
