//! Tipos e regras puras dos relatórios (spec §12.3). A tela, o CSV e o PDF são renderizações da mesma
//! `ReportTable`, então os três sempre mostram as mesmas linhas.

use serde::Serialize;

/// Os dez relatórios previstos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Occupancy,
    Reservations,
    Stays,
    Cancellations,
    Receipts,
    Balances,
    GuestExpenses,
    PropertyExpenses,
    Payables,
    Closings,
}

/// Filtros que um relatório pode aceitar (além do período).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Status,
    Room,
    Method,
    Category,
    Text,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::Status => "situação",
            Filter::Room => "quarto",
            Filter::Method => "forma de pagamento",
            Filter::Category => "categoria",
            Filter::Text => "texto",
        }
    }
}

impl ReportKind {
    pub const ALL: [ReportKind; 10] = [
        ReportKind::Occupancy,
        ReportKind::Reservations,
        ReportKind::Stays,
        ReportKind::Cancellations,
        ReportKind::Receipts,
        ReportKind::Balances,
        ReportKind::GuestExpenses,
        ReportKind::PropertyExpenses,
        ReportKind::Payables,
        ReportKind::Closings,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ReportKind::Occupancy => "occupancy",
            ReportKind::Reservations => "reservations",
            ReportKind::Stays => "stays",
            ReportKind::Cancellations => "cancellations",
            ReportKind::Receipts => "receipts",
            ReportKind::Balances => "balances",
            ReportKind::GuestExpenses => "guest_expenses",
            ReportKind::PropertyExpenses => "property_expenses",
            ReportKind::Payables => "payables",
            ReportKind::Closings => "closings",
        }
    }

    pub fn parse(value: &str) -> Option<ReportKind> {
        ReportKind::ALL.into_iter().find(|k| k.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            ReportKind::Occupancy => "Ocupação",
            ReportKind::Reservations => "Reservas",
            ReportKind::Stays => "Hospedagens",
            ReportKind::Cancellations => "Cancelamentos e no-shows",
            ReportKind::Receipts => "Recebimentos e devoluções",
            ReportKind::Balances => "Saldos pendentes",
            ReportKind::GuestExpenses => "Despesas extras dos hóspedes",
            ReportKind::PropertyExpenses => "Despesas da pousada",
            ReportKind::Payables => "Contas a pagar",
            ReportKind::Closings => "Fechamentos de caixa",
        }
    }

    /// Relatórios financeiros detalhados: só o administrador (spec §12.3).
    pub fn financial(self) -> bool {
        !matches!(self, ReportKind::Occupancy | ReportKind::Reservations | ReportKind::Stays | ReportKind::Cancellations)
    }

    /// Saldos pendentes é a situação de agora; os demais usam um período.
    pub fn uses_period(self) -> bool {
        self != ReportKind::Balances
    }

    pub fn accepts(self, filter: Filter) -> bool {
        use Filter::*;
        let accepted: &[Filter] = match self {
            ReportKind::Occupancy => &[Room],
            ReportKind::Reservations | ReportKind::Stays | ReportKind::Cancellations => &[Status, Room, Text],
            ReportKind::Receipts => &[Method, Room, Text],
            ReportKind::Balances | ReportKind::GuestExpenses => &[Room, Text],
            ReportKind::PropertyExpenses | ReportKind::Payables => &[Status, Category, Text],
            ReportKind::Closings => &[Method],
        };
        accepted.contains(&filter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnKind {
    Text,
    Date,
    Time,
    Int,
    Money,
    Percent,
}

#[derive(Debug, Clone, Serialize)]
pub struct Column {
    pub key: String,
    pub label: String,
    pub kind: ColumnKind,
}

impl Column {
    pub fn new(key: &str, label: &str, kind: ColumnKind) -> Self {
        Self { key: key.into(), label: label.into(), kind }
    }
}

/// Valor de uma célula. Dinheiro em centavos e percentual em pontos-base (10000 = 100%).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Cell {
    Text(String),
    Int(i64),
    Money(i64),
    Percent(i64),
    Empty,
}

impl Cell {
    pub fn text(value: impl Into<String>) -> Cell {
        Cell::Text(value.into())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportTable {
    pub kind: String,
    pub title: String,
    /// Frases dos filtros aplicados, por exemplo "Período: 01/10/2026 a 31/10/2026".
    pub filters: Vec<String>,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Cell>>,
    pub totals: Option<Vec<Cell>>,
    /// Havia mais linhas que o limite; tela, CSV e PDF avisam.
    pub truncated: bool,
}

/// `2026-10-21` → `21/10/2026` (texto que não for data fica como está).
pub fn br_date(iso: &str) -> String {
    match (iso.get(0..4), iso.get(5..7), iso.get(8..10)) {
        (Some(y), Some(m), Some(d)) if iso.len() >= 10 && iso.as_bytes()[4] == b'-' => format!("{d}/{m}/{y}"),
        _ => iso.to_string(),
    }
}

fn group_thousands(value: i64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

/// `15000` → `R$ 150,00`; negativo `-R$ 30,00`.
pub fn money_text(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs() as i64;
    format!("{sign}R$ {},{:02}", group_thousands(abs / 100), abs % 100)
}

/// `7500` → `75,0%`.
pub fn percent_text(bp: i64) -> String {
    let sign = if bp < 0 { "-" } else { "" };
    let abs = bp.unsigned_abs() as i64;
    let tenths = (abs + 5) / 10;
    format!("{sign}{},{}%", tenths / 10, tenths % 10)
}

/// Texto da célula como aparece na tela e no PDF.
pub fn cell_text(cell: &Cell) -> String {
    match cell {
        Cell::Text(t) => t.clone(),
        Cell::Int(n) => n.to_string(),
        Cell::Money(c) => money_text(*c),
        Cell::Percent(bp) => percent_text(*bp),
        Cell::Empty => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money_percent_and_dates_like_the_screen() {
        assert_eq!(money_text(15_000), "R$ 150,00");
        assert_eq!(money_text(-3_000), "-R$ 30,00");
        assert_eq!(money_text(123_456_789), "R$ 1.234.567,89");
        assert_eq!(money_text(5), "R$ 0,05");
        assert_eq!(percent_text(7_500), "75,0%");
        assert_eq!(percent_text(3_333), "33,3%");
        assert_eq!(br_date("2026-10-21"), "21/10/2026");
        assert_eq!(br_date("Total"), "Total");
    }

    #[test]
    fn financial_reports_are_the_six_admin_only_ones() {
        let financial: Vec<&str> = ReportKind::ALL.into_iter().filter(|k| k.financial()).map(ReportKind::as_str).collect();
        assert_eq!(financial, ["receipts", "balances", "guest_expenses", "property_expenses", "payables", "closings"]);
        assert!(ReportKind::Occupancy.accepts(Filter::Room) && !ReportKind::Occupancy.accepts(Filter::Method));
        assert!(!ReportKind::Balances.uses_period() && ReportKind::Receipts.uses_period());
    }
}
