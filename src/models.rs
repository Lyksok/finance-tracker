use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

impl User {
    pub async fn find_by_username(
        pool: &SqlitePool,
        username: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &SqlitePool,
        username: &str,
        password_hash: &str,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
            .bind(username)
            .bind(password_hash)
            .execute(pool)
            .await
    }

    pub async fn update_password(
        pool: &SqlitePool,
        user_id: i64,
        new_password_hash: &str,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(new_password_hash)
            .bind(user_id)
            .execute(pool)
            .await
    }
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub c_type: String, // "INCOME" or "EXPENSE"
}

impl Category {
    pub async fn find_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM categories WHERE user_id = ? ORDER BY c_type DESC, name ASC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    pub async fn count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories")
            .fetch_one(pool)
            .await?;
        Ok(count.0)
    }

    pub async fn create(
        pool: &SqlitePool,
        name: &str,
        c_type: &str,
        user_id: i64,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        sqlx::query("INSERT INTO categories (name, c_type, user_id) VALUES (?, ?, ?)")
            .bind(name)
            .bind(c_type)
            .bind(user_id)
            .execute(pool)
            .await
    }
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub amount: i64,  // Stored in cents
    pub date: String, // stored as YYYY-MM-DD string
    pub description: String,
    pub category_id: i64,
}

impl Transaction {
    pub async fn find_for_user(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM transactions WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &SqlitePool,
        amount_cents: i64,
        date: &str,
        description: &str,
        category_id: i64,
        user_id: i64,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        sqlx::query(
            "INSERT INTO transactions (amount, date, description, category_id, user_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(amount_cents)
        .bind(date)
        .bind(description)
        .bind(category_id)
        .bind(user_id)
        .execute(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: i64,
        user_id: i64,
        amount_cents: i64,
        date: &str,
        description: &str,
        category_id: i64,
    ) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
        sqlx::query(
            "UPDATE transactions SET amount = ?, date = ?, description = ?, category_id = ? WHERE id = ? AND user_id = ?",
        )
        .bind(amount_cents)
        .bind(date)
        .bind(description)
        .bind(category_id)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct TransactionDetail {
    pub id: i64,
    pub amount: i64,
    pub date: String,
    pub description: String,
    pub category_name: String,
    pub c_type: String,
}

impl TransactionDetail {
    pub async fn find_monthly_for_user(
        pool: &SqlitePool,
        month: &str,
        user_id: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT t.id, t.amount, t.date, t.description, c.name as category_name, c.c_type
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
            WHERE strftime('%Y-%m', t.date) = ? AND t.user_id = ?
            ORDER BY c.c_type DESC, c.name ASC, t.date DESC
            "#,
        )
        .bind(month)
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    pub fn formatted_amount(&self) -> String {
        format!("{:.2}", (self.amount as f64) / 100.0)
    }
}
