//! Regras de quarto compartilhadas pelos serviços: intervalo de limpeza efetivo e portão "quarto limpo".

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use super::reservation_service::br;
use crate::error::AppError;
use crate::models::block::RoomBlock;
use crate::models::settings::AppSettings;
use crate::repositories::{block_repo, catalog_repo, cleaning_repo};

/// Intervalo mínimo de limpeza do quarto: o próprio, se houver, senão o padrão das configurações.
pub(crate) fn cleaning_minutes(conn: &Connection, room_id: i64, settings: &AppSettings) -> Result<u32, AppError> {
    let own = catalog_repo::find_room(conn, room_id)?.and_then(|r| r.cleaning_interval_minutes);
    Ok(own.and_then(|m| u32::try_from(m).ok()).unwrap_or(settings.cleaning_interval_minutes))
}

/// Recusa o check-in/walk-in se o quarto tem limpeza aberta ou se a última limpeza concluída ainda não
/// cumpriu o intervalo mínimo (spec §7). Quarto sem histórico de limpeza está livre.
/// `offset_seconds` converte os instantes UTC gravados para o horário local de `now`.
pub(crate) fn ensure_room_clean(
    conn: &Connection,
    room_id: i64,
    room_code: &str,
    interval_minutes: u32,
    now: NaiveDateTime,
    offset_seconds: i64,
) -> Result<(), AppError> {
    if let Some(open) = cleaning_repo::open_for_room(conn, room_id)? {
        let state = if open.status == "in_progress" { "em limpeza" } else { "aguardando limpeza" };
        return Err(AppError::Validation(format!(
            "O quarto {room_code} está {state}. Conclua a limpeza antes de hospedar."
        )));
    }
    let Some(done) = cleaning_repo::last_done_for_room(conn, room_id)? else {
        return Ok(());
    };
    let completed = done
        .completed_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.naive_utc() + Duration::seconds(offset_seconds))
        .ok_or_else(|| AppError::Database("limpeza concluída sem data".into()))?;
    let free_at = completed + Duration::minutes(i64::from(interval_minutes));
    if free_at > now {
        return Err(AppError::Validation(format!(
            "O quarto {room_code} foi limpo, mas o intervalo mínimo de limpeza só termina às {} de {}.",
            free_at.format("%H:%M"),
            br(&free_at.format("%Y-%m-%d").to_string()),
        )));
    }
    Ok(())
}

/// Texto para a recepção: o que bloqueia o quarto e por quê.
pub(crate) fn blocks_message(blocks: &[RoomBlock]) -> String {
    let list = blocks
        .iter()
        .map(|b| {
            let period = match &b.end_date {
                Some(end) => format!("de {} a {}", br(&b.start_date), br(end)),
                None => format!("a partir de {} até ser liberado", br(&b.start_date)),
            };
            let kind = if b.status == "preventive" { "tem bloqueio preventivo" } else { "está bloqueado" };
            format!("{} {kind} {period} ({})", b.room_code, b.reason)
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("O quarto {list}.")
}

/// Recusa o período se algum bloqueio o toca. `effective_only` ignora os preventivos (que só impedem
/// novas reservas — uma reserva já existente ainda pode fazer o check-in).
pub(crate) fn ensure_no_blocks(
    conn: &Connection,
    room_id: i64,
    start: NaiveDate,
    end: Option<NaiveDate>,
    effective_only: bool,
) -> Result<(), AppError> {
    let blocks = block_repo::overlapping(conn, room_id, start, end, effective_only)?;
    if blocks.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(blocks_message(&blocks)))
    }
}
