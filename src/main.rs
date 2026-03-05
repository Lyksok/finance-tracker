mod auth;
mod db;
mod handlers;
mod models;
mod templates;

use axum::{
    routing::{get, post},
    Router,
};
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
}

use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use handlers::{
    auth::{login, logout, register, render_login, render_register},
    categories::{create_category, render_categories},
    dashboard::render_dashboard,
    profile::{render_profile, update_profile},
    transactions::{create_record, render_add_record, render_edit_record, update_record},
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let file_appender = tracing_appender::rolling::daily("logs", "events.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finance_tracker=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    let pool = db::init_db().await;
    let state = AppState { pool: pool.clone() };

    // Setup Session Store
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(1)));

    // Setup Router
    let app = Router::new()
        .route("/login", get(render_login).post(login))
        .route("/register", get(render_register).post(register))
        .route("/logout", post(logout))
        .route("/profile", get(render_profile).post(update_profile))
        .route("/", get(render_dashboard))
        .route("/records/new", get(render_add_record))
        .route("/records", post(create_record))
        .route(
            "/records/:id/edit",
            get(render_edit_record).post(update_record),
        )
        .route("/categories", get(render_categories).post(create_category))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(session_layer)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_db_initialization() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                c_type TEXT NOT NULL CHECK(c_type IN ('INCOME', 'EXPENSE')),
                user_id INTEGER NOT NULL DEFAULT 1 REFERENCES users(id),
                UNIQUE(name COLLATE NOCASE, user_id)
            );
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row_count.0, 0);
    }
}
