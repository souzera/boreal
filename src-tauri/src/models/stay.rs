use serde::{Deserialize, Serialize};

/// Pessoa hospedada informada no check-in. O titular da reserva deve constar da lista (`guest_id`).
#[derive(Debug, Clone, Deserialize)]
pub struct OccupantInput {
    pub full_name: String,
    pub guest_id: Option<i64>,
    #[serde(default)]
    pub document: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckInInput {
    pub reservation_id: i64,
    pub expected_version: i64,
    pub occupants: Vec<OccupantInput>,
    /// Aceita mais pessoas que a capacidade do quarto; exige `ExceedCapacity`.
    #[serde(default)]
    pub exceed_capacity: bool,
}

/// Hospedagem sem reserva prévia: entra hoje, agora.
#[derive(Debug, Clone, Deserialize)]
pub struct WalkInInput {
    pub room_id: i64,
    pub guest_id: i64,
    pub check_out_date: String,
    /// `HH:MM`; se ausente, usa o horário padrão de saída das configurações.
    pub expected_check_out_time: Option<String>,
    pub occupants: Vec<OccupantInput>,
    #[serde(default)]
    pub exceed_capacity: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Occupant {
    pub id: i64,
    pub full_name: String,
    pub guest_id: Option<i64>,
    pub document: String,
    pub is_holder: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stay {
    pub id: i64,
    pub code: String,
    pub reservation_id: i64,
    /// `in_house` ou `checked_out`.
    pub status: String,
    pub room_id: i64,
    pub room_code: String,
    pub guest_id: i64,
    pub guest_name: String,
    pub check_in_date: String,
    pub check_out_date: String,
    pub expected_check_in_time: String,
    pub expected_check_out_time: String,
    pub checked_in_at: String,
    pub checked_out_at: Option<String>,
    pub early_check_in: bool,
    pub late_tolerance_minutes: Option<i64>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChargeLine {
    pub id: i64,
    /// `night`, `consumption`, `extra`, `discount`, `adjustment`, `late_fee` ou `reversal`.
    pub kind: String,
    pub description: String,
    pub amount_cents: i64,
    pub night_date: Option<String>,
    pub justification: Option<String>,
    pub reverses_charge_id: Option<i64>,
    /// Já foi estornado por outro lançamento (continua no histórico).
    pub reversed: bool,
    pub created_by_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentLine {
    pub id: i64,
    pub code: String,
    pub amount_cents: i64,
    pub method: String,
    pub received_at: String,
    pub received_by_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundLine {
    pub id: i64,
    pub code: String,
    pub payment_code: String,
    pub amount_cents: i64,
    pub reason: String,
    pub refunded_at: String,
}

/// Conta da hospedagem: `saldo = lançamentos − recebido + devolvido`.
#[derive(Debug, Clone, Serialize)]
pub struct StayAccount {
    pub charges: Vec<ChargeLine>,
    pub payments: Vec<PaymentLine>,
    pub refunds: Vec<RefundLine>,
    pub charges_total_cents: i64,
    pub received_cents: i64,
    pub refunded_cents: i64,
    pub balance_cents: i64,
}

/// Saída atrasada que pode atrapalhar a próxima chegada no mesmo quarto (só um aviso).
#[derive(Debug, Clone, Serialize)]
pub struct DepartureRiskView {
    pub next_reservation_id: i64,
    pub next_code: String,
    pub next_start: String,
    pub ready_at: String,
    pub overrun_minutes: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaySummary {
    pub stay: Stay,
    pub balance_cents: i64,
    pub minutes_late: i64,
    pub risk: Option<DepartureRiskView>,
}

#[derive(Debug, Serialize)]
pub struct StayDetail {
    pub stay: Stay,
    pub occupants: Vec<Occupant>,
    pub account: StayAccount,
}

#[derive(Debug, Deserialize)]
pub struct StayQuery {
    /// Código, nome do hóspede ou código do quarto.
    pub text: Option<String>,
    /// `in_house` (padrão) ou `checked_out`.
    pub status: Option<String>,
    pub page: u32,
    pub page_size: u32,
}
