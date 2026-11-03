use unicode_normalization::UnicodeNormalization;

/// Chave de comparação de nomes: minúsculas, sem acentos e com espaços colapsados.
pub fn fold(input: &str) -> String {
    let stripped: String = input
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .collect();
    stripped
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn lowercases_strips_accents_and_collapses_spaces() {
        assert_eq!(fold("  José  da  SILVA "), "jose da silva");
        assert_eq!(fold("Conceição"), "conceicao");
        assert_eq!(fold("ÀÉÎÕÜ"), "aeiou");
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(fold("   "), "");
    }
}
