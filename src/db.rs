use crate::models::User;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

pub async fn init_db() -> SqlitePool {
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
    let admin_exists = User::find_by_username(&pool, "admin")
        .await
        .unwrap()
        .is_some();

    if !admin_exists {
        let salt = SaltString::generate(&mut OsRng);
        let default_hash = Argon2::default()
            .hash_password(b"admin", &salt)
            .expect("Failed to hash default admin password")
            .to_string();
        User::create(&pool, "admin", &default_hash)
            .await
            .expect("Failed to insert default admin user");
    }

    pool
}
