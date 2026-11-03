//! Escritor de PDF mínimo (A4, Helvetica embutida, codificação WinAnsi): texto, tabelas com quebra de
//! página e rodapé "Página N de M". Sem dependências; o conteúdo não é comprimido, o que mantém os
//! testes simples. Serve relatórios (§12.3) e documentos internos (§17).

const MARGIN: f32 = 40.0;
const PORTRAIT: (f32, f32) = (595.0, 842.0);
const LANDSCAPE: (f32, f32) = (842.0, 595.0);
const FOOTER_SPACE: f32 = 28.0;
const CELL_PAD: f32 = 3.0;

/// Larguras (milésimos de em) da Helvetica para os caracteres ASCII 32 a 126.
const WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584,
    584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611,
    278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722,
    500, 500, 500, 334, 260, 334, 584,
];

#[derive(Debug, Clone)]
pub struct PdfColumn {
    pub label: String,
    /// Largura relativa da coluna.
    pub weight: u32,
    /// Alinhado à direita (números e valores).
    pub right: bool,
}

/// Código WinAnsi de um caractere; o que não existe vira `?`.
fn winansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '\u{00A0}'..='\u{00FF}' => c as u32 as u8,
        '€' => 0x80,
        '…' => 0x85,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        _ => b'?',
    }
}

/// Largura estimada do texto em pontos (a negrito é um pouco mais larga).
fn text_width(text: &str, size: f32, bold: bool) -> f32 {
    let units: f32 = text
        .chars()
        .map(|c| match c {
            ' '..='~' => f32::from(WIDTHS[(c as u8 - b' ') as usize]),
            '…' => 1000.0,
            _ => 600.0,
        })
        .sum();
    units * size / 1000.0 * if bold { 1.1 } else { 1.0 }
}

/// Texto como literal de PDF: bytes WinAnsi, com `(`, `)`, `\` e tudo fora do ASCII visível em octal.
fn literal(text: &str) -> String {
    let mut out = String::from("(");
    for c in text.chars() {
        match winansi(c) {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            b @ 32..=126 => out.push(b as char),
            b => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out.push(')');
    out
}

fn fit(text: &str, max: f32, size: f32, bold: bool) -> String {
    if text_width(text, size, bold) <= max {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if text_width(&candidate, size, bold) <= max {
            return candidate;
        }
    }
    String::new()
}

pub struct PdfDoc {
    width: f32,
    height: f32,
    pages: Vec<String>,
    y: f32,
}

impl PdfDoc {
    pub fn new(landscape: bool) -> Self {
        let (width, height) = if landscape { LANDSCAPE } else { PORTRAIT };
        Self { width, height, pages: vec![String::new()], y: MARGIN }
    }

    fn content_width(&self) -> f32 {
        self.width - 2.0 * MARGIN
    }

    fn page(&mut self) -> &mut String {
        self.pages.last_mut().expect("sempre há uma página")
    }

    fn new_page(&mut self) {
        self.pages.push(String::new());
        self.y = MARGIN;
    }

    /// Garante `needed` pontos livres na página; senão abre outra. Devolve se abriu.
    fn ensure(&mut self, needed: f32) -> bool {
        if self.y + needed > self.height - MARGIN - FOOTER_SPACE {
            self.new_page();
            true
        } else {
            false
        }
    }

    fn put(&mut self, x: f32, baseline_from_top: f32, size: f32, bold: bool, text: &str) {
        let y = self.height - baseline_from_top;
        let font = if bold { "F2" } else { "F1" };
        let op = format!("BT /{font} {size} Tf {x:.2} {y:.2} Td {} Tj ET\n", literal(text));
        self.page().push_str(&op);
    }

    pub fn spacer(&mut self, points: f32) {
        self.y += points;
    }

    fn write_lines(&mut self, text: &str, size: f32, bold: bool) {
        let max = self.content_width();
        let mut line = String::new();
        let mut lines = Vec::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text_width(&candidate, size, bold) <= max || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        if !line.is_empty() || lines.is_empty() {
            lines.push(line);
        }
        let leading = size + 4.0;
        for l in lines {
            self.ensure(leading);
            self.y += leading;
            let shown = fit(&l, max, size, bold);
            self.put(MARGIN, self.y - 3.0, size, bold, &shown);
        }
    }

    pub fn heading(&mut self, text: &str) {
        self.write_lines(text, 15.0, true);
    }

    pub fn subheading(&mut self, text: &str) {
        self.write_lines(text, 11.0, true);
    }

    pub fn text(&mut self, text: &str) {
        self.write_lines(text, 10.0, false);
    }

    pub fn bold_text(&mut self, text: &str) {
        self.write_lines(text, 10.0, true);
    }

    /// Duas colunas "rótulo: valor" numa linha só (fichas e comprovantes).
    pub fn field(&mut self, label: &str, value: &str) {
        self.ensure(14.0);
        self.y += 14.0;
        let label_text = format!("{label}:");
        let label_width = text_width(&label_text, 10.0, true) + 6.0;
        self.put(MARGIN, self.y - 3.0, 10.0, true, &label_text);
        let room = self.content_width() - label_width;
        self.put(MARGIN + label_width, self.y - 3.0, 10.0, false, &fit(value, room, 10.0, false));
    }

    pub fn signature_line(&mut self, label: &str) {
        self.ensure(50.0);
        self.y += 36.0;
        let (x1, x2) = (MARGIN, MARGIN + 240.0);
        let y = self.height - self.y;
        self.page().push_str(&format!("0.4 G 0.6 w {x1:.2} {y:.2} m {x2:.2} {y:.2} l S\n"));
        self.y += 12.0;
        self.put(MARGIN, self.y - 2.0, 9.0, false, label);
    }

    pub fn table(&mut self, columns: &[PdfColumn], rows: &[Vec<String>], totals: Option<&[String]>) {
        if rows.is_empty() {
            self.text("Nenhum registro no período.");
            return;
        }
        let size = if columns.len() > 7 { 8.0 } else { 9.0 };
        let row_h = size + 7.0;
        let sum: f32 = columns.iter().map(|c| c.weight as f32).sum::<f32>().max(1.0);
        let widths: Vec<f32> = columns.iter().map(|c| self.content_width() * c.weight as f32 / sum).collect();

        self.header_row(columns, &widths, size, row_h);
        for row in rows.iter().map(|r| r.as_slice()).chain(totals) {
            let is_total = totals.is_some_and(|t| std::ptr::eq(t, row));
            if self.ensure(row_h) {
                self.header_row(columns, &widths, size, row_h);
            }
            self.row(columns, &widths, row, size, row_h, is_total);
        }
    }

    fn header_row(&mut self, columns: &[PdfColumn], widths: &[f32], size: f32, row_h: f32) {
        self.ensure(row_h * 2.0);
        let top = self.y;
        let y = self.height - top - row_h;
        let w = self.content_width();
        self.page().push_str(&format!("0.90 g {MARGIN:.2} {y:.2} {w:.2} {row_h:.2} re f 0 g\n"));
        let mut x = MARGIN;
        for (c, cw) in columns.iter().zip(widths) {
            let shown = fit(&c.label, cw - 2.0 * CELL_PAD, size, true);
            let tx = if c.right { x + cw - CELL_PAD - text_width(&shown, size, true) } else { x + CELL_PAD };
            self.put(tx, top + row_h - 4.0, size, true, &shown);
            x += cw;
        }
        self.y += row_h;
    }

    fn row(&mut self, columns: &[PdfColumn], widths: &[f32], cells: &[String], size: f32, row_h: f32, bold: bool) {
        let top = self.y;
        let mut x = MARGIN;
        for ((c, cw), cell) in columns.iter().zip(widths).zip(cells) {
            let shown = fit(cell, cw - 2.0 * CELL_PAD, size, bold);
            let tx = if c.right { x + cw - CELL_PAD - text_width(&shown, size, bold) } else { x + CELL_PAD };
            self.put(tx, top + row_h - 4.0, size, bold, &shown);
            x += cw;
        }
        let y = self.height - top - row_h;
        let w = self.content_width();
        self.page().push_str(&format!("0.8 G 0.4 w {MARGIN:.2} {y:.2} m {:.2} {y:.2} l S 0 G\n", MARGIN + w));
        self.y += row_h;
    }

    /// Fecha o documento: rodapé "texto · Página N de M" em cada página e as tabelas de objetos do PDF.
    pub fn finish(mut self, footer: &str) -> Vec<u8> {
        let total = self.pages.len();
        for i in 0..total {
            let text = format!("{footer}  ·  Página {} de {total}", i + 1);
            let y = MARGIN / 2.0 + 4.0;
            let op = format!("BT /F1 8 Tf {MARGIN:.2} {y:.2} Td {} Tj ET\n", literal(&text));
            self.pages[i].push_str(&op);
        }

        let n_pages = self.pages.len();
        let mut objects: Vec<String> = Vec::new();
        objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
        let kids: Vec<String> = (0..n_pages).map(|i| format!("{} 0 R", 5 + 2 * i)).collect();
        objects.push(format!("<< /Type /Pages /Kids [{}] /Count {n_pages} >>", kids.join(" ")));
        objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string());
        objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_string());
        for (i, content) in self.pages.iter().enumerate() {
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                self.width as i32,
                self.height as i32,
                6 + 2 * i
            ));
            objects.push(format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content));
        }

        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
        out
    }
}

/// Textos desenhados no PDF (uma string por operador `Tj`), decodificando os octais para Latin-1.
#[cfg(test)]
pub fn extract_text(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf).to_string();
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(start) = line.find(" Td (") else { continue };
        let body = &line[start + 5..];
        let mut s = String::new();
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some(d) if d.is_ascii_digit() => {
                        let mut oct = d.to_string();
                        for _ in 0..2 {
                            if let Some(n) = chars.peek().copied().filter(char::is_ascii_digit) {
                                oct.push(n);
                                chars.next();
                            }
                        }
                        s.push(char::from(u8::from_str_radix(&oct, 8).unwrap_or(b'?')));
                    }
                    Some(other) => s.push(other),
                    None => {}
                },
                ')' => break,
                other => s.push(other),
            }
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols() -> Vec<PdfColumn> {
        vec![
            PdfColumn { label: "Nome".into(), weight: 30, right: false },
            PdfColumn { label: "Valor".into(), weight: 15, right: true },
        ]
    }

    fn rows(n: usize) -> Vec<Vec<String>> {
        (0..n).map(|i| vec![format!("Pessoa {i}"), format!("R$ {i},00")]).collect()
    }

    fn count(haystack: &[u8], needle: &str) -> usize {
        String::from_utf8_lossy(haystack).matches(needle).count()
    }

    #[test]
    fn has_a_valid_structure_with_a_consistent_xref() {
        let mut doc = PdfDoc::new(false);
        doc.heading("Título");
        doc.table(&cols(), &rows(3), None);
        let pdf = doc.finish("Boreal");

        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(text.starts_with("%PDF-1.4\n") && text.trim_end().ends_with("%%EOF"));
        let xref_at: usize = text.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert!(text[xref_at..].starts_with("xref\n"), "startxref aponta para a tabela");
        let entries: Vec<&str> = text[xref_at..].lines().skip(3).take_while(|l| *l != "trailer").collect();
        let objects = count(&pdf, " 0 obj\n");
        assert_eq!(entries.len(), objects, "um deslocamento por objeto");
        for (i, entry) in entries.iter().enumerate() {
            let offset: usize = entry[..10].parse().unwrap();
            assert!(text[offset..].starts_with(&format!("{} 0 obj", i + 1)), "objeto {} no deslocamento {offset}", i + 1);
        }
    }

    #[test]
    fn page_count_matches_the_page_objects() {
        let mut doc = PdfDoc::new(false);
        doc.table(&cols(), &rows(120), None);
        let pdf = doc.finish("Boreal");

        let pages = count(&pdf, "/Type /Page ");
        assert!(pages >= 3, "{pages}");
        assert_eq!(count(&pdf, &format!("/Count {pages} ")), 1);
    }

    #[test]
    fn table_header_repeats_on_each_page_and_the_footer_numbers_the_pages() {
        let mut doc = PdfDoc::new(false);
        doc.table(&cols(), &rows(120), None);
        let pdf = doc.finish("Boreal PMS");

        let texts = extract_text(&pdf);
        let pages = count(&pdf, "/Type /Page ");
        assert_eq!(texts.iter().filter(|t| *t == "Nome").count(), pages, "cabeçalho em cada página");
        assert!(texts.contains(&format!("Boreal PMS  ·  Página 1 de {pages}")));
        assert!(texts.contains(&format!("Boreal PMS  ·  Página {pages} de {pages}")));
        assert!(texts.contains(&"Pessoa 119".to_string()), "nenhuma linha se perde");
    }

    #[test]
    fn accents_are_winansi_and_special_characters_are_escaped() {
        let mut doc = PdfDoc::new(false);
        doc.text("Confirmação (reserva) a\\b");
        let pdf = doc.finish("x");

        let raw = String::from_utf8_lossy(&pdf).to_string();
        assert!(raw.contains("Confirma\\347\\343o \\(reserva\\) a\\\\b"), "{raw}");
        assert!(extract_text(&pdf).contains(&"Confirmação (reserva) a\\b".to_string()));
        assert!(raw.contains("/Encoding /WinAnsiEncoding"));
    }

    #[test]
    fn text_wider_than_the_column_is_truncated_with_an_ellipsis() {
        let mut doc = PdfDoc::new(false);
        let long = "Nome muito comprido que nunca caberia na coluna do relatório ".repeat(4);
        doc.table(&cols(), &[vec![long.clone(), "1".into()]], None);
        let pdf = doc.finish("x");

        let texts = extract_text(&pdf);
        assert!(!texts.contains(&long));
        let shown = texts.iter().find(|t| t.starts_with("Nome muito")).expect("texto truncado presente");
        // O extrator decodifica o byte WinAnsi 0x85 (reticências) como U+0085.
        assert!(shown.ends_with('\u{85}') && shown.chars().count() < long.chars().count(), "{shown}");
    }

    #[test]
    fn landscape_pages_use_the_wide_media_box() {
        let doc = PdfDoc::new(true);
        let pdf = doc.finish("x");

        assert!(String::from_utf8_lossy(&pdf).contains("/MediaBox [0 0 842 595]"));
    }

    #[test]
    fn an_empty_table_still_produces_a_valid_pdf_with_a_message() {
        let mut doc = PdfDoc::new(false);
        doc.table(&cols(), &[], None);
        let pdf = doc.finish("x");

        assert!(extract_text(&pdf).contains(&"Nenhum registro no período.".to_string()));
        assert!(String::from_utf8_lossy(&pdf).trim_end().ends_with("%%EOF"));
    }

    #[test]
    fn totals_are_bold_and_come_after_the_rows() {
        let mut doc = PdfDoc::new(false);
        doc.table(&cols(), &rows(2), Some(&["Total".to_string(), "R$ 1,00".to_string()]));
        let pdf = doc.finish("x");

        let texts = extract_text(&pdf);
        let total_at = texts.iter().position(|t| t == "Total").unwrap();
        let last_row_at = texts.iter().position(|t| t == "Pessoa 1").unwrap();
        assert!(total_at > last_row_at);
        assert!(String::from_utf8_lossy(&pdf).contains("/F2 9 Tf"), "negrito para cabeçalho e totais");
    }

    #[test]
    fn long_paragraphs_wrap_instead_of_leaving_the_page() {
        let mut doc = PdfDoc::new(false);
        doc.text(&"palavra ".repeat(80));
        let pdf = doc.finish("x");

        let lines = extract_text(&pdf).into_iter().filter(|t| t.starts_with("palavra")).count();
        assert!(lines >= 3, "{lines}");
    }
}
