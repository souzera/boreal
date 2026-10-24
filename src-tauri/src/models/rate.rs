use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RateRule {
    pub id: i64,
    pub category_id: i64,
    /// `weekday`, `season`, `holiday` ou `date_override`.
    pub rule_type: String,
    /// 0 = domingo … 6 = sábado (só em `weekday`).
    pub weekday: Option<u8>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// Só em `date_override`.
    pub date: Option<String>,
    pub amount_cents: i64,
    pub label: String,
    pub active: bool,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct NewRateRule {
    pub category_id: i64,
    pub rule_type: String,
    pub weekday: Option<u8>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub date: Option<String>,
    pub amount_cents: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Holiday {
    pub id: i64,
    pub date: String,
    pub name: String,
}
