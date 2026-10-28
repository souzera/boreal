//! Banco em memória com usuário, quartos e hospedagens mínimas para testar repositórios.

use rusqlite::Connection;

use crate::migrations::run_migrations;

const T: &str = "2026-10-21T12:00:00Z";

pub fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    run_migrations(&mut conn).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(&format!(
        "INSERT INTO users (id, username, display_name, password_hash, role, active, must_change_password, created_at, updated_at)
           VALUES (1, 'u', 'U', 'x', 'admin', 1, 0, '{T}', '{T}');
         INSERT INTO room_categories (id, name, description, active, version, created_at, updated_at)
           VALUES (1, 'Standard', '', 1, 1, '{T}', '{T}');
         INSERT INTO rooms (id, code, category_id, max_capacity, description, notes, active, version, created_at, updated_at)
           VALUES (1, '101', 1, 2, '', '', 1, 1, '{T}', '{T}'), (2, '102', 1, 2, '', '', 1, 1, '{T}', '{T}');
         INSERT INTO guests (id, code, full_name, name_key, phone, email, address, active, version, created_at, updated_at)
           VALUES (1, 'HOS-1', 'Ana', 'ana', '11999990000', '', '', 1, 1, '{T}', '{T}');"
    ))
    .unwrap();
    conn
}

/// Reserva `id` no quarto indicado (entre as datas) com status dado.
pub fn reservation(conn: &Connection, id: i64, room: i64, from: &str, to: &str, status: &str) {
    conn.execute(
        "INSERT INTO reservations (id, code, room_id, guest_id, check_in_date, check_out_date, expected_check_in_time,
             expected_check_out_time, occupants, status, list_total_cents, agreed_total_cents, deposit_expected_cents,
             deposit_due_date, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, ?4, ?5, '14:00', '12:00', 1, ?6, 40000, 40000, 12000, ?4, 1, ?7, ?7)",
        rusqlite::params![id, format!("RES-{id:06}"), room, from, to, status, T],
    )
    .unwrap();
}

/// Hospedagem `id` ligada à reserva de mesmo número.
pub fn stay(conn: &Connection, id: i64) {
    conn.execute(
        "INSERT INTO stays (id, reservation_id, code, status, checked_in_at, checked_in_by, created_at, updated_at)
         VALUES (?1, ?1, ?2, 'in_house', ?3, 1, ?3, ?3)",
        rusqlite::params![id, format!("HOS-{id:06}"), T],
    )
    .unwrap();
}
