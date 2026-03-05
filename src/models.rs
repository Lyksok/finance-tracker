use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub c_type: String, // "INCOME" or "EXPENSE"
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub amount: i64,  // Stored in cents
    pub date: String, // stored as YYYY-MM-DD string
    pub description: String,
    pub category_id: i64,
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
    pub fn formatted_amount(&self) -> String {
        format!("{:.2}", (self.amount as f64) / 100.0)
    }
}
