use askama::Template;
use axum::{
    extract::{Query, State, Form},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};
use std::collections::BTreeMap;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// --- DATABASE MODELS ---

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub c_type: String, // "INCOME" or "EXPENSE"
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub amount: i64, // Stored in cents
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

// --- TEMPLATES ---

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    selected_month: String,
    total_income: String,
    total_expense: String,
    net_balance: String,
    groups: Vec<CategoryGroup>,
}

pub struct CategoryGroup {
    pub category_name: String,
    pub category_type: String,
    pub total: String,
    pub transactions: Vec<TransactionDetail>,
}

#[derive(Template)]
#[template(path = "add_record.html")]
struct AddRecordTemplate {
    today: String,
    categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "edit_record.html")]
struct EditRecordTemplate {
    transaction: Transaction,
    formatted_amount: String,
    categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "categories.html")]
struct CategoriesTemplate {
    categories: Vec<Category>,
}

// --- HANDLERS & FORMS ---

#[derive(Deserialize)]
struct DashboardQuery {
    month: Option<String>,
}

#[derive(Deserialize)]
struct CreateCategoryForm {
    name: String,
    c_type: String,
}

#[derive(Deserialize)]
struct CreateTransactionForm {
    amount: f64,
    date: String,
    description: String,
    category_id: i64,
}

#[derive(Deserialize)]
struct UpdateTransactionForm {
    amount: f64,
    date: String,
    description: String,
    category_id: i64,
}

async fn render_dashboard(
    State(pool): State<SqlitePool>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    let selected_month = params.month.unwrap_or_else(|| {
        let now = Local::now().naive_local();
        format!("{:04}-{:02}", now.year(), now.month())
    });

    let records = sqlx::query_as::<_, TransactionDetail>(
        r#"
        SELECT t.id, t.amount, t.date, t.description, c.name as category_name, c.c_type
        FROM transactions t
        JOIN categories c ON t.category_id = c.id
        WHERE strftime('%Y-%m', t.date) = ?
        ORDER BY c.c_type DESC, c.name ASC, t.date DESC
        "#,
    )
    .bind(&selected_month)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let mut income_cents = 0;
    let mut expense_cents = 0;

    let mut grouped: BTreeMap<(String, String), Vec<TransactionDetail>> = BTreeMap::new();

    for r in records {
        if r.c_type == "INCOME" {
            income_cents += r.amount;
        } else {
            expense_cents += r.amount;
        }
        let key = (r.category_name.clone(), r.c_type.clone());
        grouped.entry(key).or_default().push(r);
    }

    let groups: Vec<CategoryGroup> = grouped
        .into_iter()
        .map(|((name, c_type), txs)| {
            let total_cents: i64 = txs.iter().map(|t| t.amount).sum();
            CategoryGroup {
                category_name: name,
                category_type: c_type,
                total: format!("{:.2}", (total_cents as f64) / 100.0),
                transactions: txs,
            }
        })
        .collect();

    let net_cents = income_cents - expense_cents;

    let tpl = DashboardTemplate {
        selected_month,
        total_income: format!("{:.2}", (income_cents as f64) / 100.0),
        total_expense: format!("{:.2}", (expense_cents as f64) / 100.0),
        net_balance: format!("{:.2}", (net_cents as f64) / 100.0),
        groups,
    };
    Html(tpl.render().unwrap())
}

async fn render_add_record(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY c_type DESC, name ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let today = Local::now().naive_local().format("%Y-%m-%d").to_string();

    let tpl = AddRecordTemplate { today, categories };
    Html(tpl.render().unwrap())
}

async fn create_record(
    State(pool): State<SqlitePool>,
    Form(form): Form<CreateTransactionForm>,
) -> impl IntoResponse {
    let amount_cents = (form.amount * 100.0).round() as i64;

    sqlx::query(
        "INSERT INTO transactions (amount, date, description, category_id) VALUES (?, ?, ?, ?)",
    )
    .bind(amount_cents)
    .bind(form.date)
    .bind(form.description)
    .bind(form.category_id)
    .execute(&pool)
    .await
    .unwrap();

    Redirect::to("/")
}

async fn render_edit_record(
    State(pool): State<SqlitePool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let transaction = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .unwrap();

    let transaction = match transaction {
        Some(t) => t,
        None => return Redirect::to("/").into_response(),
    };

    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY c_type DESC, name ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let formatted_amount = format!("{:.2}", (transaction.amount as f64) / 100.0);

    let tpl = EditRecordTemplate { transaction, formatted_amount, categories };
    Html(tpl.render().unwrap()).into_response()
}

async fn update_record(
    State(pool): State<SqlitePool>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Form(form): Form<UpdateTransactionForm>,
) -> impl IntoResponse {
    let amount_cents = (form.amount * 100.0).round() as i64;

    sqlx::query(
        "UPDATE transactions SET amount = ?, date = ?, description = ?, category_id = ? WHERE id = ?",
    )
    .bind(amount_cents)
    .bind(form.date)
    .bind(form.description)
    .bind(form.category_id)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    Redirect::to("/").into_response()
}

async fn render_categories(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY c_type DESC, name ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
    let tpl = CategoriesTemplate { categories };
    Html(tpl.render().unwrap())
}

async fn create_category(
    State(pool): State<SqlitePool>,
    Form(form): Form<CreateCategoryForm>,
) -> impl IntoResponse {
    sqlx::query("INSERT INTO categories (name, c_type) VALUES (?, ?)")
        .bind(form.name)
        .bind(form.c_type.to_uppercase())
        .execute(&pool)
        .await
        .unwrap();

    Redirect::to("/categories")
}

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
            "#
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
