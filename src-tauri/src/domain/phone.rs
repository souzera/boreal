/// Telefone brasileiro só com dígitos (DDD + número, 10 ou 11 dígitos).
/// Remove o código do país (55) quando presente.
pub fn normalize(input: &str) -> Option<String> {
    let mut digits: String = input.chars().filter(char::is_ascii_digit).collect();
    if matches!(digits.len(), 12 | 13) && digits.starts_with("55") {
        digits.drain(..2);
    }
    matches!(digits.len(), 10 | 11).then_some(digits)
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn keeps_only_digits() {
        assert_eq!(normalize("(11) 99999-0000").as_deref(), Some("11999990000"));
        assert_eq!(normalize("11 3333-4444").as_deref(), Some("1133334444"));
    }

    #[test]
    fn drops_country_code() {
        assert_eq!(normalize("+55 11 99999-0000").as_deref(), Some("11999990000"));
        assert_eq!(normalize("55 11 3333-4444").as_deref(), Some("1133334444"));
    }

    #[test]
    fn rejects_short_long_or_empty() {
        for bad in ["123", "", "abc", "119999900001234", "(11) 9999"] {
            assert_eq!(normalize(bad), None, "{bad} deveria ser inválido");
        }
    }
}
