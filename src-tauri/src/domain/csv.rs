//! Renderizador CSV de uma `ReportTable` (spec §12.3): UTF-8 com BOM, aspas duplas, fim de linha `\r\n`.

use super::report::{Cell, ReportTable};

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub const TRUNCATED_NOTE: &str = "Atenção: mostrando apenas as primeiras linhas. Refine os filtros para ver o resto.";

/// Casas decimais com vírgula quando o separador é `;` e com ponto quando é `,`.
fn decimal(delimiter: char, whole: i64, fraction: String) -> String {
    let point = if delimiter == ';' { ',' } else { '.' };
    format!("{whole}{point}{fraction}")
}

fn cell_value(cell: &Cell, delimiter: char) -> String {
    match cell {
        Cell::Text(text) => guard_formula(text),
        Cell::Int(n) => n.to_string(),
        Cell::Money(cents) => {
            let sign = if *cents < 0 { "-" } else { "" };
            let abs = cents.unsigned_abs() as i64;
            format!("{sign}{}", decimal(delimiter, abs / 100, format!("{:02}", abs % 100)))
        }
        Cell::Percent(bp) => {
            let sign = if *bp < 0 { "-" } else { "" };
            let tenths = (bp.unsigned_abs() as i64 + 5) / 10;
            format!("{sign}{}", decimal(delimiter, tenths / 10, (tenths % 10).to_string()))
        }
        Cell::Empty => String::new(),
    }
}

/// Texto que começa como fórmula (`=`, `+`, `-`, `@`) ganha um apóstrofo, para o Excel não executá-lo.
fn guard_formula(text: &str) -> String {
    match text.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{text}"),
        _ => text.to_string(),
    }
}

fn escape(field: &str, delimiter: char) -> String {
    if field.contains(delimiter) || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn line(fields: impl IntoIterator<Item = String>, delimiter: char) -> String {
    let joined: Vec<String> = fields.into_iter().map(|f| escape(&f, delimiter)).collect();
    format!("{}\r\n", joined.join(&delimiter.to_string()))
}

/// Cabeçalho, linhas, totais (se houver) e, quando truncada, a linha de aviso.
pub fn render(table: &ReportTable, delimiter: char) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(&line(table.columns.iter().map(|c| c.label.clone()), delimiter));
    for row in &table.rows {
        out.push_str(&line(row.iter().map(|c| cell_value(c, delimiter)), delimiter));
    }
    if let Some(totals) = &table.totals {
        out.push_str(&line(totals.iter().map(|c| cell_value(c, delimiter)), delimiter));
    }
    if table.truncated {
        out.push_str(&line([TRUNCATED_NOTE.to_string()], delimiter));
    }
    let mut bytes = BOM.to_vec();
    bytes.extend_from_slice(out.as_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::report::{Column, ColumnKind};

    fn table(rows: Vec<Vec<Cell>>) -> ReportTable {
        ReportTable {
            kind: "x".into(),
            title: "X".into(),
            filters: vec![],
            columns: vec![Column::new("a", "Nome", ColumnKind::Text), Column::new("b", "Valor", ColumnKind::Money)],
            rows,
            totals: None,
            truncated: false,
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8(bytes[3..].to_vec()).unwrap()
    }

    #[test]
    fn starts_with_the_bom_and_uses_semicolon_and_crlf() {
        let bytes = render(&table(vec![vec![Cell::text("Ana"), Cell::Money(15_000)]]), ';');

        assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
        assert_eq!(text(&bytes), "Nome;Valor\r\nAna;150,00\r\n");
    }

    #[test]
    fn comma_delimiter_uses_a_decimal_point() {
        let bytes = render(&table(vec![vec![Cell::text("Ana"), Cell::Money(-3_000)]]), ',');

        assert_eq!(text(&bytes), "Nome,Valor\r\nAna,-30.00\r\n");
    }

    #[test]
    fn quotes_fields_with_delimiter_quotes_and_line_breaks() {
        let bytes = render(&table(vec![vec![Cell::text("Souza; Ana \"Nana\"\nCasa"), Cell::Int(2)]]), ';');

        assert_eq!(text(&bytes), "Nome;Valor\r\n\"Souza; Ana \"\"Nana\"\"\nCasa\";2\r\n");
    }

    #[test]
    fn formula_looking_text_gets_an_apostrophe_but_numbers_do_not() {
        let rows = vec![
            vec![Cell::text("=SOMA(A1)"), Cell::Money(-500)],
            vec![Cell::text("+55 11 99999"), Cell::Money(0)],
            vec![Cell::text("-"), Cell::Money(1)],
            vec![Cell::text("@x"), Cell::Money(1)],
            vec![Cell::text("Normal"), Cell::Money(1)],
        ];

        let out = text(&render(&table(rows), ';'));

        assert!(out.contains("\r\n'=SOMA(A1);-5,00\r\n"), "{out}");
        assert!(out.contains("'+55 11 99999;0,00"), "{out}");
        assert!(out.contains("\r\n'-;0,01"), "{out}");
        assert!(out.contains("'@x;0,01"), "{out}");
        assert!(out.contains("\r\nNormal;0,01"), "{out}");
    }

    #[test]
    fn percent_dates_and_empty_cells() {
        let mut t = table(vec![vec![Cell::text("21/10/2026"), Cell::Percent(7_500)], vec![Cell::Empty, Cell::Percent(3_333)]]);
        t.columns[1].kind = ColumnKind::Percent;

        assert_eq!(text(&render(&t, ';')), "Nome;Valor\r\n21/10/2026;75,0\r\n;33,3\r\n");
    }

    #[test]
    fn totals_are_the_last_row_and_truncation_adds_a_note() {
        let mut t = table(vec![vec![Cell::text("Ana"), Cell::Money(100)]]);
        t.totals = Some(vec![Cell::text("Total"), Cell::Money(100)]);
        t.truncated = true;

        let out = text(&render(&t, ';'));

        assert!(out.ends_with(&format!("Total;1,00\r\n{TRUNCATED_NOTE}\r\n")), "{out}");
    }
}
