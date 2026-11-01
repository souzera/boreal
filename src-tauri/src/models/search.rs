use serde::Serialize;

/// Até 8 itens de um grupo de resultados, mais o total encontrado.
#[derive(Debug, Clone, Serialize)]
pub struct Group<T> {
    pub items: Vec<T>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuestHit {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub phone: String,
    /// CPF sempre mascarado (`***.***.***-25`).
    pub cpf_masked: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReservationHit {
    pub id: i64,
    pub code: String,
    pub guest_name: String,
    pub room_code: String,
    pub check_in_date: String,
    pub check_out_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StayHit {
    pub id: i64,
    pub code: String,
    pub guest_name: String,
    pub room_code: String,
    /// `in_house` ou `checked_out`.
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MovementHit {
    /// `payment`, `refund` ou `payable`.
    pub kind: String,
    pub code: String,
    pub label: String,
    pub amount_cents: i64,
    pub at: String,
    /// Rota da tela do registro relacionado.
    pub link: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub guests: Group<GuestHit>,
    pub reservations: Group<ReservationHit>,
    pub stays: Group<StayHit>,
    pub movements: Group<MovementHit>,
}
