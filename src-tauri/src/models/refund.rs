use serde::{Deserialize, Serialize};

use super::payment::ComponentInput;

/// Devolução de parte ou do todo de um recebimento, dividida entre modalidades.
#[derive(Debug, Clone, Deserialize)]
pub struct RefundInput {
    pub payment_id: i64,
    pub components: Vec<ComponentInput>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundReceipt {
    pub id: i64,
    pub code: String,
    pub total_cents: i64,
}
