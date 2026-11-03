use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ExpenseCategory {
    pub id: i64,
    pub name: String,
    pub active: bool,
    pub version: i64,
}

/// Pagamento à vista no momento do cadastro: grava a conta já quitada, com esta modalidade.
#[derive(Debug, Clone, Deserialize)]
pub struct PayNow {
    pub method: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewPayable {
    pub supplier: String,
    pub description: String,
    pub amount_cents: i64,
    /// Data da despesa (competência), `YYYY-MM-DD`.
    pub competence_date: String,
    pub due_date: String,
    pub category_id: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub paid_now: Option<PayNow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PayInput {
    pub amount_cents: i64,
    /// `cash`, `pix`, `debit`, `credit` ou `transfer`.
    pub method: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PayableQuery {
    /// `pending`, `partial`, `paid` ou `overdue`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category_id: Option<i64>,
    #[serde(default)]
    pub due_from: Option<String>,
    #[serde(default)]
    pub due_to: Option<String>,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayableView {
    pub id: i64,
    pub code: String,
    pub supplier: String,
    pub description: String,
    pub amount_cents: i64,
    pub paid_cents: i64,
    pub balance_cents: i64,
    /// `pending`, `partial`, `paid` ou `overdue` (calculado).
    pub status: String,
    pub status_label: String,
    pub competence_date: String,
    pub due_date: String,
    pub category_id: i64,
    pub category_name: String,
    pub notes: String,
    pub version: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayablePaymentLine {
    pub id: i64,
    pub amount_cents: i64,
    pub method: String,
    pub paid_at: String,
    pub paid_by_name: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayableDetail {
    pub payable: PayableView,
    pub payments: Vec<PayablePaymentLine>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SummaryQuery {
    pub from: String,
    pub to: String,
    /// `competence` (data da despesa) ou `payment` (data efetiva de cada pagamento).
    pub basis: String,
    #[serde(default)]
    pub category_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryTotal {
    pub category_id: i64,
    pub category_name: String,
    pub total_cents: i64,
}
