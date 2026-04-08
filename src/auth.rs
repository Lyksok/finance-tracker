use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

use crate::{models::User, AppState};

pub struct AuthUser(pub User);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session").into_response())?;

        if let Ok(Some(user_id)) = session.get::<i64>("user_id").await {
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response())?;

            if let Some(user) = user {
                return Ok(AuthUser(user));
            }
        }

        Err(Redirect::to("/login").into_response())
    }
}
