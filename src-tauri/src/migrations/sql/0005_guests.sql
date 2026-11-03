CREATE TABLE guests (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    code       TEXT    NOT NULL UNIQUE,
    full_name  TEXT    NOT NULL,
    name_key   TEXT    NOT NULL,   -- nome sem acentos/caixa, para comparação e busca
    phone      TEXT    NOT NULL,   -- só dígitos
    cpf        TEXT,               -- só dígitos; NULL se não informado
    email      TEXT    NOT NULL DEFAULT '',
    address    TEXT    NOT NULL DEFAULT '',
    active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    version    INTEGER NOT NULL DEFAULT 1,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);

-- Duplicidade é apenas um alerta (spec §3.2): por isso os índices não são únicos.
CREATE INDEX idx_guests_name_key ON guests(name_key);
CREATE INDEX idx_guests_phone ON guests(phone);
CREATE INDEX idx_guests_cpf ON guests(cpf);
