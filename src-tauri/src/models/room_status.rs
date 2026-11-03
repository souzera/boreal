use serde::Serialize;

/// Estado operacional calculado de um quarto ativo (nunca gravado).
#[derive(Debug, Clone, Serialize)]
pub struct RoomStatus {
    pub room_id: i64,
    pub code: String,
    pub category_name: String,
    /// `occupied`, `awaiting_cleaning`, `cleaning`, `clean_waiting_interval`, `available` ou `blocked`.
    pub state: String,
    pub label_pt: String,
    /// Quando o quarto fica livre (`AAAA-MM-DDTHH:MM`), no estado "limpo, aguardando intervalo".
    pub free_at: Option<String>,
    pub cleaning_task_id: Option<i64>,
}
