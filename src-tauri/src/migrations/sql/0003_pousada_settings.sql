CREATE TABLE pousada (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    name       TEXT    NOT NULL,
    address    TEXT    NOT NULL DEFAULT '',
    phone      TEXT    NOT NULL DEFAULT '',
    logo       BLOB,
    logo_mime  TEXT,
    version    INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT    NOT NULL
);

INSERT INTO pousada (id, name, updated_at) VALUES (1, '', '1970-01-01T00:00:00Z');

-- Valores iniciais provisórios (pendência §25.5 da especificação); configuráveis na tela.
INSERT INTO settings (key, value) VALUES
    ('default_check_in_time', '14:00'),
    ('default_check_out_time', '12:00'),
    ('late_checkout_tolerance_minutes', '30'),
    ('no_show_tolerance_minutes', '60'),
    ('cleaning_interval_minutes', '60'),
    ('settings_version', '1');
