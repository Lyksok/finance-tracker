use crate::AppState;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use chrono::{Datelike, Local};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::auth::AuthUser;
use crate::models::TransactionDetail;
use crate::templates::{CategoryGroup, DashboardTemplate};

#[derive(Deserialize)]
pub struct DashboardQuery {
    pub month: Option<String>,
}

pub async fn render_dashboard(
    auth_user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<DashboardQuery>,
) -> impl IntoResponse {
    let selected_month = params.month.unwrap_or_else(|| {
        let now = Local::now().naive_local();
        format!("{:04}-{:02}", now.year(), now.month())
    });

    let records =
        TransactionDetail::find_monthly_for_user(&state.pool, &selected_month, auth_user.0.id)
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

    let now = Local::now().naive_local();

    let average_day: u32 = if *selected_month == format!("{:04}-{:02}", now.year(), now.month()) {
        now.day()
    } else {
        let year = selected_month[..4].parse::<u16>().unwrap();
        match &selected_month[5..] {
            "01" => 31,
            "02" if ((year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)) => 29,
            "02" => 28,
            "03" => 31,
            "04" => 30,
            "05" => 31,
            "06" => 30,
            "07" => 31,
            "08" => 31,
            "09" => 30,
            "10" => 31,
            "11" => 30,
            "12" => 31,
            _ => 31,
        }
    };
    let tpl = DashboardTemplate {
        logged_in: true,
        selected_month: selected_month.clone(),
        total_income: format!("{:.2}", (income_cents as f64) / 100.0),
        total_expense: format!("{:.2}", (expense_cents as f64) / 100.0),
        avg_expense: format!("{:.2}", (expense_cents as f64) / 100.0 / average_day as f64),
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
