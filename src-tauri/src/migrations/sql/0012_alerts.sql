-- Alertas persistentes (Marco 6.2): mantidos por detectores, resolvidos quando a causa some e nunca apagados.
CREATE TABLE alerts (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    kind             TEXT    NOT NULL CHECK (kind IN (
        'arrival_today', 'departure_today', 'arrival_overdue', 'deposit_overdue', 'departure_late', 'departure_risk',
        'maintenance_conflict', 'cleaning_urgent', 'payable_overdue', 'checkout_balance', 'closing_pending',
        'backup_missing', 'disk_low', 'update_failed')),
    subject_key      TEXT    NOT NULL,
    severity         TEXT    NOT NULL CHECK (severity IN ('critical', 'high', 'normal')),
    title            TEXT    NOT NULL,
    detail           TEXT    NOT NULL,
    link             TEXT    NOT NULL,
    status           TEXT    NOT NULL CHECK (status IN ('active', 'resolved', 'dismissed')),
    -- Só importa em alertas dispensados: a causa ainda existe, então o alerta não volta.
    still_present    INTEGER NOT NULL DEFAULT 1 CHECK (still_present IN (0, 1)),
    first_seen_at    TEXT    NOT NULL,
    last_seen_at     TEXT    NOT NULL,
    resolved_at      TEXT,
    resolved_by      INTEGER REFERENCES users(id),
    resolution_note  TEXT,
    version          INTEGER NOT NULL DEFAULT 1
);

-- Um alerta ativo por causa; um dispensado por causa enquanto ela continuar.
CREATE UNIQUE INDEX idx_alerts_one_active ON alerts(kind, subject_key) WHERE status = 'active';
CREATE UNIQUE INDEX idx_alerts_one_dismissed ON alerts(kind, subject_key) WHERE status = 'dismissed' AND still_present = 1;
CREATE INDEX idx_alerts_status ON alerts(status, severity);

CREATE TABLE alert_events (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    alert_id INTEGER NOT NULL REFERENCES alerts(id) ON DELETE RESTRICT,
    at       TEXT    NOT NULL,
    user_id  INTEGER REFERENCES users(id),
    event    TEXT    NOT NULL CHECK (event IN ('raised', 'updated', 'resolved', 'dismissed')),
    note     TEXT
);

CREATE INDEX idx_alert_events_alert ON alert_events(alert_id);

CREATE TRIGGER alerts_no_delete BEFORE DELETE ON alerts
BEGIN
    SELECT RAISE(ABORT, 'histórico de alertas é imutável');
END;

CREATE TRIGGER alert_events_no_delete BEFORE DELETE ON alert_events
BEGIN
    SELECT RAISE(ABORT, 'histórico de alertas é imutável');
END;
