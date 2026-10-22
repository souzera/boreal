/// Devolve o CPF só com os 11 dígitos se for válido (dígitos verificadores corretos e
/// não composto por um único dígito repetido). Aceita máscara e espaços; qualquer
/// outro caractere invalida.
pub fn normalize(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed
        .chars()
        .any(|c| !(c.is_ascii_digit() || matches!(c, '.' | '-' | ' ')))
    {
        return None;
    }
    let digits: Vec<u32> = trimmed.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 11 || digits.iter().all(|d| *d == digits[0]) {
        return None;
    }
    let check = |count: usize| -> u32 {
        let sum: u32 = digits[..count]
            .iter()
            .enumerate()
            .map(|(i, d)| d * (count as u32 + 1 - i as u32))
            .sum();
        let rest = (sum * 10) % 11;
        if rest == 10 {
            0
        } else {
            rest
        }
    };
    if check(9) != digits[9] || check(10) != digits[10] {
        return None;
    }
    Some(digits.iter().map(|d| char::from_digit(*d, 10).unwrap()).collect())
}

/// `***.***.***-25`: esconde tudo menos os dígitos verificadores (uso em auditoria e logs).
pub fn mask(cpf: &str) -> String {
    let tail: String = cpf.chars().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
    format!("***.***.***-{tail}")
}

#[cfg(test)]
mod tests {
    use super::{mask, normalize};

    #[test]
    fn accepts_valid_cpf_with_or_without_mask() {
        assert_eq!(normalize("529.982.247-25").as_deref(), Some("52998224725"));
        assert_eq!(normalize("52998224725").as_deref(), Some("52998224725"));
        assert_eq!(normalize(" 168.995.350-09 ").as_deref(), Some("16899535009"));
    }

    #[test]
    fn rejects_invalid_cpf() {
        for bad in ["111.111.111-11", "00000000000", "52998224724", "5299822472", "529982247250", "", "abc", "52a98224725"] {
            assert_eq!(normalize(bad), None, "{bad} deveria ser inválido");
        }
    }

    #[test]
    fn masks_all_but_the_check_digits() {
        assert_eq!(mask("52998224725"), "***.***.***-25");
    }
}
