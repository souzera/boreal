use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CleaningTask {
    pub id: i64,
    pub room_id: i64,
    pub room_code: String,
    pub stay_id: Option<i64>,
    pub status: String,
    /// Hora efetiva da saída (UTC).
    pub ready_since: String,
    pub priority_override: Option<String>,
    pub priority_justification: Option<String>,
    pub started_at: Option<String>,
    pub started_by: Option<i64>,
    pub completed_at: Option<String>,
    pub completed_by: Option<i64>,
    pub version: i64,
}
