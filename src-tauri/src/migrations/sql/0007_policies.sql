-- Valores provisórios (Anexo A da spec, aguardando aprovação); todos editáveis pelo administrador.
INSERT INTO settings (key, value) VALUES
    ('deposit_percent_bp', '3000'),
    ('deposit_due_days_after_booking', '3'),
    ('deposit_due_days_before_arrival', '1'),
    ('cancellation_policy_version', '1');

CREATE TABLE cancellation_tiers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    min_days_before INTEGER NOT NULL UNIQUE CHECK (min_days_before >= 0),
    refund_bp       INTEGER NOT NULL CHECK (refund_bp BETWEEN 0 AND 10000)
);

INSERT INTO cancellation_tiers (min_days_before, refund_bp) VALUES (7, 10000), (3, 5000), (0, 0);
