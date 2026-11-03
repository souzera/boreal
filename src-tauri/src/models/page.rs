use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

/// Normaliza paginação vinda do cliente: página a partir de 1, tamanho de 1 a 100.
pub fn clamp_paging(page: u32, page_size: u32) -> (u32, u32, i64) {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 100);
    (page, page_size, i64::from(page - 1) * i64::from(page_size))
}

/// Escapa `%`, `_` e `\` para uso em `LIKE ... ESCAPE '\'`.
pub fn like_pattern(text: &str) -> String {
    let escaped = text
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
