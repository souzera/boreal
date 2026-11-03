use serde::Serialize;

/// Uma execução de backup (ou tentativa) registrada no histórico.
#[derive(Debug, Clone, Serialize)]
pub struct BackupRunView {
    pub id: i64,
    /// `manual`, `auto`, `pre_migration` ou `pre_restore`.
    pub kind: String,
    pub kind_label: String,
    /// `ok` ou `failed`.
    pub status: String,
    pub finished_at: String,
    pub file_name: Option<String>,
    pub bytes: i64,
    pub message: String,
}

/// Um arquivo de backup encontrado na pasta de destino.
#[derive(Debug, Clone, Serialize)]
pub struct BackupFileView {
    pub file_name: String,
    /// Data e hora locais do nome do arquivo, `DD/MM/AAAA HH:MM`.
    pub at: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DestinationView {
    pub path: String,
    /// Sem pasta configurada o sistema usa a pasta "backups" ao lado do banco.
    pub is_default: bool,
}
