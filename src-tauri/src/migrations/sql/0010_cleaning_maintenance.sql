-- Intervalo mínimo de limpeza próprio do quarto; nulo usa o padrão das configurações (Marco 4, M4-3).
ALTER TABLE rooms ADD COLUMN cleaning_interval_minutes INTEGER CHECK (cleaning_interval_minutes IS NULL OR cleaning_interval_minutes BETWEEN 0 AND 1440);

INSERT INTO sequences (name, next_value) VALUES ('maintenance', 1);

-- Uma tarefa por hospedagem encerrada. O estado do quarto é calculado a partir dela, nunca gravado.
CREATE TABLE cleaning_tasks (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id                INTEGER NOT NULL REFERENCES rooms(id) ON DELETE RESTRICT,
    stay_id                INTEGER UNIQUE REFERENCES stays(id) ON DELETE RESTRICT,
    status                 TEXT    NOT NULL DEFAULT 'awaiting' CHECK (status IN ('awaiting', 'in_progress', 'done')),
    ready_since            TEXT    NOT NULL,   -- hora efetiva da saída
    priority_override      TEXT CHECK (priority_override IS NULL OR priority_override IN ('urgent', 'high', 'normal', 'low')),
    priority_justification TEXT,
    started_at             TEXT,
    started_by             INTEGER REFERENCES users(id),
    completed_at           TEXT,
    completed_by           INTEGER REFERENCES users(id),
    version                INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT    NOT NULL,
    updated_at             TEXT    NOT NULL
);

CREATE INDEX idx_cleaning_tasks_room ON cleaning_tasks(room_id, status);

-- Período em noites: início inclusivo, fim exclusivo; fim nulo = até liberação manual.
CREATE TABLE room_blocks (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id               INTEGER NOT NULL REFERENCES rooms(id) ON DELETE RESTRICT,
    kind                  TEXT    NOT NULL CHECK (kind IN ('manual', 'preventive', 'maintenance')),
    status                TEXT    NOT NULL CHECK (status IN ('preventive', 'active', 'awaiting_release', 'released')),
    start_date            TEXT    NOT NULL,
    end_date              TEXT,
    reason                TEXT    NOT NULL,
    maintenance_order_id  INTEGER,
    created_by            INTEGER NOT NULL REFERENCES users(id),
    released_at           TEXT,
    released_by           INTEGER REFERENCES users(id),
    release_justification TEXT,
    version               INTEGER NOT NULL DEFAULT 1,
    created_at            TEXT    NOT NULL,
    updated_at            TEXT    NOT NULL
);

CREATE INDEX idx_room_blocks_room ON room_blocks(room_id, status);

CREATE TABLE maintenance_orders (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    code          TEXT    NOT NULL UNIQUE,
    room_id       INTEGER NOT NULL REFERENCES rooms(id) ON DELETE RESTRICT,
    description   TEXT    NOT NULL,
    priority      TEXT    NOT NULL CHECK (priority IN ('urgent', 'high', 'normal', 'low')),
    status        TEXT    NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'in_progress', 'awaiting_resolution', 'completed')),
    planned_start TEXT    NOT NULL,
    planned_end   TEXT,
    actual_start  TEXT,
    actual_end    TEXT,
    responsible   TEXT    NOT NULL DEFAULT '',
    notes         TEXT    NOT NULL DEFAULT '',
    version       INTEGER NOT NULL DEFAULT 1,
    created_by    INTEGER NOT NULL REFERENCES users(id),
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL
);

CREATE TABLE maintenance_conflicts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id        INTEGER NOT NULL REFERENCES maintenance_orders(id) ON DELETE RESTRICT,
    reservation_id  INTEGER NOT NULL REFERENCES reservations(id) ON DELETE RESTRICT,
    status          TEXT    NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    resolution_note TEXT,
    resolved_at     TEXT,
    resolved_by     INTEGER REFERENCES users(id),
    UNIQUE (order_id, reservation_id)
);

CREATE INDEX idx_maintenance_conflicts_order ON maintenance_conflicts(order_id, status);
