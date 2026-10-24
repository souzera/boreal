use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RoomRow {
    pub id: i64,
    pub code: String,
    pub category_name: String,
    pub active: bool,
}

/// Uma reserva ocupando um quarto num dia da grade.
#[derive(Debug, Serialize)]
pub struct MapCell {
    pub room_id: i64,
    pub date: String,
    pub reservation_id: i64,
    pub code: String,
    pub guest_name: String,
    pub status: String,
    /// Dia da chegada.
    pub is_check_in: bool,
    /// Dia da saída (a noite do dia da saída não é ocupada).
    pub is_check_out: bool,
}

#[derive(Debug, Serialize)]
pub struct RoomMap {
    pub from: String,
    pub to: String,
    pub rooms: Vec<RoomRow>,
    pub cells: Vec<MapCell>,
}
