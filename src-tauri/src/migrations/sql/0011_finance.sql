-- Financeiro (Marco 5): recebimentos divididos por modalidade, devoluções, contas a pagar e fechamento de caixa.

INSERT INTO sequences (name, next_value) VALUES ('payable', 1);

-- Devolução calculada no cancelamento (valor a devolver ao hóspede); a devolução real é registrada depois.
ALTER TABLE reservations ADD COLUMN cancellation_refund_cents INTEGER NOT NULL DEFAULT 0 CHECK (cancellation_refund_cents >= 0);
UPDATE reservations
   SET cancellation_refund_cents = MAX(0, deposit_paid_cents - COALESCE(cancellation_fee_cents, 0))
 WHERE status = 'cancelled';

-- `payments` e `refunds` são reconstruídas (a modalidade sai da linha e vai para os componentes).
-- Cópias temporárias evitam RENAME, que reescreveria as chaves estrangeiras.
CREATE TABLE payments_legacy AS SELECT * FROM payments;
CREATE TABLE refunds_legacy AS SELECT * FROM refunds;
DROP TABLE refunds;
DROP TABLE payments;

CREATE TABLE payments (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    code                    TEXT    NOT NULL UNIQUE,
    reservation_id          INTEGER NOT NULL REFERENCES reservations(id) ON DELETE RESTRICT,
    stay_id                 INTEGER REFERENCES stays(id) ON DELETE RESTRICT,   -- nulo enquanto é sinal da reserva
    kind                    TEXT    NOT NULL CHECK (kind IN ('deposit', 'stay')),
    amount_cents            INTEGER NOT NULL CHECK (amount_cents > 0),
    received_at             TEXT    NOT NULL,
    received_by             INTEGER NOT NULL REFERENCES users(id),
    note                    TEXT    NOT NULL DEFAULT '',
    duplicate_of_payment_id INTEGER REFERENCES payments(id) ON DELETE RESTRICT,
    duplicate_justification TEXT,
    duplicate_released_by   INTEGER REFERENCES users(id)
);

CREATE INDEX idx_payments_stay ON payments(stay_id);
CREATE INDEX idx_payments_reservation ON payments(reservation_id);
CREATE INDEX idx_payments_received_at ON payments(received_at);

CREATE TABLE payment_components (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    payment_id   INTEGER NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    method       TEXT    NOT NULL CHECK (method IN ('cash', 'pix', 'debit', 'credit', 'transfer', 'unspecified')),
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    UNIQUE (payment_id, method)
);

CREATE TABLE refunds (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    code           TEXT    NOT NULL UNIQUE,
    payment_id     INTEGER NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    reservation_id INTEGER NOT NULL REFERENCES reservations(id) ON DELETE RESTRICT,
    stay_id        INTEGER REFERENCES stays(id) ON DELETE RESTRICT,
    amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),
    reason         TEXT    NOT NULL,
    refunded_at    TEXT    NOT NULL,
    refunded_by    INTEGER NOT NULL REFERENCES users(id)
);

CREATE INDEX idx_refunds_stay ON refunds(stay_id);
CREATE INDEX idx_refunds_payment ON refunds(payment_id);
CREATE INDEX idx_refunds_refunded_at ON refunds(refunded_at);

CREATE TABLE refund_components (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    refund_id    INTEGER NOT NULL REFERENCES refunds(id) ON DELETE RESTRICT,
    method       TEXT    NOT NULL CHECK (method IN ('cash', 'pix', 'debit', 'credit', 'transfer', 'unspecified')),
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    UNIQUE (refund_id, method)
);

-- Dados anteriores ao Marco 5 não têm modalidade confiável para o sinal: ficam como 'unspecified' (decisão M5-5).
INSERT INTO payments (id, code, reservation_id, stay_id, kind, amount_cents, received_at, received_by, note)
SELECT id, code, reservation_id, stay_id,
       CASE WHEN stay_id IS NULL OR method = 'prior_deposit' THEN 'deposit' ELSE 'stay' END,
       amount_cents, received_at, received_by, note
  FROM payments_legacy;

INSERT INTO payment_components (payment_id, method, amount_cents)
SELECT id, CASE WHEN method = 'prior_deposit' THEN 'unspecified' ELSE method END, amount_cents
  FROM payments_legacy;

INSERT INTO refunds (id, code, payment_id, reservation_id, stay_id, amount_cents, reason, refunded_at, refunded_by)
SELECT r.id, r.code, r.payment_id, p.reservation_id, r.stay_id, r.amount_cents, r.reason, r.refunded_at, r.refunded_by
  FROM refunds_legacy r JOIN payments p ON p.id = r.payment_id;

INSERT INTO refund_components (refund_id, method, amount_cents)
SELECT id, 'unspecified', amount_cents FROM refunds_legacy;

DROP TABLE payments_legacy;
DROP TABLE refunds_legacy;

-- Despesas da pousada.
CREATE TABLE expense_categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    name_key   TEXT    NOT NULL UNIQUE,  -- nome sem acentos/caixa, para unicidade
    active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    version    INTEGER NOT NULL DEFAULT 1,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);

-- Valores provisórios (M5-13): lista inicial de categorias, editável pelo administrador.
INSERT INTO expense_categories (name, name_key, created_at, updated_at) VALUES
    ('Energia', 'energia', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Água', 'agua', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Internet e telefone', 'internet e telefone', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Alimentos e bebidas', 'alimentos e bebidas', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Limpeza e lavanderia', 'limpeza e lavanderia', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Manutenção', 'manutencao', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Pessoal', 'pessoal', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Impostos e taxas', 'impostos e taxas', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'),
    ('Outros', 'outros', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z');

CREATE TABLE payables (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    code             TEXT    NOT NULL UNIQUE,
    supplier         TEXT    NOT NULL,
    description      TEXT    NOT NULL,
    amount_cents     INTEGER NOT NULL CHECK (amount_cents > 0),
    competence_date  TEXT    NOT NULL,
    due_date         TEXT    NOT NULL,
    category_id      INTEGER NOT NULL REFERENCES expense_categories(id) ON DELETE RESTRICT,
    notes            TEXT    NOT NULL DEFAULT '',
    created_by       INTEGER NOT NULL REFERENCES users(id),
    version          INTEGER NOT NULL DEFAULT 1,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE INDEX idx_payables_due ON payables(due_date);
CREATE INDEX idx_payables_category ON payables(category_id);

CREATE TABLE payable_payments (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    payable_id   INTEGER NOT NULL REFERENCES payables(id) ON DELETE RESTRICT,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    method       TEXT    NOT NULL CHECK (method IN ('cash', 'pix', 'debit', 'credit', 'transfer')),
    paid_at      TEXT    NOT NULL,
    paid_by      INTEGER NOT NULL REFERENCES users(id),
    note         TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX idx_payable_payments_payable ON payable_payments(payable_id);
CREATE INDEX idx_payable_payments_paid_at ON payable_payments(paid_at);

-- Fechamento diário (um por data local); depois de gravado não é alterado.
CREATE TABLE cash_closings (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    closing_date TEXT    NOT NULL UNIQUE,
    closed_at    TEXT    NOT NULL,
    closed_by    INTEGER NOT NULL REFERENCES users(id)
);

CREATE TABLE cash_closing_lines (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    closing_id       INTEGER NOT NULL REFERENCES cash_closings(id) ON DELETE RESTRICT,
    method           TEXT    NOT NULL CHECK (method IN ('cash', 'pix', 'debit', 'credit', 'transfer', 'unspecified')),
    expected_cents   INTEGER NOT NULL,
    counted_cents    INTEGER NOT NULL CHECK (counted_cents >= 0),
    difference_cents INTEGER NOT NULL,
    justification    TEXT,
    UNIQUE (closing_id, method)
);
