use axum::{
    extract::{Query, State, Form},
    response::{Html, IntoResponse, Redirect},
};
use askama::Template;
use chrono::{Datelike, Local};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;

use crate::models::{Category, Transaction, TransactionDetail};
use crate::templates::{
    AddRecordTemplate, CategoriesTemplate, CategoryGroup, DashboardTemplate, EditRecordTemplate,
};

// --- QUERY & FORM PARAMETERS ---

#[derive(Deserialize)]
pub struct DashboardQuery {
    pub month: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateCategoryForm {
    pub name: String,
    pub c_type: String,
}

#[derive(Deserialize)]
pub struct CreateTransactionForm {
    pub amount: f64,
    pub date: String,
    pub description: String,
    pub category_id: i64,
}

#[derive(Deserialize)]
pub struct UpdateTransactionForm {
    pub amount: f64,
    pub date: String,
    pub description: String,
    pub category_id: i64,
}

// --- DASHBOARD ---

pub async fn render_dashboard(
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

// --- TRANSACTIONS ---

pub async fn render_add_record(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY c_type DESC, name ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let today = Local::now().naive_local().format("%Y-%m-%d").to_string();

    let tpl = AddRecordTemplate { today, categories };
    Html(tpl.render().unwrap())
}

pub async fn create_record(
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

pub async fn render_edit_record(
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

pub async fn update_record(
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

// --- CATEGORIES ---

pub async fn render_categories(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY c_type DESC, name ASC")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
    let tpl = CategoriesTemplate { categories };
    Html(tpl.render().unwrap())
}

pub async fn create_category(
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
