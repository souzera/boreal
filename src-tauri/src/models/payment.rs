use serde::{Deserialize, Serialize};

/// Parte de um recebimento ou devolução em uma modalidade (`cash`, `pix`, `debit`, `credit`, `transfer`).
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentInput {
    pub method: String,
    pub amount_cents: i64,
}

/// Liberação de um possível recebimento duplicado; exige `ReleaseDuplicatePayment` e justificativa.
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateRelease {
    pub justification: String,
}

/// Recebimento, possivelmente dividido entre modalidades; o total é a soma exata dos componentes.
#[derive(Debug, Clone, Deserialize)]
pub struct PaymentInput {
    pub components: Vec<ComponentInput>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub duplicate_release: Option<DuplicateRelease>,
}

/// A quem pertence o recebimento: conta da hospedagem ou sinal de uma reserva (com a versão vista na tela).
#[derive(Debug, Clone, Copy)]
pub enum PaymentTarget {
    Stay { stay_id: i64 },
    Reservation { reservation_id: i64, expected_version: i64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentReceipt {
    pub id: i64,
    pub code: String,
    pub total_cents: i64,
}

/// O recebimento parecido que bloqueou a gravação.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateInfo {
    pub code: String,
    pub received_at: String,
    pub total_cents: i64,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentLine {
    pub method: String,
    pub amount_cents: i64,
}

/// Recebimento de uma reserva (sinal ou conta), com o que já foi devolvido e o que ainda pode ser.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentSummary {
    pub id: i64,
    pub code: String,
    pub amount_cents: i64,
    pub refunded_cents: i64,
    pub refundable_cents: i64,
    pub components: Vec<ComponentLine>,
    pub received_at: String,
    pub stay_id: Option<i64>,
}
