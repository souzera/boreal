CREATE TABLE rate_rules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id  INTEGER NOT NULL REFERENCES room_categories(id) ON DELETE RESTRICT,
    rule_type    TEXT    NOT NULL CHECK (rule_type IN ('weekday', 'season', 'holiday', 'date_override')),
    weekday      INTEGER CHECK (weekday BETWEEN 0 AND 6),   -- 0 = domingo
    start_date   TEXT,
    end_date     TEXT,
    date         TEXT,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    label        TEXT    NOT NULL DEFAULT '',
    active       INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    version      INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT    NOT NULL,
    updated_at   TEXT    NOT NULL,
    CHECK (
        (rule_type = 'weekday'       AND weekday IS NOT NULL AND start_date IS NULL AND end_date IS NULL AND date IS NULL) OR
        (rule_type = 'season'        AND weekday IS NULL AND start_date IS NOT NULL AND end_date IS NOT NULL AND date IS NULL) OR
        (rule_type = 'holiday'       AND weekday IS NULL AND start_date IS NULL AND end_date IS NULL AND date IS NULL) OR
        (rule_type = 'date_override' AND weekday IS NULL AND start_date IS NULL AND end_date IS NULL AND date IS NOT NULL)
    )
);

CREATE INDEX idx_rate_rules_category ON rate_rules(category_id);

-- Rede de segurança: o serviço valida antes, mas o banco também impede duplicidade entre regras ativas.
CREATE UNIQUE INDEX uq_rate_weekday  ON rate_rules(category_id, weekday) WHERE rule_type = 'weekday' AND active = 1;
CREATE UNIQUE INDEX uq_rate_holiday  ON rate_rules(category_id)          WHERE rule_type = 'holiday' AND active = 1;
CREATE UNIQUE INDEX uq_rate_override ON rate_rules(category_id, date)    WHERE rule_type = 'date_override' AND active = 1;

-- Calendário de feriados (cadastrado pelo administrador); o preço de feriado é a regra 'holiday' da categoria.
CREATE TABLE holidays (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    date       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL
);
