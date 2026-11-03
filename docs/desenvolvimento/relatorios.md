# Relatórios, CSV e PDF (Marco 6.4)

Plano: `docs/superpowers/plans/2026-11-03-marco-6-4-relatorios-e-documentos.md`; decisões D1 a D8 e Anexo A.8 da spec. **Sem migração** nesta parte (a `0013` é dos documentos).

## Um motor, três saídas (D1)

`ReportService::run(token, ReportQuery) -> ReportTable` é o único caminho de consulta. A tela mostra a tabela; `domain::csv::render` e `report_service::table_pdf` a renderizam em CSV e PDF. `export_csv` e `export_pdf` chamam o mesmo `run`, então "CSV igual ao filtro da tela" vale por construção (o teste de volume confere que o CSV tem as mesmas linhas da tela nos dez relatórios).

- `domain/report.rs`: `ReportKind` (dez tipos, `financial()`, `accepts(Filter)`, `uses_period()`), `Column`/`ColumnKind`, `Cell` (`Text`, `Int`, `Money` em centavos, `Percent` em pontos-base, `Empty`), `ReportTable { kind, title, filters, columns, rows, totals, truncated }`, `money_text`, `percent_text`, `br_date`.
- `repositories/report_repo.rs` (operacionais, `Scope` com os limites) e `report_finance_repo.rs` (financeiros): SQL à mão, somente leitura, uma função por relatório.
- `services/report_service.rs`: validações, permissão, montagem das colunas e dos totais.

## Permissões (D2)

Ocupação, Reservas, Hospedagens e Cancelamentos: qualquer sessão. Os seis financeiros exigem `Permission::ViewFinancialReports` (administrador). A checagem vem **antes** de qualquer validação (recepção com filtro inválido também recebe `Forbidden`) e vale para `run`, `export_csv` e `export_pdf`; nada é gravado em disco se a permissão falhar.

## Filtros e datas (D3, D4)

- Período: `from`/`to` (datas de calendário), padrão = últimos 30 dias até hoje, `from ≤ to`, no máximo 366 dias. Saldos pendentes é a situação de agora e **recusa** período.
- Cada relatório declara os filtros que aceita (`accepts`); filtro não aceito é recusado (`Validation`), nunca ignorado. Situação, forma de pagamento e categoria são validadas.
- Datas de referência: Reservas = chegada; Hospedagens = dia local do check-in; Cancelamentos = dia local de `cancelled_at`; Recebimentos = dia **local** do instante (`utc_bounds` + `date(x, '±N seconds')`); Despesas extras = dia local do lançamento; Despesas da pousada = **competência** (mais "pago no período" pela data efetiva, nos filtros); Contas a pagar = vencimento; Fechamentos = `closing_date`.
- Recebimentos: **uma linha por componente** (débito e crédito separados); devolução sai com valor **negativo**, então o rodapé é o líquido, que bate com `closing_repo` e com a visão gerencial (testes de conciliação).
- Limite de **5.000 linhas** (`ROW_LIMIT`); a consulta busca uma a mais para saber se truncou; truncado = sem rodapé de totais e com aviso.

## CSV (D6)

UTF-8 com BOM, separador `;` (ou `,` por parâmetro), `\r\n`, aspas com escape, dinheiro como número (`150,00` com `;`, `150.00` com `,`), percentual com uma casa, datas `DD/MM/AAAA`. Texto que começa com `=`, `+`, `-`, `@`, tabulação ou retorno ganha um apóstrofo (injeção de fórmula); números não.

## PDF (D7)

`domain/pdf.rs` é um escritor mínimo e próprio: A4 (paisagem quando o relatório tem mais de 7 colunas), Helvetica e Helvetica-Bold embutidas do PDF com `WinAnsiEncoding`, cabeçalho de tabela repetido a cada página, texto truncado com reticências, rodapé "Página N de M". O conteúdo **não é comprimido**, o que deixa os testes simples (`extract_text` decodifica os octais). Larguras estimadas por tabela da Helvetica. Sem logotipo e sem fonte embutida (limitação).

## Exportação (D8)

`write_export(path, ext, bytes)`: extensão obrigatória, grava num temporário ao lado e renomeia (atômico), apaga o temporário se falhar e devolve "Não foi possível gravar neste local. Escolha outra pasta e tente de novo." O caminho nunca vai para log. A tela usa o diálogo "Salvar como" (`@tauri-apps/plugin-dialog`, permissão `dialog:allow-save`); abrir o arquivo passa por `open_exported_file`, que só aceita `.pdf`/`.csv` existentes e usa o plugin `opener` (por isso não há permissão `opener:allow-open-path` com escopo amplo).

## Frontend

`/relatorios` (`ReportsPage`, `ReportFilters`, `ReportTableView`, `lib/report.ts`): a recepção só lista os quatro operacionais; os filtros ficam na URL (`?tipo=&de=&ate=&situacao=&quarto=&forma=&categoria=&texto=`); exportar usa a consulta **aplicada** (a da tela), não o rascunho do formulário; **Tentar de novo** repete só a gravação.

## Volume

`five_years_of_fake_data_run_every_report_under_the_limit_and_csv_matches_the_screen`: 5 anos (1.825 dias × 8 quartos = 14.600 reservas com noites, hospedagens, cobranças, recebimentos, 1.825 contas e fechamentos) e cada relatório com 365 dias e com 1 dia. Medição em build de teste: o mais lento foi Hospedagens (2.912 linhas, ≈ 0,5–0,65 s); os demais ficaram abaixo de 0,1 s. Limite do teste: 3 s por relatório.

## Limitações

- Ocupação usa os quartos ativos de hoje (como a visão gerencial).
- Sem logotipo nos PDFs; fonte única (Helvetica).
- Saldos pendentes considera só hospedagens em andamento.
- O limite de 5.000 linhas é por relatório, sem paginação.
