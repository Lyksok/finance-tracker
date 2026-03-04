mod handlers;
mod models;
mod templates;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use handlers::{
    create_category, create_record, render_add_record, render_categories, render_dashboard,
    render_edit_record, update_record,
};

#[tokio::main]
async fn main() {
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
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            c_type TEXT NOT NULL CHECK(c_type IN ('INCOME', 'EXPENSE')),
            UNIQUE(name COLLATE NOCASE)
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            amount INTEGER NOT NULL,
            date TEXT NOT NULL,
            description TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            FOREIGN KEY(category_id) REFERENCES categories(id)
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Setup Router
    let app = Router::new()
        .route("/", get(render_dashboard))
        .route("/records/new", get(render_add_record))
        .route("/records", post(create_record))
        .route("/records/:id/edit", get(render_edit_record).post(update_record))
        .route("/categories", get(render_categories).post(create_category))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(pool.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
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
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                c_type TEXT NOT NULL CHECK(c_type IN ('INCOME', 'EXPENSE')),
                UNIQUE(name COLLATE NOCASE)
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
