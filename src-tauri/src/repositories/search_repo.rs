use rusqlite::{named_params, Connection};

use crate::domain::cpf;
use crate::error::AppError;
use crate::models::search::{Group, GuestHit, MovementHit, ReservationHit, StayHit};

/// Tudo o que a consulta precisa, já normalizado e com `%`, `_` e `\` escapados.
pub struct Terms<'a> {
    /// Texto original aparado (comparação exata de código).
    pub raw: &'a str,
    /// `%texto%` do código, em qualquer caixa.
    pub code: String,
    /// `%texto normalizado%` do nome (sem acento e sem caixa).
    pub name: String,
    /// `%dígitos%` para telefone e CPF; vazio quando há menos de 4 dígitos.
    pub digits: Option<String>,
    /// Nome normalizado inteiro, para pôr o nome exato primeiro.
    pub folded: String,
}

pub fn guests(conn: &Connection, t: &Terms, limit: i64) -> Result<Group<GuestHit>, AppError> {
    let filter = "(g.name_key LIKE :name ESCAPE '\\' OR g.code LIKE :code ESCAPE '\\'
        OR (:digits IS NOT NULL AND (g.phone LIKE :digits ESCAPE '\\' OR g.cpf LIKE :digits ESCAPE '\\')))";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) FROM guests g WHERE {filter}"),
        named_params! { ":name": t.name, ":code": t.code, ":digits": t.digits },
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT g.id, g.code, g.full_name, g.phone, g.cpf, g.active FROM guests g WHERE {filter}
         ORDER BY (g.code = upper(:raw)) DESC, (g.name_key = :folded) DESC, g.name_key, g.id LIMIT :limit"
    ))?;
    let items = stmt
        .query_map(
            named_params! { ":name": t.name, ":code": t.code, ":digits": t.digits, ":raw": t.raw, ":folded": t.folded, ":limit": limit },
            |r| {
                let digits: Option<String> = r.get(4)?;
                Ok(GuestHit {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    name: r.get(2)?,
                    phone: r.get(3)?,
                    cpf_masked: digits.map(|d| cpf::mask(&d)),
                    active: r.get::<_, i64>(5)? == 1,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Group { items, total })
}

pub fn reservations(conn: &Connection, t: &Terms, limit: i64) -> Result<Group<ReservationHit>, AppError> {
    let from = "FROM reservations r JOIN guests g ON g.id = r.guest_id JOIN rooms rm ON rm.id = r.room_id";
    let filter = "(r.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR rm.code = :raw COLLATE NOCASE)";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) {from} WHERE {filter}"),
        named_params! { ":name": t.name, ":code": t.code, ":raw": t.raw },
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT r.id, r.code, g.full_name, rm.code, r.check_in_date, r.check_out_date, r.status {from} WHERE {filter}
         ORDER BY (r.code = upper(:raw)) DESC, r.check_in_date DESC, r.id DESC LIMIT :limit"
    ))?;
    let items = stmt
        .query_map(named_params! { ":name": t.name, ":code": t.code, ":raw": t.raw, ":limit": limit }, |r| {
            Ok(ReservationHit {
                id: r.get(0)?,
                code: r.get(1)?,
                guest_name: r.get(2)?,
                room_code: r.get(3)?,
                check_in_date: r.get(4)?,
                check_out_date: r.get(5)?,
                status: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Group { items, total })
}

pub fn stays(conn: &Connection, t: &Terms, limit: i64) -> Result<Group<StayHit>, AppError> {
    let from = "FROM stays s JOIN reservations r ON r.id = s.reservation_id JOIN guests g ON g.id = r.guest_id JOIN rooms rm ON rm.id = r.room_id";
    let filter = "(s.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\' OR rm.code = :raw COLLATE NOCASE)";
    let total: i64 = conn.query_row(
        &format!("SELECT count(*) {from} WHERE {filter}"),
        named_params! { ":name": t.name, ":code": t.code, ":raw": t.raw },
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        "SELECT s.id, s.code, g.full_name, rm.code, s.status {from} WHERE {filter}
         ORDER BY (s.code = upper(:raw)) DESC, (s.status = 'in_house') DESC, s.id DESC LIMIT :limit"
    ))?;
    let items = stmt
        .query_map(named_params! { ":name": t.name, ":code": t.code, ":raw": t.raw, ":limit": limit }, |r| {
            Ok(StayHit { id: r.get(0)?, code: r.get(1)?, guest_name: r.get(2)?, room_code: r.get(3)?, status: r.get(4)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Group { items, total })
}

/// Recebimentos, devoluções e (só com permissão) contas a pagar.
pub fn movements(conn: &Connection, t: &Terms, with_payables: bool, limit: i64) -> Result<Group<MovementHit>, AppError> {
    let payments = "SELECT 'payment' AS kind, p.code AS code, 'Recebimento ' || p.code || ' · ' || g.full_name AS label, p.amount_cents AS amount,
            p.received_at AS at, CASE WHEN p.stay_id IS NOT NULL THEN '/hospedagens/' || p.stay_id ELSE '/reservas/' || p.reservation_id END AS link
         FROM payments p JOIN reservations r ON r.id = p.reservation_id JOIN guests g ON g.id = r.guest_id
         WHERE p.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\'";
    let refunds = "SELECT 'refund', f.code, 'Devolução ' || f.code || ' · ' || g.full_name, f.amount_cents, f.refunded_at,
            CASE WHEN f.stay_id IS NOT NULL THEN '/hospedagens/' || f.stay_id ELSE '/reservas/' || f.reservation_id END
         FROM refunds f JOIN reservations r ON r.id = f.reservation_id JOIN guests g ON g.id = r.guest_id
         WHERE f.code LIKE :code ESCAPE '\\' OR g.name_key LIKE :name ESCAPE '\\'";
    let payables = "SELECT 'payable', b.code, 'Conta a pagar ' || b.code || ' · ' || b.supplier, b.amount_cents, b.created_at, '/contas-a-pagar'
         FROM payables b WHERE b.code LIKE :code ESCAPE '\\' OR b.supplier LIKE :text ESCAPE '\\' OR b.description LIKE :text ESCAPE '\\'";
    let union = if with_payables { format!("{payments} UNION ALL {refunds} UNION ALL {payables}") } else { format!("{payments} UNION ALL {refunds}") };
    // O SQLite recusa parâmetro nomeado que a consulta não usa: monta só os que valem.
    let text = t.code.clone();
    let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = vec![(":name", &t.name), (":code", &t.code)];
    if with_payables {
        params.push((":text", &text));
    }
    let total: i64 = conn.query_row(&format!("SELECT count(*) FROM ({union})"), params.as_slice(), |r| r.get(0))?;
    let mut stmt = conn.prepare(&format!(
        "SELECT kind, code, label, amount, at, link FROM ({union}) ORDER BY (code = upper(:raw)) DESC, at DESC, code DESC LIMIT :limit"
    ))?;
    params.push((":raw", &t.raw));
    params.push((":limit", &limit));
    let items = stmt
        .query_map(params.as_slice(), |r| {
            Ok(MovementHit { kind: r.get(0)?, code: r.get(1)?, label: r.get(2)?, amount_cents: r.get(3)?, at: r.get(4)?, link: r.get(5)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Group { items, total })
}
