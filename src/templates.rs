use crate::models::{Category, Transaction, TransactionDetail};
use askama::Template;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub logged_in: bool,
    pub selected_month: String,
    pub total_income: String,
    pub total_expense: String,
    pub net_balance: String,
    pub groups: Vec<CategoryGroup>,
}

pub struct CategoryGroup {
    pub category_name: String,
    pub category_type: String,
    pub total: String,
    pub transactions: Vec<TransactionDetail>,
}

#[derive(Template)]
#[template(path = "add_record.html")]
pub struct AddRecordTemplate {
    pub logged_in: bool,
    pub today: String,
    pub categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "edit_record.html")]
pub struct EditRecordTemplate {
    pub logged_in: bool,
    pub transaction: Transaction,
    pub formatted_amount: String,
    pub categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "categories.html")]
pub struct CategoriesTemplate {
    pub logged_in: bool,
    pub categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub logged_in: bool,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {
    pub logged_in: bool,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate {
    pub logged_in: bool,
    pub username: String,
    pub error: Option<String>,
    pub success: Option<String>,
}
