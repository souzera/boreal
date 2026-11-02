use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DayOccupancy {
    pub date: String,
    pub occupied: i64,
    pub rooms: i64,
    /// Ocupação em pontos-base (10000 = 100%).
    pub rate_bp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodNet {
    pub method: String,
    pub label: String,
    pub received_cents: i64,
    pub refunded_cents: i64,
    pub net_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayNet {
    pub date: String,
    pub net_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceSummary {
    pub to_receive_count: i64,
    pub to_receive_cents: i64,
    pub credit_count: i64,
    pub credit_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryOpen {
    pub category_name: String,
    pub open_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PayablesSummary {
    pub overdue_count: i64,
    pub overdue_cents: i64,
    pub open_cents: i64,
    pub by_category: Vec<CategoryOpen>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagementDashboard {
    pub from: String,
    pub to: String,
    pub rooms_total: i64,
    pub occupancy: Vec<DayOccupancy>,
    pub arrivals: i64,
    pub departures: i64,
    pub in_house_now: i64,
    pub receipts_by_method: Vec<MethodNet>,
    pub receipts_daily: Vec<DayNet>,
    pub balances: BalanceSummary,
    pub payables: PayablesSummary,
}
