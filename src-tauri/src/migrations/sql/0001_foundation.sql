CREATE TABLE users (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    username             TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    display_name         TEXT    NOT NULL,
    password_hash        TEXT    NOT NULL,
    role                 TEXT    NOT NULL CHECK (role IN ('admin', 'recepcao')),
    active               INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    must_change_password INTEGER NOT NULL DEFAULT 0 CHECK (must_change_password IN (0, 1)),
    failed_attempts      INTEGER NOT NULL DEFAULT 0,
    locked_until         TEXT,
    version              INTEGER NOT NULL DEFAULT 1,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Valores iniciais provisórios (pendência §25.5 da especificação); configuráveis.
INSERT INTO settings (key, value) VALUES
    ('max_failed_attempts', '5'),
    ('lockout_minutes', '5'),
    ('idle_lock_minutes', '10');

CREATE TABLE audit_log (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER REFERENCES users(id),
    at            TEXT    NOT NULL,
    module        TEXT    NOT NULL,
    operation     TEXT    NOT NULL,
    entity        TEXT    NOT NULL,
    entity_id     INTEGER,
    before_json   TEXT,
    after_json    TEXT,
    justification TEXT
);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log é imutável');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log é imutável');
END;
