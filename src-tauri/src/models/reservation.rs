use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct NewReservation {
    pub room_id: i64,
    pub guest_id: i64,
    pub check_in_date: String,
    pub check_out_date: String,
    /// `HH:MM`; se ausente, usa o horário padrão das configurações.
    pub expected_check_in_time: Option<String>,
    pub expected_check_out_time: Option<String>,
    pub occupants: u32,
    pub notes: String,
    /// Aceita mais pessoas que a capacidade do quarto; exige `ExceedCapacity`.
    pub exceed_capacity: bool,
}

#[derive(Debug, Serialize)]
pub struct CreatedReservation {
    pub id: i64,
    pub code: String,
    pub agreed_total_cents: i64,
    pub deposit_expected_cents: i64,
    pub deposit_due_date: String,
}

/// Reserva com os dados do quarto e do hóspede, para listas e para a tela da reserva.
#[derive(Debug, Clone, Serialize)]
pub struct Reservation {
    pub id: i64,
    pub code: String,
    pub room_id: i64,
    pub room_code: String,
    pub guest_id: i64,
    pub guest_name: String,
    pub check_in_date: String,
    pub check_out_date: String,
    pub expected_check_in_time: String,
    pub expected_check_out_time: String,
    pub nights: i64,
    pub occupants: i64,
    /// `pending`, `confirmed`, `checked_in`, `completed`, `cancelled` ou `no_show`.
    pub status: String,
    pub list_total_cents: i64,
    pub discount_cents: i64,
    pub discount_reason: Option<String>,
    pub agreed_total_cents: i64,
    pub deposit_expected_cents: i64,
    pub deposit_paid_cents: i64,
    pub deposit_due_date: String,
    /// Sinal ainda não recebido por completo e com prazo vencido (só um alerta; a reserva segue ativa).
    pub deposit_overdue: bool,
    pub notes: String,
    pub cancelled_at: Option<String>,
    pub cancellation_fee_cents: Option<i64>,
    pub version: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReservationNight {
    pub date: String,
    pub amount_cents: i64,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReservationChange {
    pub id: i64,
    pub changed_at: String,
    pub changed_by: i64,
    pub changed_by_name: String,
    pub kind: String,
    pub before_json: String,
    pub after_json: String,
    pub price_before_cents: Option<i64>,
    pub price_after_cents: Option<i64>,
    pub justification: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReservationDetail {
    pub reservation: Reservation,
    pub nights: Vec<ReservationNight>,
    pub history: Vec<ReservationChange>,
}

#[derive(Debug, Deserialize)]
pub struct ReservationQuery {
    /// Código da reserva ou nome do hóspede.
    pub text: Option<String>,
    pub status: Option<Vec<String>>,
    pub room_id: Option<i64>,
    /// Reservas que ocupam algum dia do período `from..=to` (YYYY-MM-DD).
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: u32,
    pub page_size: u32,
    pub sort: Option<String>,
    /// `"asc"` (padrão) ou `"desc"`.
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictInfo {
    pub reservation_id: i64,
    pub code: String,
    pub guest_name: String,
    pub check_in_date: String,
    pub check_out_date: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AvailabilityResult {
    pub available: bool,
    pub conflicts: Vec<ConflictInfo>,
}
