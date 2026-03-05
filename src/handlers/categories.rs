use crate::AppState;
use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::models::Category;
use crate::templates::CategoriesTemplate;

pub async fn render_categories(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let categories = Category::find_for_user(&state.pool, auth_user.0.id)
        .await
        .unwrap_or_default();

    let tpl = CategoriesTemplate {
        logged_in: true,
        categories,
    };
    Html(tpl.render().unwrap())
}

#[derive(Deserialize)]
pub struct CreateCategoryForm {
    pub name: String,
    pub c_type: String,
}

pub async fn create_category(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateCategoryForm>,
) -> impl IntoResponse {
    Category::create(
        &state.pool,
        &form.name,
        &form.c_type.to_uppercase(),
        auth_user.0.id,
    )
    .await
    .unwrap();

    tracing::info!(
        "User {} created new category: {}",
        auth_user.0.username,
        form.name
    );

    Redirect::to("/categories")
}
