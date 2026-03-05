mod auth;
mod handlers;
mod models;
mod templates;

use axum::{
    extract::FromRef,
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
}

use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use handlers::{
    create_category, create_record, login, logout, register, render_add_record, render_categories,
    render_dashboard, render_edit_record, render_login, render_profile, render_register,
    update_profile, update_record,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "solo_finance_watcher=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = "sqlite:finance.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await;

    let pool = match pool {
        Ok(pool) => pool,
        Err(_) => {
            // If DB doesn't exist, try creating it with sqlite
            std::fs::File::create("finance.db").unwrap();
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect("sqlite:finance.db")
                .await
                .unwrap();
            pool
        }
    };

    // Initialize Schema
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

        CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            amount INTEGER NOT NULL,
            date TEXT NOT NULL,
            description TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL DEFAULT 1 REFERENCES users(id),
            FOREIGN KEY(category_id) REFERENCES categories(id)
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Default admin user setup
    let admin_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = 'admin'")
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));

    if admin_exists.0 == 0 {
        let salt = SaltString::generate(&mut OsRng);
        let default_hash = Argon2::default()
            .hash_password(b"admin", &salt)
            .expect("Failed to hash default admin password")
            .to_string();
        sqlx::query("INSERT INTO users (username, password_hash) VALUES ('admin', ?)")
            .bind(default_hash)
            .execute(&pool)
            .await
            .expect("Failed to insert default admin user");
    }

    let state = AppState {
        pool: pool.clone(),
    };

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
        .route("/records/:id/edit", get(render_edit_record).post(update_record))
        .route("/categories", get(render_categories).post(create_category))
        .nest_service("/assets", ServeDir::new("assets"))
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
