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
    /// Cópia de segurança do estado de antes de restaurar ou atualizar (não é um backup diário).
    pub safety: bool,
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

/// Contagens para o administrador comparar o backup com o banco atual antes de restaurar.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
    pub reservations: i64,
    pub stays: i64,
    pub payments: i64,
    pub guests: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestorePreview {
    pub file_name: String,
    /// Data e hora do backup (do nome do arquivo), `DD/MM/AAAA HH:MM`.
    pub backup_at: String,
    pub schema_version: u32,
    pub app_schema_version: u32,
    pub restorable: bool,
    /// Por que não pode ser restaurado, em linguagem simples.
    pub problem: Option<String>,
    pub counts: Counts,
    pub current_counts: Counts,
    /// O banco atual tem atividade mais recente que o backup: o que foi feito depois dele será perdido.
    pub loses_recent_data: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    /// Nome da cópia de segurança do estado anterior.
    pub safety_copy: String,
    pub backup_at: String,
}
