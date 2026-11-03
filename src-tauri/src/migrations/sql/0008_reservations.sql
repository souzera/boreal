CREATE TABLE reservations (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    code                    TEXT    NOT NULL UNIQUE,
    room_id                 INTEGER NOT NULL REFERENCES rooms(id)  ON DELETE RESTRICT,
    guest_id                INTEGER NOT NULL REFERENCES guests(id) ON DELETE RESTRICT,
    check_in_date           TEXT    NOT NULL,
    check_out_date          TEXT    NOT NULL,
    expected_check_in_time  TEXT    NOT NULL,
    expected_check_out_time TEXT    NOT NULL,
    occupants               INTEGER NOT NULL CHECK (occupants >= 1),
    status                  TEXT    NOT NULL CHECK (status IN ('pending', 'confirmed', 'checked_in', 'completed', 'cancelled', 'no_show')),
    list_total_cents        INTEGER NOT NULL CHECK (list_total_cents >= 0),
    discount_cents          INTEGER NOT NULL DEFAULT 0 CHECK (discount_cents >= 0),
    discount_reason         TEXT,
    discount_applied_by     INTEGER REFERENCES users(id),
    agreed_total_cents      INTEGER NOT NULL CHECK (agreed_total_cents >= 0),
    deposit_expected_cents  INTEGER NOT NULL CHECK (deposit_expected_cents >= 0),
    deposit_paid_cents      INTEGER NOT NULL DEFAULT 0 CHECK (deposit_paid_cents >= 0),
    deposit_due_date        TEXT    NOT NULL,
    notes                   TEXT    NOT NULL DEFAULT '',
    cancelled_at            TEXT,
    cancellation_fee_cents  INTEGER,
    version                 INTEGER NOT NULL DEFAULT 1,
    created_by              INTEGER NOT NULL REFERENCES users(id),
    created_at              TEXT    NOT NULL,
    updated_at              TEXT    NOT NULL,
    CHECK (check_out_date > check_in_date)
);

CREATE INDEX idx_reservations_room_period ON reservations(room_id, check_in_date, check_out_date);
CREATE INDEX idx_reservations_status ON reservations(status);
CREATE INDEX idx_reservations_guest ON reservations(guest_id);

-- Preço de cada noite, congelado na reserva: mudar a tabela de tarifas não altera reservas existentes.
CREATE TABLE reservation_nights (
    reservation_id INTEGER NOT NULL REFERENCES reservations(id) ON DELETE CASCADE,
    date           TEXT    NOT NULL,
    amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),
    rule           TEXT    NOT NULL,
    PRIMARY KEY (reservation_id, date)
);

CREATE TABLE reservation_changes (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id     INTEGER NOT NULL REFERENCES reservations(id) ON DELETE CASCADE,
    changed_at         TEXT    NOT NULL,
    changed_by         INTEGER NOT NULL REFERENCES users(id),
    kind               TEXT    NOT NULL,
    before_json        TEXT    NOT NULL,
    after_json         TEXT    NOT NULL,
    price_before_cents INTEGER,
    price_after_cents  INTEGER,
    justification      TEXT
);

CREATE INDEX idx_reservation_changes_reservation ON reservation_changes(reservation_id);
