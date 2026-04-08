use crate::AppState;
use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
};
use chrono::Local;
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::models::{Category, Transaction};
use crate::templates::{AddRecordTemplate, EditRecordTemplate};

#[derive(Deserialize)]
pub struct CreateTransactionForm {
    pub amount: f64,
    pub date: String,
    pub description: String,
    pub category_id: i64,
}

pub async fn render_add_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let categories = Category::find_for_user(&state.pool, auth_user.0.id)
        .await
        .unwrap_or_default();

    let today = Local::now().naive_local().format("%Y-%m-%d").to_string();

    let tpl = AddRecordTemplate {
        logged_in: true,
        today,
        categories,
    };
    Html(tpl.render().unwrap())
}

pub async fn create_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateTransactionForm>,
) -> impl IntoResponse {
    let amount_cents = (form.amount * 100.0).round() as i64;

    Transaction::create(
        &state.pool,
        amount_cents,
        &form.date,
        &form.description,
        form.category_id,
        auth_user.0.id,
    )
    .await
    .unwrap();

    tracing::info!(
        "User {} created a new record: {} cents, category ID {}",
        auth_user.0.username,
        amount_cents,
        form.category_id
    );

    Redirect::to("/")
}

#[derive(Deserialize)]
pub struct UpdateTransactionForm {
    pub amount: f64,
    pub date: String,
    pub description: String,
    pub category_id: i64,
}

pub async fn render_edit_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let transaction = Transaction::find_for_user(&state.pool, id, auth_user.0.id)
        .await
        .unwrap();

    let transaction = match transaction {
        Some(t) => t,
        None => return Redirect::to("/").into_response(),
    };

    let categories = Category::find_for_user(&state.pool, auth_user.0.id)
        .await
        .unwrap_or_default();

    let formatted_amount = format!("{:.2}", (transaction.amount as f64) / 100.0);

    let tpl = EditRecordTemplate {
        logged_in: true,
        transaction,
        formatted_amount,
        categories,
    };
    Html(tpl.render().unwrap()).into_response()
}

pub async fn update_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Form(form): Form<UpdateTransactionForm>,
) -> impl IntoResponse {
    let amount_cents = (form.amount * 100.0).round() as i64;

    Transaction::update(
        &state.pool,
        id,
        auth_user.0.id,
        amount_cents,
        &form.date,
        &form.description,
        form.category_id,
    )
    .await
    .unwrap();

    tracing::info!("User {} updated transaction {}", auth_user.0.username, id);

    Redirect::to("/").into_response()
}
