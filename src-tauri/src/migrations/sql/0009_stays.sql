-- Origem da reserva: 'walk_in' nasce já hospedada, sem reserva prévia (Marco 3, decisão M3-1).
ALTER TABLE reservations ADD COLUMN origin TEXT NOT NULL DEFAULT 'booking' CHECK (origin IN ('booking', 'walk_in'));

CREATE TABLE stays (
    id                           INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id               INTEGER NOT NULL UNIQUE REFERENCES reservations(id) ON DELETE RESTRICT,
    code                         TEXT    NOT NULL UNIQUE,
    status                       TEXT    NOT NULL CHECK (status IN ('in_house', 'checked_out')),
    checked_in_at                TEXT    NOT NULL,
    checked_in_by                INTEGER NOT NULL REFERENCES users(id),
    early_check_in               INTEGER NOT NULL DEFAULT 0 CHECK (early_check_in IN (0, 1)),
    exceeded_capacity            INTEGER NOT NULL DEFAULT 0 CHECK (exceeded_capacity IN (0, 1)),
    late_tolerance_minutes       INTEGER CHECK (late_tolerance_minutes IS NULL OR late_tolerance_minutes BETWEEN 0 AND 600),
    checked_out_at               TEXT,
    checked_out_by               INTEGER REFERENCES users(id),
    late_minutes                 INTEGER,
    late_fee_calculated_cents    INTEGER,
    late_fee_decision            TEXT CHECK (late_fee_decision IS NULL OR late_fee_decision IN ('charged', 'waived', 'not_applicable')),
    late_fee_waive_justification TEXT,
    late_fee_waived_by           INTEGER REFERENCES users(id),
    balance_at_checkout_cents    INTEGER,
    balance_justification        TEXT,
    balance_authorized_by        INTEGER REFERENCES users(id),
    version                      INTEGER NOT NULL DEFAULT 1,
    created_at                   TEXT    NOT NULL,
    updated_at                   TEXT    NOT NULL
);

CREATE INDEX idx_stays_status ON stays(status);

CREATE TABLE stay_occupants (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    stay_id   INTEGER NOT NULL REFERENCES stays(id) ON DELETE CASCADE,
    full_name TEXT    NOT NULL,
    guest_id  INTEGER REFERENCES guests(id) ON DELETE RESTRICT,
    document  TEXT    NOT NULL DEFAULT '',
    is_holder INTEGER NOT NULL DEFAULT 0 CHECK (is_holder IN (0, 1))
);

CREATE INDEX idx_stay_occupants_stay ON stay_occupants(stay_id);

CREATE TABLE catalog_items (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT    NOT NULL,
    name_key            TEXT    NOT NULL UNIQUE,  -- nome sem acentos/caixa, para unicidade
    default_price_cents INTEGER NOT NULL CHECK (default_price_cents > 0),
    active              INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    version             INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);

-- Livro de lançamentos da conta: nunca editado nem apagado; correção = estorno (valor negativo) + novo lançamento.
CREATE TABLE charges (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    stay_id            INTEGER NOT NULL REFERENCES stays(id) ON DELETE RESTRICT,
    kind               TEXT    NOT NULL CHECK (kind IN ('night', 'consumption', 'extra', 'discount', 'adjustment', 'late_fee', 'reversal')),
    description        TEXT    NOT NULL,
    amount_cents       INTEGER NOT NULL CHECK (amount_cents <> 0),
    item_id            INTEGER REFERENCES catalog_items(id) ON DELETE RESTRICT,
    night_date         TEXT,
    reverses_charge_id INTEGER UNIQUE REFERENCES charges(id) ON DELETE RESTRICT,
    replaces_charge_id INTEGER REFERENCES charges(id) ON DELETE RESTRICT,
    justification      TEXT,
    created_by         INTEGER NOT NULL REFERENCES users(id),
    created_at         TEXT    NOT NULL
);

CREATE INDEX idx_charges_stay ON charges(stay_id);
CREATE UNIQUE INDEX idx_charges_one_night_per_date ON charges(stay_id, night_date) WHERE kind = 'night';

-- Esquema mínimo antecipado; os serviços completos (divisão por modalidade, duplicidade, fechamento) são do Marco 5.
CREATE TABLE payments (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    code           TEXT    NOT NULL UNIQUE,
    reservation_id INTEGER NOT NULL REFERENCES reservations(id) ON DELETE RESTRICT,
    stay_id        INTEGER REFERENCES stays(id) ON DELETE RESTRICT,
    amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),
    method         TEXT    NOT NULL CHECK (method IN ('cash', 'pix', 'debit', 'credit', 'transfer', 'prior_deposit')),
    received_at    TEXT    NOT NULL,
    received_by    INTEGER NOT NULL REFERENCES users(id),
    note           TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX idx_payments_stay ON payments(stay_id);

CREATE TABLE refunds (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    code         TEXT    NOT NULL UNIQUE,
    payment_id   INTEGER NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    stay_id      INTEGER REFERENCES stays(id) ON DELETE RESTRICT,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    reason       TEXT    NOT NULL,
    refunded_at  TEXT    NOT NULL,
    refunded_by  INTEGER NOT NULL REFERENCES users(id)
);

CREATE INDEX idx_refunds_stay ON refunds(stay_id);

-- Valor provisório (D-15): cobrança de saída tardia, em pontos-base da última diária.
INSERT INTO settings (key, value) VALUES ('late_checkout_fee_bp', '5000');
