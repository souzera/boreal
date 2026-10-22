use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Configurações operacionais. Os valores iniciais são provisórios (pendência §25.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub max_failed_attempts: u32,
    pub lockout_minutes: u32,
    pub idle_lock_minutes: u32,
    pub default_check_in_time: String,
    pub default_check_out_time: String,
    pub late_checkout_tolerance_minutes: u32,
    pub no_show_tolerance_minutes: u32,
    pub cleaning_interval_minutes: u32,
}

#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub settings: AppSettings,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionPolicy {
    pub idle_lock_minutes: u32,
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

fn in_range(value: u32, min: u32, max: u32, message: &str) -> Result<(), AppError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(invalid(message))
    }
}

/// `HH:MM` com horas 00–23 e minutos 00–59, exatamente nesse formato.
fn valid_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 5
        && bytes[2] == b':'
        && value[..2].parse::<u32>().is_ok_and(|h| h < 24)
        && value[3..].parse::<u32>().is_ok_and(|m| m < 60)
        && bytes[..2].iter().chain(&bytes[3..]).all(u8::is_ascii_digit)
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), AppError> {
        in_range(self.max_failed_attempts, 3, 20, "Tentativas inválidas: informe de 3 a 20.")?;
        in_range(self.lockout_minutes, 1, 1440, "Bloqueio da conta: informe de 1 a 1440 minutos.")?;
        in_range(self.idle_lock_minutes, 1, 480, "Bloqueio por inatividade: informe de 1 a 480 minutos.")?;
        if !valid_time(&self.default_check_in_time) {
            return Err(invalid("Horário de check-in inválido. Use HH:MM."));
        }
        if !valid_time(&self.default_check_out_time) {
            return Err(invalid("Horário de check-out inválido. Use HH:MM."));
        }
        in_range(self.late_checkout_tolerance_minutes, 0, 600, "Tolerância de saída tardia: informe de 0 a 600 minutos.")?;
        in_range(self.no_show_tolerance_minutes, 0, 600, "Tolerância de no-show: informe de 0 a 600 minutos.")?;
        in_range(self.cleaning_interval_minutes, 0, 600, "Intervalo de limpeza: informe de 0 a 600 minutos.")?;
        Ok(())
    }
}
