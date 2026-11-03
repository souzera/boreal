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
