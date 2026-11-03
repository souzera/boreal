use serde::{Deserialize, Serialize};

/// Valor conferido de uma modalidade no fechamento; o esperado é sempre recalculado no backend.
#[derive(Debug, Clone, Deserialize)]
pub struct CountedInput {
    pub method: String,
    pub counted_cents: i64,
    #[serde(default)]
    pub justification: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloseInput {
    /// Data local a fechar, `YYYY-MM-DD`.
    pub date: String,
    pub counted: Vec<CountedInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpectedLineView {
    pub method: String,
    pub label: String,
    pub received_cents: i64,
    pub refunded_cents: i64,
    /// Recebido menos devolvido no dia.
    pub expected_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClosingLineView {
    pub method: String,
    pub label: String,
    pub expected_cents: i64,
    pub counted_cents: i64,
    pub difference_cents: i64,
    pub justification: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClosingView {
    pub id: i64,
    pub closing_date: String,
    pub closed_at: String,
    pub closed_by_name: String,
    pub lines: Vec<ClosingLineView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClosingPreview {
    pub date: String,
    pub closed: Option<ClosingView>,
    /// Esperado calculado agora com os movimentos do dia local.
    pub expected: Vec<ExpectedLineView>,
    /// Pagamentos a fornecedores no dia: informativo, não entra no esperado (M5-10).
    pub supplier_payments_cents: i64,
    /// Com o dia fechado: quanto o esperado mudou por movimentos feitos depois do fechamento.
    pub after_closing_cents: Option<i64>,
}
