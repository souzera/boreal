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

/// Tarefa da fila com a prioridade já resolvida (ajuste manual, senão a sugerida).
#[derive(Debug, Clone, Serialize)]
pub struct CleaningItem {
    pub task: CleaningTask,
    pub priority: String,
    pub suggested_priority: String,
    /// Próxima chegada prevista no quarto, `AAAA-MM-DDTHH:MM`.
    pub next_arrival: Option<String>,
}
