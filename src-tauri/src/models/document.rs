use serde::Serialize;

/// Os quatro documentos internos do §17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    ReservationConfirmation,
    PaymentReceipt,
    StayRecord,
    FinalStatement,
}

impl DocumentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentKind::ReservationConfirmation => "reservation_confirmation",
            DocumentKind::PaymentReceipt => "payment_receipt",
            DocumentKind::StayRecord => "stay_record",
            DocumentKind::FinalStatement => "final_statement",
        }
    }

    pub fn parse(value: &str) -> Option<DocumentKind> {
        [DocumentKind::ReservationConfirmation, DocumentKind::PaymentReceipt, DocumentKind::StayRecord, DocumentKind::FinalStatement]
            .into_iter()
            .find(|k| k.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            DocumentKind::ReservationConfirmation => "Confirmação de reserva",
            DocumentKind::PaymentReceipt => "Comprovante de pagamento",
            DocumentKind::StayRecord => "Ficha de hospedagem",
            DocumentKind::FinalStatement => "Demonstrativo final da conta",
        }
    }
}

/// Uma versão emitida de um documento. `current` marca a mais nova.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentView {
    pub id: i64,
    pub kind: String,
    pub kind_label: String,
    pub source_id: i64,
    pub version: i64,
    pub issued_at: String,
    pub issued_by_name: String,
    pub current: bool,
}
