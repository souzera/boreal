CREATE TABLE sequences (
    name       TEXT    PRIMARY KEY,
    next_value INTEGER NOT NULL CHECK (next_value >= 1)
);

INSERT INTO sequences (name, next_value) VALUES
    ('guest', 1),
    ('reservation', 1),
    ('stay', 1),
    ('payment', 1),
    ('refund', 1);

CREATE TABLE user_permissions (
    user_id    INTEGER NOT NULL REFERENCES users(id),
    permission TEXT    NOT NULL,
    granted_by INTEGER REFERENCES users(id),
    granted_at TEXT    NOT NULL,
    PRIMARY KEY (user_id, permission)
);
