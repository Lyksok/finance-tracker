use askama::Template;
use crate::models::{Category, Transaction, TransactionDetail};

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
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
    pub today: String,
    pub categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "edit_record.html")]
pub struct EditRecordTemplate {
    pub transaction: Transaction,
    pub formatted_amount: String,
    pub categories: Vec<Category>,
}

#[derive(Template)]
#[template(path = "categories.html")]
pub struct CategoriesTemplate {
    pub categories: Vec<Category>,
}
