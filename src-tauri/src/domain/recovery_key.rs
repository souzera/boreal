//! Chave de recuperação (spec §14): a chave do banco em texto para guardar fora do computador.
//! Formato: 8 grupos de 8 caracteres hexadecimais mais um grupo de conferência de 4 (CRC-16), em maiúsculas.

use crate::error::AppError;

const KEY_BYTES: usize = 32;

/// CRC-16/CCITT-FALSE (polinômio 0x1021, início 0xFFFF).
fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// `XXXXXXXX-XXXXXXXX-…-CCCC`.
pub fn format(key: &[u8]) -> String {
    let hex: String = key.iter().map(|b| format!("{b:02X}")).collect();
    let mut groups: Vec<String> = hex.as_bytes().chunks(8).map(|c| String::from_utf8_lossy(c).to_string()).collect();
    groups.push(format!("{:04X}", crc16(key)));
    groups.join("-")
}

/// Lê a chave digitada: ignora espaços, hífens e caixa, confere o tamanho e o dígito de conferência.
pub fn parse(text: &str) -> Result<Vec<u8>, AppError> {
    let clean: String = text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_uppercase();
    if clean.len() != KEY_BYTES * 2 + 4 {
        return Err(AppError::Validation("A chave de recuperação está incompleta.".into()));
    }
    if !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation("Há um erro de digitação na chave de recuperação.".into()));
    }
    let (body, check) = clean.split_at(KEY_BYTES * 2);
    let key: Vec<u8> = (0..KEY_BYTES).map(|i| u8::from_str_radix(&body[i * 2..i * 2 + 2], 16).unwrap_or(0)).collect();
    if format!("{:04X}", crc16(&key)) != check {
        return Err(AppError::Validation("Há um erro de digitação na chave de recuperação.".into()));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Vec<u8> {
        (0u8..32).collect()
    }

    #[test]
    fn formats_eight_groups_of_eight_plus_a_four_character_check() {
        let text = format(&key());

        let groups: Vec<&str> = text.split('-').collect();
        assert_eq!(groups.len(), 9);
        assert!(groups[..8].iter().all(|g| g.len() == 8) && groups[8].len() == 4, "{text}");
        assert_eq!(groups[0], "00010203");
        assert_eq!(text, text.to_uppercase());
    }

    #[test]
    fn round_trips_and_ignores_case_spaces_and_hyphens() {
        let text = format(&key());

        assert_eq!(parse(&text).unwrap(), key());
        assert_eq!(parse(&text.to_lowercase()).unwrap(), key());
        assert_eq!(parse(&text.replace('-', " ")).unwrap(), key());
        assert_eq!(parse(&text.replace('-', "")).unwrap(), key());
        assert_eq!(parse(&format!("  {text}\n")).unwrap(), key());
    }

    #[test]
    fn a_wrong_character_is_a_typing_error_and_a_short_key_is_incomplete() {
        let text = format(&key());
        let mut wrong: Vec<char> = text.chars().collect();
        wrong[3] = if wrong[3] == 'A' { 'B' } else { 'A' };
        let wrong: String = wrong.into_iter().collect();

        let typo = parse(&wrong).unwrap_err();
        let short = parse(&text[..40]).unwrap_err();
        let not_hex = parse(&text.replace('0', "Z")).unwrap_err();

        assert!(matches!(&typo, AppError::Validation(m) if m.contains("erro de digitação")), "{typo:?}");
        assert!(matches!(&short, AppError::Validation(m) if m.contains("incompleta")), "{short:?}");
        assert!(matches!(&not_hex, AppError::Validation(m) if m.contains("erro de digitação")), "{not_hex:?}");
        assert!(matches!(parse(""), Err(AppError::Validation(m)) if m.contains("incompleta")));
    }

    #[test]
    fn crc16_matches_the_standard_check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }
}
