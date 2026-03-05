use crate::AppState;
use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use chrono::{Datelike, Local};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::auth::AuthUser;
use crate::models::{Category, Transaction, TransactionDetail};
use crate::templates::{
    AddRecordTemplate, CategoriesTemplate, CategoryGroup, DashboardTemplate, EditRecordTemplate,
    LoginTemplate, ProfileTemplate, RegisterTemplate,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tower_sessions::Session;

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
    auth_user: AuthUser,
    State(state): State<AppState>,
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
        WHERE strftime('%Y-%m', t.date) = ? AND t.user_id = ?
        ORDER BY c.c_type DESC, c.name ASC, t.date DESC
        "#,
    )
    .bind(&selected_month)
    .bind(auth_user.0.id)
    .fetch_all(&state.pool)
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
        logged_in: true,
        selected_month: selected_month.clone(),
        total_income: format!("{:.2}", (income_cents as f64) / 100.0),
        total_expense: format!("{:.2}", (expense_cents as f64) / 100.0),
        net_balance: format!("{:.2}", (net_cents as f64) / 100.0),
        groups,
    };

    tracing::debug!(
        "User {} accessed dashboard for month {}",
        auth_user.0.username,
        selected_month
    );

    Html(tpl.render().unwrap())
}

// --- TRANSACTIONS ---

pub async fn render_add_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories WHERE user_id = ? ORDER BY c_type DESC, name ASC",
    )
    .bind(auth_user.0.id)
    .fetch_all(&state.pool)
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

    sqlx::query(
        "INSERT INTO transactions (amount, date, description, category_id, user_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(amount_cents)
    .bind(&form.date)
    .bind(&form.description)
    .bind(form.category_id)
    .bind(auth_user.0.id)
    .execute(&state.pool)
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

pub async fn render_edit_record(
    auth_user: AuthUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    let transaction =
        sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth_user.0.id)
            .fetch_optional(&state.pool)
            .await
            .unwrap();

    let transaction = match transaction {
        Some(t) => t,
        None => return Redirect::to("/").into_response(),
    };

    let categories = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories WHERE user_id = ? ORDER BY c_type DESC, name ASC",
    )
    .bind(auth_user.0.id)
    .fetch_all(&state.pool)
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

    sqlx::query(
        "UPDATE transactions SET amount = ?, date = ?, description = ?, category_id = ? WHERE id = ? AND user_id = ?",
    )
    .bind(amount_cents)
    .bind(&form.date)
    .bind(&form.description)
    .bind(form.category_id)
    .bind(id)
    .bind(auth_user.0.id)
    .execute(&state.pool)
    .await
    .unwrap();

    tracing::info!("User {} updated transaction {}", auth_user.0.username, id);

    Redirect::to("/").into_response()
}

// --- CATEGORIES ---

pub async fn render_categories(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let categories = sqlx::query_as::<_, Category>(
        "SELECT * FROM categories WHERE user_id = ? ORDER BY c_type DESC, name ASC",
    )
    .bind(auth_user.0.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    let tpl = CategoriesTemplate {
        logged_in: true,
        categories,
    };
    Html(tpl.render().unwrap())
}

pub async fn create_category(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateCategoryForm>,
) -> impl IntoResponse {
    sqlx::query("INSERT INTO categories (name, c_type, user_id) VALUES (?, ?, ?)")
        .bind(&form.name)
        .bind(form.c_type.to_uppercase())
        .bind(auth_user.0.id)
        .execute(&state.pool)
        .await
        .unwrap();

    tracing::info!(
        "User {} created new category: {}",
        auth_user.0.username,
        form.name
    );

    Redirect::to("/categories")
}

// --- AUTHENTICATION ---

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn render_login() -> impl IntoResponse {
    Html(
        LoginTemplate {
            logged_in: false,
            error: None,
        }
        .render()
        .unwrap(),
    )
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE username = ?")
        .bind(&form.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let valid = match user {
        Some(ref u) => {
            let parsed_hash = PasswordHash::new(&u.password_hash)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Argon2::default()
                .verify_password(form.password.as_bytes(), &parsed_hash)
                .is_ok()
        }
        None => false,
    };

    if valid {
        let u = user.unwrap();
        session.insert("user_id", u.id).await.map_err(|e| {
            tracing::error!("Failed to save session for user {}: {:?}", u.username, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        tracing::info!("User {} logged in successfully", u.username);
        return Ok(Redirect::to("/").into_response());
    }

    tracing::warn!("Failed login attempt for username: {}", form.username);

    let tpl = LoginTemplate {
        logged_in: false,
        error: Some("Invalid username or password".to_string()),
    };
    Ok(Html(
        tpl.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response())
}

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub password: String,
}

pub async fn render_register() -> impl IntoResponse {
    Html(
        RegisterTemplate {
            logged_in: false,
            error: None,
        }
        .render()
        .unwrap(),
    )
}

pub async fn register(
    State(state): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(form.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let res = sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&form.username)
        .bind(&hash)
        .execute(&state.pool)
        .await;

    match res {
        Ok(_) => {
            tracing::info!("New user registered successfully: {}", form.username);
            Ok(Redirect::to("/login").into_response())
        }
        Err(e) => {
            tracing::warn!(
                "Failed registration attempt (username likely exists): {} - {:?}",
                form.username,
                e
            );
            let tpl = RegisterTemplate {
                logged_in: false,
                error: Some("Username already exists".to_string()),
            };
            Ok(Html(
                tpl.render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            )
            .into_response())
        }
    }
}

pub async fn logout(session: Session) -> Result<impl IntoResponse, StatusCode> {
    session.delete().await.map_err(|e| {
        tracing::error!("Failed to delete session on logout: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("A user logged out successfully");
    Ok(Redirect::to("/login").into_response())
}

#[derive(Deserialize)]
pub struct UpdateProfileForm {
    pub current_password: String,
    pub new_password: String,
}

pub async fn render_profile(auth_user: AuthUser) -> impl IntoResponse {
    let tpl = ProfileTemplate {
        logged_in: true,
        username: auth_user.0.username,
        error: None,
        success: None,
    };
    Html(tpl.render().unwrap())
}

pub async fn update_profile(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<UpdateProfileForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let parsed_hash = PasswordHash::new(&auth_user.0.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let valid = Argon2::default()
        .verify_password(form.current_password.as_bytes(), &parsed_hash)
        .is_ok();

    if !valid {
        tracing::warn!(
            "User {} failed to update password: incorrect current password",
            auth_user.0.username
        );
        let tpl = ProfileTemplate {
            logged_in: true,
            username: auth_user.0.username.clone(),
            error: Some("Incorrect current password".to_string()),
            success: None,
        };
        return Ok(Html(
            tpl.render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .into_response());
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(form.new_password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(auth_user.0.id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to update password hashing for user {}: {:?}",
                auth_user.0.username,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "User {} updated their password successfully",
        auth_user.0.username
    );

    let tpl = ProfileTemplate {
        logged_in: true,
        username: auth_user.0.username,
        error: None,
        success: Some("Password updated successfully".to_string()),
    };
    Ok(Html(
        tpl.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .into_response())
}
