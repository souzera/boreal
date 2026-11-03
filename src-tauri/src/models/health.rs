use serde::Serialize;

/// Espaço em disco da pasta do banco.
#[derive(Debug, Clone, Serialize)]
pub struct DiskStatus {
    pub free_bytes: u64,
    pub db_bytes: u64,
    /// Abaixo disto o sistema avisa: o maior entre 1 GB e três vezes o tamanho do banco.
    pub threshold_bytes: u64,
    pub low: bool,
}

/// Resultado da verificação feita ao abrir o aplicativo.
#[derive(Debug, Clone, Serialize)]
pub struct StartupReport {
    /// Instante UTC da verificação.
    pub checked_at: String,
    pub integrity_ok: bool,
    /// Descrições curtas, sem dados de hóspedes (no máximo 20).
    pub problems: Vec<String>,
    pub app_version: String,
    pub schema_version: u32,
}
