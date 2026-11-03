-- Histórico de backups e eventos de saúde do sistema (spec §15). Nada é alterado nem apagado.
CREATE TABLE backup_runs (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    kind           TEXT    NOT NULL CHECK (kind IN ('manual', 'auto', 'pre_migration', 'pre_restore')),
    status         TEXT    NOT NULL CHECK (status IN ('ok', 'failed')),
    started_at     TEXT    NOT NULL,
    finished_at    TEXT    NOT NULL,
    file_name      TEXT,
    bytes          INTEGER NOT NULL DEFAULT 0,
    schema_version INTEGER,
    message        TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX idx_backup_runs_finished ON backup_runs(finished_at);

CREATE TRIGGER backup_runs_no_update BEFORE UPDATE ON backup_runs
BEGIN
    SELECT RAISE(ABORT, 'histórico de backup é imutável');
END;

CREATE TRIGGER backup_runs_no_delete BEFORE DELETE ON backup_runs
BEGIN
    SELECT RAISE(ABORT, 'histórico de backup é imutável');
END;

CREATE TABLE system_events (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    kind    TEXT NOT NULL CHECK (kind IN ('update_failed', 'update_ok', 'integrity_problem', 'restore')),
    at      TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_system_events_kind ON system_events(kind, at);

CREATE TRIGGER system_events_no_update BEFORE UPDATE ON system_events
BEGIN
    SELECT RAISE(ABORT, 'eventos do sistema são imutáveis');
END;

CREATE TRIGGER system_events_no_delete BEFORE DELETE ON system_events
BEGIN
    SELECT RAISE(ABORT, 'eventos do sistema são imutáveis');
END;

-- Pasta de destino dos backups; vazio = a pasta "backups" ao lado do banco.
INSERT INTO settings (key, value) VALUES ('backup_dir', '');
