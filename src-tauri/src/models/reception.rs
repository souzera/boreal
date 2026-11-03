use serde::Serialize;

use super::search::Group;

#[derive(Debug, Clone, Serialize)]
pub struct ReceptionSummary {
    pub arrivals: i64,
    pub departures: i64,
    pub occupied_rooms: i64,
    pub available_rooms: i64,
    pub total_rooms: i64,
    pub critical_alerts: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArrivalItem {
    pub reservation_id: i64,
    pub code: String,
    pub guest_name: String,
    pub room_code: String,
    pub check_in_date: String,
    pub expected_time: String,
    pub status: String,
    /// Passou da hora prevista com a tolerância de no-show (ou a data já ficou para trás).
    pub overdue: bool,
    pub deposit_overdue: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepartureItem {
    pub stay_id: i64,
    pub code: String,
    pub guest_name: String,
    pub room_code: String,
    pub check_out_date: String,
    pub expected_time: String,
    pub balance_cents: i64,
    pub minutes_late: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskItem {
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub link: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceptionOverview {
    pub date: String,
    pub summary: ReceptionSummary,
    pub arrivals: Group<ArrivalItem>,
    pub departures: Group<DepartureItem>,
    pub at_risk: Group<RiskItem>,
}
