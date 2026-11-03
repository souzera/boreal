# Marco 6.4 — Relatórios, exportações e documentos internos Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar à pousada os 10 relatórios do §12.3 com filtros combinados, exportação em CSV e PDF gerada no backend, e os 4 documentos internos do §17 (confirmação de reserva, comprovante de pagamento, ficha de hospedagem, demonstrativo final) com versões preservadas e reimpressão.

**Architecture:** Todo relatório vira a **mesma estrutura de dados** (`ReportTable`: colunas tipadas, linhas, totais e a descrição dos filtros). A tela, o CSV e o PDF são três *renderizadores* dessa estrutura, então "CSV igual ao filtro da tela" vale por construção. As consultas ficam isoladas em `report_repo`; a permissão é checada no serviço (relatórios financeiros só `ViewFinancialReports`). Um escritor de PDF mínimo e próprio (Helvetica embutida, sem dependência nova) serve relatórios e documentos. Os documentos gravam um **instantâneo JSON** versionado (`documents`), de onde o PDF é sempre gerado: reimprimir devolve a versão emitida; mudar os dados e emitir de novo cria a versão seguinte. O arquivo é gravado no caminho escolhido pela pessoa (diálogo "Salvar como") e pode ser aberto no leitor padrão para imprimir.

**Tech Stack:** Rust/rusqlite/SQLCipher, React/TS/TanStack Query/Vitest; plugins Tauri 2 `dialog` (salvar) e `opener` (abrir o PDF). Sem biblioteca de PDF nem de CSV.

**Spec:** `local/spec.md` §12.3 (relatórios), §17 (documentos), §9/§10/§11 (competência × pagamento, débito e crédito separados, fechamentos), §13 (permissões), §19 (datas e valores); `local/plano-completo.md` §10, tarefas 6.5 a 6.8. Parte **6.4 de 4** do Marco 6 (a última).

**Branch:** `marco-6-4-relatorios`, a partir de `identidade-visual` (que já contém 6.3). **Migração `0013`** (`documents`).

## Decisões deste plano (chamadas minhas; revisar na tela pronta)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| D1 | **Um motor, três saídas:** `ReportService::run(token, ReportQuery) -> ReportTable`; `export_csv` e `export_pdf` chamam o mesmo `run`. Nada de consulta separada para exportar. | Garante tela = CSV = PDF. Custo: o PDF herda as colunas da tela (sem layout exclusivo). |
| D2 | **Permissões:** Ocupação, Reservas, Hospedagens e Cancelamentos/no-shows → qualquer pessoa logada. Recebimentos e devoluções, Saldos pendentes, Despesas extras, Despesas da pousada, Contas a pagar e Fechamentos de caixa → só `ViewFinancialReports` (administrador). A tela lista só o que a pessoa pode abrir. | Spec §12.3: financeiros detalhados só para o administrador. Custo: a recepção com permissão de contas a pagar continua usando a tela de Contas a pagar, mas não o relatório. |
| D3 | **Filtros combinados** (todos opcionais, cada relatório declara quais aceita): período (`from`/`to`, no máximo 366 dias, padrão últimos 30 dias, datas de calendário), situação, quarto, forma de pagamento, categoria de despesa, texto (hóspede ou código). Filtro que o relatório não aceita é recusado (não ignorado em silêncio). | Evita "filtrei e nada mudou". |
| D4 | **Data de referência de cada relatório:** Reservas = data de chegada; Hospedagens = data de entrada (check-in local); Cancelamentos e no-shows = data do cancelamento / da marcação; Recebimentos e devoluções = instante do lançamento **na data local** (`utc_bounds`); Despesas extras = data do lançamento; Despesas da pousada = **competência**; Contas a pagar = vencimento (com a coluna de pago e a data do último pagamento); Fechamentos = dia fechado; Ocupação = cada dia do período. Despesas da pousada mostra também o total **pago no período** (por data efetiva), como pede o §10. | Spec §10: competência × pagamento distintos. |
| D5 | **Ocupação** = uma linha por dia (quartos ativos hoje, ocupados, livres, taxa em %), com total/média no rodapé — mesma regra da visão gerencial (6.3). | Mesma limitação já documentada (quartos ativos de hoje). |
| D6 | **CSV:** UTF-8 **com BOM**, separador `;` por padrão (Excel em português) e `,` opcional por parâmetro da exportação, aspas duplas com escape, fim de linha `\r\n`, valores monetários como número decimal com vírgula (`150,00`) quando o separador é `;` e ponto quando é `,`, datas `DD/MM/AAAA`. Campos que começam com `=`, `+`, `-`, `@` recebem um apóstrofo à frente (proteção contra fórmula no Excel), exceto números. | Abrir no Excel sem ajustes; não executar fórmula vinda de nome de hóspede. |
| D7 | **PDF próprio e mínimo** (`domain/pdf.rs`): A4, Helvetica e Helvetica-Bold embutidas do PDF (codificação WinAnsi cobre acentos do português), cabeçalho com nome da pousada, título, filtros e data/hora de emissão, tabela com quebra de página e repetição do cabeçalho da tabela, rodapé "Página N de M". Sem logotipo na v1. | Nenhuma dependência nova nem fonte para embutir; o PDF é conferível nos testes. Custo: logotipo e tipografia avançada ficam para depois. |
| D8 | **Salvar e imprimir:** o backend recebe o caminho escolhido no diálogo "Salvar como" (extensão `.pdf`/`.csv` obrigatória) e grava o arquivo **fora do banco**; a tela mostra "Arquivo salvo em …" com **Abrir arquivo** (leitor padrão, de onde se imprime) e **Tentar de novo**. Falha de gravação nunca repete a operação financeira, porque o documento já está emitido. | Spec §17: falha de impressão permite salvar ou tentar de novo. |
| D9 | **Documentos versionados** (`documents`: tipo, origem, versão, instantâneo JSON, quem e quando emitiu). **Emitir** devolve a versão vigente se o instantâneo atual for idêntico ao último; se os dados mudaram, cria a versão seguinte (nunca altera a anterior). **Reimprimir** sempre usa o instantâneo gravado. A lista de versões fica na tela da origem. Emitir audita (`documents`, `issue_document`) na mesma transação. | Spec §17. |
| D10 | **Quem emite documentos:** qualquer pessoa logada (a recepção entrega comprovante e ficha). O **demonstrativo final** só depois do check-out; a **confirmação de reserva** só para reserva ativa (pendente/confirmada/hospedada/concluída); o **comprovante** é de um recebimento; a **ficha** é de uma hospedagem (em andamento ou encerrada). | Evita documento sem sentido. |
| D11 | **Conteúdo mínimo provisório:** *Confirmação* — pousada, código, hóspede, quarto/categoria, entrada/saída com horários previstos, ocupantes, valor combinado, sinal esperado/recebido, política de cancelamento em texto curto. *Comprovante* — pousada, número do recebimento, data/hora, hóspede, referência (reserva ou hospedagem), formas de pagamento com valores, total. *Ficha* — dados da hospedagem, ocupantes (nome e documento), quarto, datas, linha de assinatura. *Demonstrativo* — lançamentos (diárias, consumos, estornos), recebimentos, devoluções e saldo final. ⚠ Aguardam aprovação (pendência §25.3). | Marcado como provisório no roteiro. |

## Global Constraints

- Regras críticas e permissões no backend; o frontend só esconde o que não pode abrir.
- Valores em centavos inteiros no motor; a formatação (`R$ 150,00`, `150,00`) fica no renderizador. Datas `DD/MM/AAAA`, horas `HH:MM`, "hoje" = data local (`ctx.today()`), instantes por `utc_bounds`.
- Relatórios e exportações são **somente leitura**; documentos gravam só a tabela `documents` e a auditoria, numa transação.
- Nada de dados pessoais em log; CPF **mascarado** em relatórios e documentos (a ficha mostra o documento informado pelo ocupante, como já é gravado).
- Conciliação: os totais dos relatórios batem com o fechamento de caixa, a visão gerencial e a conta da hospedagem (testes explícitos).
- Consultas dentro de limite de tempo com **5 anos de dados fictícios** (teste de volume).
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano, o Anexo A.8 da spec e o grafo.

## Review Focus

1. **Tela = CSV = PDF:** mesmos filtros, mesmas linhas, mesmos totais e a mesma ordem; filtro combinado nunca ignorado (T1, T2, T4).
2. **Recepção e relatório financeiro:** `Forbidden` no backend para os seis financeiros, inclusive na exportação; a tela não os lista (T1, T2, T4, T5).
3. **Dia local nos relatórios:** recebimento às 23:30 locais não cai no dia seguinte; período de 1 dia traz só esse dia; período invertido, vazio ou acima de 366 dias é recusado (T2).
4. **CSV seguro e abrível:** BOM, aspas, vírgula/ponto e vírgula dentro de campo, quebra de linha dentro de campo, nome começando por `=` ou `+` (T1).
5. **Documento emitido não muda:** alterar a reserva/conta depois da emissão não altera a versão antiga; reemitir sem mudança não cria versão; com mudança cria a seguinte; falha ao gravar o arquivo não cria nem repete nada financeiro (T6, T7).

---

## File Structure

```
src-tauri/
  migrations/sql/0013_documents.sql
  src/models/report.rs, document.rs
  src/domain/report.rs        # ReportTable, Cell, ColumnKind, ReportKind, formatação de célula
  src/domain/csv.rs           # renderizador CSV
  src/domain/pdf.rs           # escritor de PDF mínimo (páginas, texto, tabela, quebra)
  src/repositories/report_repo.rs, document_repo.rs
  src/services/report_service.rs, document_service.rs
  src/commands/reports.rs, documents.rs
src/
  lib/api.ts, lib/report.ts   # tipos, chamadas, rótulos dos relatórios e filtros
  pages/reports/ReportsPage.tsx, ReportFilters.tsx, ReportTableView.tsx (+ testes)
  components/DocumentActions.tsx (+ teste)   # Emitir / versões / Salvar PDF / Abrir
  pages/reservations/ReservationPage.tsx, pages/stays/StayPage.tsx   # botões de documento
  components/layout/NavItems.ts, routes.tsx  # item "Relatórios" ativo
docs/usuario/{relatorios,documentos}.md   docs/desenvolvimento/{relatorios,documentos}.md
```

---

### Task 1: Motor de relatórios, CSV e os quatro relatórios operacionais

**Files:**
- Create: `models/report.rs`, `domain/report.rs`, `domain/csv.rs`, `repositories/report_repo.rs`, `services/report_service.rs`
- Modify: `models/mod.rs`, `domain/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `app.rs`, `services/test_support.rs` (campo `reports`)
- Test: `domain/csv.rs`, `services/report_service.rs`

**Interfaces:**
- Produces:
  - `enum ReportKind { Occupancy, Reservations, Stays, Cancellations, Receipts, Balances, GuestExpenses, PropertyExpenses, Payables, Closings }` com `as_str()`/`parse()` (`occupancy`, `reservations`, `stays`, `cancellations`, `receipts`, `balances`, `guest_expenses`, `property_expenses`, `payables`, `closings`), `label()` em português, `financial() -> bool` e `accepts(filter: Filter) -> bool`;
  - `struct ReportQuery { kind: String, from: Option<String>, to: Option<String>, status: Option<String>, room_id: Option<i64>, method: Option<String>, category_id: Option<i64>, text: Option<String> }`;
  - `enum ColumnKind { Text, Date, Time, Int, Money, Percent }` e `struct Column { key: String, label: String, kind: ColumnKind }`; `enum Cell { Text(String), Int(i64), Money(i64), Percent(i64 /* pontos-base */), Empty }`;
  - `struct ReportTable { kind: String, title: String, filters: Vec<String> /* "Período: 01/10/2026 a 31/10/2026" */, columns: Vec<Column>, rows: Vec<Vec<Cell>>, totals: Option<Vec<Cell>>, truncated: bool }` (até 5.000 linhas na tela; `truncated` avisa; CSV e PDF usam o mesmo limite e o aviso);
  - `domain::csv::render(table: &ReportTable, delimiter: char) -> Vec<u8>` (BOM + linhas) — D6;
  - `ReportService::run(&self, token: &str, query: ReportQuery) -> Result<ReportTable, AppError>`.
- Regras: período padrão = últimos 30 dias até hoje; `from ≤ to`, ≤ 366 dias (`Validation`); filtro não aceito pelo relatório ⇒ `Validation("Este relatório não usa o filtro …")`; relatório financeiro sem `ViewFinancialReports` ⇒ `Forbidden`; relatório desconhecido ⇒ `Validation`.
  - **Occupancy:** uma linha por dia (`Data`, `Quartos`, `Ocupados`, `Livres`, `Ocupação %`); filtro `room_id` restringe a um quarto; rodapé com a taxa média.
  - **Reservations** (chegada no período): `Código`, `Hóspede`, `Quarto`, `Entrada`, `Saída`, `Noites`, `Situação`, `Valor combinado`; filtros `status`, `room_id`, `text`; rodapé com a soma do valor.
  - **Stays** (check-in no período): `Código`, `Hóspede`, `Quarto`, `Entrada`, `Saída prevista`, `Situação`, `Saldo`; filtros `status` (`in_house`/`checked_out`), `room_id`, `text`.
  - **Cancellations** (cancelamento ou no-show no período): `Código`, `Hóspede`, `Quarto`, `Tipo` (Cancelamento/No-show), `Data`, `Valor retido`, `Devolução prevista`; filtros `status` (`cancelled`/`no_show`), `room_id`, `text`.

- [x] **Step 1: Testes do CSV** (`domain/csv.rs`): começa com o BOM `EF BB BF`; cabeçalho e linhas separados por `;` e `\r\n`; campo com `;`, `"` e quebra de linha vai entre aspas com `""`; dinheiro com `;` sai `150,00` e com `,` sai `150.00`; negativo `-30,00`; `Percent(7500)` sai `75,0`; data permanece `DD/MM/AAAA`; texto `=SOMA(A1)` e `+55 11` saem com apóstrofo; texto `-` e `@x` idem; número negativo em `Money` **não** leva apóstrofo; rodapé de totais é a última linha; `truncated` acrescenta a linha de aviso.
- [x] **Step 2: Testes do serviço** (`Harness`, 21/10/2026 09:00 local): `reception_user_can_run_operational_reports_but_not_financial` (os quatro funcionam; os seis financeiros dão `Forbidden` — por enquanto só confere o mapeamento, os financeiros devolvem `Validation("em construção")` até a T2, então o teste cobre `Forbidden` antes do tipo); `invalid_periods_and_unknown_reports_are_refused`; `unsupported_filter_is_refused` (`method` em `occupancy`); `default_period_is_the_last_30_days`; `occupancy_has_one_row_per_day_and_matches_the_dashboard` (mesmos números da visão gerencial para o dia); `reservations_filter_by_status_room_and_text_combined` (3 reservas, filtro combinado devolve só a que bate nos três); `reservations_use_the_arrival_date` (chegada em 21/10 entra, em 22/10 não, num período de 1 dia); `stays_list_balance_and_status`; `cancellations_list_cancelled_and_no_show_with_retained_amount`; `rows_over_the_limit_are_truncated_with_a_flag` (limite reduzido por constante de teste).
- [x] **Step 3: Run** `cargo test --lib report` — Expected: FAIL.
- [x] **Step 4: Implement** modelos, `domain::report` (formatação de célula para texto: `R$ 150,00`, `75,0%`), `domain::csv`, `report_repo` (SQL à mão, uma função por relatório), `ReportService`; ligar em `app.rs` e no `Harness`.
- [x] **Step 5: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 6: Commit** `feat: report engine, csv and operational reports`

---

### Task 2: Relatórios financeiros e conciliação

**Files:**
- Modify: `repositories/report_repo.rs`, `services/report_service.rs`
- Test: `services/report_service.rs`

**Interfaces:**
- Consumes: T1; `closing_repo::{received, refunded}`, `payable_repo`, `stay_repo::balance`, `domain::dates::utc_bounds`, `domain::payment::Method`.
- Produces (todos `ViewFinancialReports`):
  - **Receipts** (recebimentos e devoluções no período, na data **local**): `Data`, `Hora`, `Número`, `Tipo` (Recebimento/Devolução), `Hóspede`, `Referência` (código da reserva ou hospedagem), `Forma`, `Valor`; **uma linha por componente** (débito e crédito separados); filtros `method`, `text`, `room_id`; rodapé: recebido, devolvido e líquido (três linhas de total ou uma linha com o líquido — **líquido na última coluna**).
  - **Balances** (situação atual das hospedagens em andamento, sem período): `Hospedagem`, `Hóspede`, `Quarto`, `Saída prevista`, `Saldo a receber`, `Crédito`; filtros `room_id`, `text`; rodapé com as duas somas.
  - **GuestExpenses** (consumos e extras lançados no período, por data local do lançamento; estornados aparecem com a linha de estorno): `Data`, `Hospedagem`, `Hóspede`, `Quarto`, `Item`, `Tipo`, `Valor`; filtros `room_id`, `text`; rodapé com o líquido.
  - **PropertyExpenses** (despesas da pousada por **competência** no período): `Competência`, `Código`, `Fornecedor`, `Descrição`, `Categoria`, `Valor`, `Pago`, `Em aberto`; filtros `category_id`, `text`, `status`; rodapé com as somas e, em `filters`, a linha "Pago no período (data efetiva): R$ …".
  - **Payables** (contas por **vencimento** no período): `Vencimento`, `Código`, `Fornecedor`, `Categoria`, `Valor`, `Pago`, `Em aberto`, `Situação`, `Último pagamento`; filtros `status` (`pending`/`partial`/`paid`/`overdue`), `category_id`, `text`.
  - **Closings** (dias fechados no período): `Dia`, `Fechado em`, `Por`, e por forma: `Esperado`, `Contado`, `Diferença` (uma linha por dia e forma); filtro `method`; rodapé com a soma das diferenças.
- Regras: nenhum relatório grava; textos de justificativa de diferença aparecem na coluna `Justificativa` do fechamento.

- [x] **Step 1: Testes:** `reception_gets_forbidden_for_every_financial_report` (seis tipos, inclusive com filtros inválidos: a permissão vem antes da validação); `receipts_have_one_row_per_component_and_split_debit_credit` (recebimento de 100 em Pix 60 + crédito 40 ⇒ duas linhas); `receipts_use_the_local_day` (23:30 local de 21/10 = 02:30Z de 22/10 entra em 21/10, não em 22/10); `receipts_net_total_matches_closing_and_dashboard` (mesmo dia: líquido do relatório = `closing.received − refunded` = `management.receipts_daily`); `refund_appears_as_its_own_row_and_lowers_the_net`; `balances_match_stay_account` (saldo da linha = `stays.get().account.balance_cents`; crédito separado); `guest_expenses_include_reversals_and_net_zero_for_reversed_items`; `property_expenses_use_competence_and_show_paid_in_period_separately` (conta de competência 10/2026 paga em 11/2026: aparece em outubro como em aberto/pago conforme o dia, e o "pago no período" só em novembro); `payables_filter_by_status_and_category_combined`; `closings_list_difference_and_justification`; `period_of_one_day_returns_only_that_day`.
- [x] **Step 2: Run** — Expected: FAIL.
- [x] **Step 3: Implement** as consultas e o mapeamento para `ReportTable`.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: financial reports with reconciliation tests`

---

### Task 3: Escritor de PDF e PDF dos relatórios

**Files:**
- Create: `domain/pdf.rs`
- Modify: `domain/mod.rs`, `services/report_service.rs` (`render_pdf`)
- Test: `domain/pdf.rs`, `services/report_service.rs`

**Interfaces:**
- Produces:
  - `struct PdfDoc` com `PdfDoc::new(title: &str)`, `page_header(&mut self, line1: &str, line2: &str)`, `heading(&mut self, text: &str)`, `text(&mut self, text: &str)` (quebra de linha por largura), `bold_text`, `spacer(mm)`, `table(&mut self, columns: &[PdfColumn], rows: &[Vec<String>], totals: Option<&[String]>)` (cabeçalho repetido a cada página, colunas com largura relativa e alinhamento à direita para números), `signature_line(label)`, `finish(self, footer: &str) -> Vec<u8>` (rodapé "Página N de M");
  - `struct PdfColumn { label: String, weight: u32, right: bool }`;
  - `ReportService::pdf(&self, token, query, issued_at_text) -> Result<Vec<u8>, AppError>` usando o mesmo `run` e o nome da pousada.
- Detalhes: A4 retrato (595×842 pt), margem 40 pt; fontes Helvetica/Helvetica-Bold com `/Encoding /WinAnsiEncoding`; o texto é convertido de UTF-8 para WinAnsi (caractere fora do mapa vira `?`); parênteses e barra invertida escapados; largura de texto estimada por tabela de larguras da Helvetica (aproximação por classe de caractere é aceitável, mas **nunca** deixar texto sair da coluna: truncar com "…"); relatório com mais colunas que cabem em retrato usa **paisagem** (841×595) quando `colunas > 7`.

- [x] **Step 1: Testes** (`domain/pdf.rs`): o arquivo começa com `%PDF-1.4` e termina com `%%EOF`; a tabela `xref` tem um offset por objeto e cada offset aponta para `N 0 obj`; `startxref` aponta para a tabela; o número de páginas em `/Count` bate com o de objetos `/Type /Page`; texto com acentos aparece como WinAnsi (`ç` = `\347`); parênteses e `\` escapados; 120 linhas de tabela geram 3 ou mais páginas, o cabeçalho da tabela aparece em cada uma e o rodapé diz "Página 1 de N"; texto maior que a coluna é truncado com "…"; mais de 7 colunas gera `/MediaBox [0 0 841 595]`; PDF vazio (sem linhas) ainda é válido, com "Nenhum registro no período.".
- [x] **Step 2: Teste do serviço:** `pdf_contains_the_title_filters_and_every_row_of_the_screen` (extrai as strings do fluxo de conteúdo e confere título, filtros e as linhas da `ReportTable` equivalente) e `pdf_of_a_financial_report_requires_the_permission`.
- [x] **Step 3: Run** — Expected: FAIL.
- [x] **Step 4: Implement** o escritor (objetos: catálogo, páginas, fontes, conteúdo; fluxo de conteúdo sem compressão para manter os testes simples) e `ReportService::pdf`.
- [x] **Step 5: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 6: Commit** `feat: pdf writer and report pdf`

---

### Task 4: Comandos, plugins e cliente da API

**Files:**
- Create: `commands/reports.rs`
- Modify: `commands/mod.rs`, `lib.rs`, `Cargo.toml` (`tauri-plugin-dialog = "2"`, `tauri-plugin-opener = "2"`), `capabilities/default.json` (`dialog:allow-save`, `opener:allow-open-path`), `package.json` (`@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener`), `src/lib/api.ts`, `src/lib/api.test.ts`

**Interfaces:**
- Produces:
  - comandos `run_report(token, query)`, `export_report_csv(token, query, delimiter: String, path: String) -> ExportResult`, `export_report_pdf(token, query, path: String) -> ExportResult`, `open_exported_file(token, path: String)`;
  - `struct ExportResult { path: String, bytes: u64 }`; gravação **atômica** (arquivo temporário vizinho + renomear); extensão obrigatória (`.csv` ou `.pdf`, senão `Validation`); pasta inexistente ou sem permissão ⇒ `Validation("Não foi possível gravar neste local. Escolha outra pasta e tente de novo.")`; o caminho **não** vai para o log;
  - TS: `ReportQuery`, `ReportTable`, `ReportColumn`, `ReportCell`, `ExportResult`, `runReport`, `exportReportCsv`, `exportReportPdf`, `openExportedFile`.
- Regras: exportar exige a mesma permissão de `run`; o CSV/PDF é gerado **a partir do mesmo `run`** (sem consulta própria).

- [x] **Step 1: Testes:** `api.test.ts` (cada função chama o comando certo com `{ token, query }`, `{ token, query, delimiter, path }`…); Rust: `export_writes_the_file_atomically_with_the_csv_bytes`, `export_refuses_a_wrong_extension`, `export_to_a_missing_folder_gives_a_friendly_error_and_leaves_nothing`, `export_requires_the_report_permission` (recepção + financeiro ⇒ `Forbidden` e **nenhum arquivo criado**).
- [x] **Step 2: Run** — Expected: FAIL.
- [x] **Step 3: Implement** comandos (a escrita em `report_service::write_export`), registrar os plugins em `lib.rs`.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run src/lib/api.test.ts`, `npx tsc --noEmit`, `cargo check` (valida as permissões) — Expected: PASS.
- [x] **Step 5: Commit** `feat: report commands, export and file plugins`

---

### Task 5: Tela de Relatórios

**Files:**
- Create: `src/lib/report.ts`, `src/pages/reports/{ReportsPage,ReportFilters,ReportTableView}.tsx` e testes
- Modify: `src/routes.tsx` (`/relatorios`), `src/components/layout/NavItems.ts` (Relatórios ativo: `to: "/relatorios"`, sem `soon`), `NavItems.test.ts`

**Interfaces:**
- Consumes: T4 (`runReport`, `exportReportCsv`, `exportReportPdf`, `openExportedFile`), `@tauri-apps/plugin-dialog` (`save`).
- Produces: `REPORTS: { kind, label, financial, filters: FilterKey[] }[]` espelhando o backend; `ReportsPage` com seletor de relatório (só os permitidos: administrador vê os 10, recepção vê 4), filtros do relatório escolhido (período com padrão últimos 30 dias, situação, quarto, forma de pagamento, categoria, texto), botão **Gerar relatório**, tabela (`ReportTableView`: cabeçalho fixo, valores `R$`, rodapé de totais, aviso de truncamento) e **Exportar CSV** / **Exportar PDF** (abrem "Salvar como" com nome sugerido `relatorio-<tipo>-AAAA-MM-DD.csv|pdf`; depois mostram "Arquivo salvo em …" com **Abrir arquivo**; erro mostra a mensagem e mantém **Tentar de novo**). Os filtros ficam na URL (`?tipo=receipts&de=…&ate=…`) para recarregar sem perder a consulta.

- [x] **Step 1: Testes:** recepção vê só os quatro operacionais e o administrador, os dez; trocar o relatório mostra só os filtros dele; período invertido ou acima de 366 dias mostra o erro e **não consulta**; **Gerar relatório** chama `runReport` com os filtros combinados e mostra colunas, linhas e rodapé em `R$`/`%`/`DD/MM/AAAA`; `truncated` mostra "Mostrando as primeiras 5.000 linhas; refine os filtros"; vazio mostra "Nenhum registro para estes filtros."; Exportar CSV: abre o diálogo com o nome sugerido, chama `exportReportCsv` com **os mesmos filtros** e o separador escolhido, mostra "Arquivo salvo em …" e **Abrir arquivo**; cancelar o diálogo não exporta nem mostra erro; falha de gravação mostra a mensagem e o botão **Tentar de novo**; os filtros e o relatório vêm da URL ao recarregar; o item do menu **Relatórios** abre a tela e deixou de ser "Em breve".
- [x] **Step 2: Run** `npx vitest run src/pages/reports src/components/layout` — Expected: FAIL.
- [x] **Step 3: Implement** a tela e o item de menu.
- [x] **Step 4: Run** `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: reports screen with csv and pdf export`

---

### Task 6: Documentos internos no backend

**Files:**
- Create: `migrations/sql/0013_documents.sql`, `models/document.rs`, `repositories/document_repo.rs`, `services/document_service.rs`
- Modify: `migrations/mod.rs`, `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `app.rs`, `services/test_support.rs` (campo `documents`)
- Test: `services/document_service.rs`, `migrations`

**Interfaces:**
- Migração `0013`: `documents(id, kind TEXT CHECK in ('reservation_confirmation','payment_receipt','stay_record','final_statement'), source_id INTEGER, version INTEGER, snapshot TEXT /* JSON */, snapshot_hash TEXT, issued_at, issued_by REFERENCES users, UNIQUE(kind, source_id, version))`, gatilhos `BEFORE UPDATE/DELETE` que abortam (**imutável**), índice `(kind, source_id)`.
- Produces: `enum DocumentKind` (`ReservationConfirmation`, `PaymentReceipt`, `StayRecord`, `FinalStatement`) com rótulos ("Confirmação de reserva", "Comprovante de pagamento", "Ficha de hospedagem", "Demonstrativo final da conta"); `struct DocumentView { id, kind, kind_label, source_id, version, issued_at, issued_by_name, current: bool }`; `DocumentService::{issue(token, kind, source_id) -> DocumentView, list(token, kind, source_id) -> Vec<DocumentView>, pdf(token, document_id) -> Vec<u8>}`.
- Regras (D9–D11): `issue` monta o instantâneo atual (JSON estável, campos ordenados), calcula `snapshot_hash`; se for igual ao da última versão devolve essa (sem gravar); senão grava a versão seguinte e audita (`documents`/`issue_document`, kind, origem e versão — **sem** o conteúdo) **na mesma transação**. `pdf` lê **só o instantâneo gravado** (nunca a origem) e o nome da pousada do **instantâneo** (o cabeçalho também é congelado na emissão: nome, endereço e telefone da pousada). Condições: confirmação só para reserva `pending|confirmed|checked_in|completed`; ficha só com hospedagem existente; demonstrativo só com `checked_out`; comprovante de um recebimento existente. Origem inexistente ⇒ `NotFound`; condição não atendida ⇒ `Validation` com texto da pousada ("O demonstrativo final só pode ser emitido depois do check-out."). Qualquer sessão válida emite e lista; sem sessão ⇒ erro de autenticação.

- [x] **Step 1: Testes:** `issue_creates_version_1_and_audits_in_the_same_transaction` (conta linhas em `documents` e `audit_log`); `issue_again_without_changes_returns_the_same_version_and_writes_nothing`; `issue_after_the_reservation_changed_creates_version_2_and_keeps_version_1_intact` (muda as datas; `pdf` da v1 ainda mostra as datas antigas, a v2 as novas); `pdf_uses_the_snapshot_even_if_the_pousada_name_changed_later`; `documents_cannot_be_updated_or_deleted` (SQL direto aborta); `final_statement_requires_checkout` / `confirmation_refused_for_cancelled_reservation` / `unknown_source_is_not_found`; `payment_receipt_lists_each_method_and_the_total` (pagamento dividido) e `statement_lists_charges_payments_refunds_and_final_balance` (valores batem com `stays.get().account`); `snapshot_has_masked_cpf_and_no_secret_fields`; `migration_0013_is_idempotent_in_the_migration_test` (atualiza a contagem de migrações do teste existente); `list_returns_versions_newest_first_with_current_flag`; `requires_a_session`.
- [x] **Step 2: Run** `cargo test --lib document` — Expected: FAIL.
- [x] **Step 3: Implement** migração, repositório, serviço e os quatro montadores de instantâneo (reaproveitando `ReservationService::get`, `StayService::get`, `PaymentService`/`payment_repo` e `pousada_repo`), mais o PDF de cada tipo usando `domain::pdf`.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: versioned internal documents`

---

### Task 7: Documentos na interface

**Files:**
- Create: `commands/documents.rs`, `src/components/DocumentActions.tsx` (+ teste)
- Modify: `commands/mod.rs`, `lib.rs`, `src/lib/api.ts` (+ teste), `src/pages/reservations/ReservationPage.tsx`, `src/pages/stays/StayPage.tsx` (+ testes)

**Interfaces:**
- Produces: comandos `issue_document(token, kind, source_id)`, `list_documents(token, kind, source_id)`, `save_document_pdf(token, document_id, path) -> ExportResult`; TS `issueDocument`, `listDocuments`, `saveDocumentPdf`, `DocumentView`; `DocumentActions({ kind, sourceId, label })`: botão **Emitir <rótulo>** (ex.: "Emitir confirmação de reserva"), lista das versões ("Versão 2 · emitida em 21/10/2026 às 14:05 por Ana · vigente") cada uma com **Salvar PDF** (diálogo "Salvar como", nome `confirmacao-RES-000005-v2.pdf`) e **Abrir** (leitor padrão, de onde se imprime); depois de salvar mostra "PDF salvo em …" com **Abrir arquivo**; falha de gravação mostra a mensagem e **Tentar de novo** **sem** emitir de novo (o documento já existe).
- Onde aparece: `ReservationPage` → Confirmação de reserva (reserva ativa); linhas de recebimento da reserva e da hospedagem → Comprovante de pagamento; `StayPage` → Ficha de hospedagem e, depois do check-out, Demonstrativo final.

- [x] **Step 1: Testes:** `api.test.ts` dos três comandos; `DocumentActions`: sem versões mostra só **Emitir …**; emitir chama `issueDocument` uma vez e atualiza a lista; lista versões em ordem com a vigente marcada **em texto**; **Salvar PDF** abre o diálogo com o nome sugerido e chama `saveDocumentPdf` com o caminho; cancelar o diálogo não faz nada; erro de gravação mostra a mensagem, mantém **Tentar de novo** e **não** chama `issueDocument`; `ReservationPage` mostra **Emitir confirmação de reserva** só para reserva ativa; `StayPage` mostra **Emitir ficha de hospedagem** sempre e **Emitir demonstrativo final** só com a hospedagem encerrada; cada recebimento tem **Emitir comprovante**.
- [x] **Step 2: Run** `npx vitest run src/components src/pages/reservations src/pages/stays src/lib/api.test.ts` — Expected: FAIL.
- [x] **Step 3: Implement** os comandos Rust e os componentes.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: document actions in reservation and stay screens`

---

### Task 8: Volume, seed, documentação e encerramento

**Files:**
- Modify: `src-tauri/src/seed.rs` (+ teste), `docs/desenvolvimento/seed.md`, `CHANGELOG.md`, `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/README.md`, `local/plano-completo.md`, `local/spec.md` (Anexo A.8 com D1–D11), este plano
- Create: `docs/usuario/{relatorios,documentos}.md`, `docs/desenvolvimento/{relatorios,documentos}.md`, teste de volume em `services/report_service.rs`

- [x] **Step 1: Volume:** `volume_5_years_of_fake_data_runs_every_report_under_the_limit` — insere por SQL ≈ 5 anos (1.800 dias) de reservas, hospedagens, recebimentos (com componentes), contas e fechamentos fictícios, roda os 10 relatórios do período máximo (366 dias) e do período curto e afirma cada um **< 2 s** em build de teste (ou o limite medido, registrado no plano), além de conferir que a tela (`run`) e o CSV têm o mesmo número de linhas.
- [x] **Step 2: Seed:** o cenário existente já alimenta os relatórios e permite emitir os quatro documentos; o teste afirma que `reports.run` (admin) devolve linhas para recebimentos e contas a pagar e que `documents.issue` funciona para a hospedagem do seed.
- [x] **Step 3: Docs:** guias de usuário (relatórios: escolher, filtrar, exportar, abrir; documentos: emitir, versões, reimprimir, o que fazer se não salvar) e de desenvolvimento (motor `ReportTable`, permissões, datas de referência, CSV, PDF, versionamento, limitações e provisórios D11), `CHANGELOG.md`, roteiro (módulos de Relatórios e Documentos; marcar ⚠ o conteúdo dos documentos), Anexo A.8 da spec, marca o 6.4 como implementado no plano-completo.
- [x] **Step 4: Verificação completa:** `cargo test --features dev-seed`, `cargo clippy --all-targets --features dev-seed -- -D warnings`, `npx vitest run`, `npx tsc --noEmit`, `npm run build`; registrar contagens no Status.
- [x] **Step 5:** `graphify update .`; anotar no Status.
- [x] **Step 6:** Revisão final da branch por revisor independente (modelo mais capaz) com o Review Focus; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [x] **Step 7: Commit** `docs: close marco 6.4`

---

## Status

**Implementado e revisado** na branch `marco-6-4-relatorios` (a partir de `identidade-visual`). T1 a T8 concluídas; migração `0013`.

- **Testes (última execução completa):** `cargo test --features dev-seed --lib` 611 testes; `cargo clippy --all-targets --features dev-seed -- -D warnings` limpo; `npx vitest run` 537 testes em 67 arquivos; `npx tsc --noEmit` e `npm run build` limpos; `graphify update .` executado.
- **Pendências de verificação:** (1) os 4 testes de integração `tests/startup.rs` não foram reexecutados nesta rodada porque um `boreal-pms.exe` (provavelmente `npm run tauri dev`) estava aberto e impedia o cargo de recompilar o binário; nada neste marco toca esse caminho. (2) Manual em `npm run tauri dev`: diálogo "Salvar como", abrir o PDF no leitor padrão, aparência dos PDFs impressos, CSV no Excel. (3) `search_over_five_years_of_data_stays_fast` é sensível a tempo sob carga paralela (passa isolado).

## Medição de volume

Cenário: 5 anos (14.600 reservas com noites, hospedagens, cobranças, recebimentos, 1.825 contas e fechamentos). Período de 365 dias: Hospedagens ≈ 0,5–0,65 s (2.912 linhas, o mais lento); Recebimentos ≈ 0,09 s; os demais < 0,06 s. Período de 1 dia: < 0,03 s. Limite do teste: 3 s.

## Desvios e decisões da execução

- A abertura do arquivo exportado é um comando do backend (`open_exported_file`, só `.pdf`/`.csv` existentes) em vez de `opener:allow-open-path` com escopo amplo; só `dialog:allow-save` entrou nas permissões.
- `ReportService::pdf(token, query)` calcula a data de emissão sozinho (`ctx.local_now`), sem o parâmetro `issued_at_text` do plano.
- Origem inexistente nos documentos devolve `Validation` ("não encontrada"), não `NotFound` (o projeto usa `Validation` para isso).
- O documento não guarda `snapshot_hash`: a comparação é pelo texto do instantâneo (evita depender de um hash que pode mudar entre versões do Rust).
- PDF paisagem usa 842×595 (A4) em vez de 841×595.
- O cenário de volume não tem cancelamentos nem hospedagens em andamento; Cancelamentos e Saldos podem vir vazios nesse teste.
- Sem logotipo nos PDFs (v1).


## Marcos seguintes

- **Marco 7:** backup, recuperação, atualização e segurança operacional (emite os alertas `backup_missing`, `disk_low` e `update_failed`; o backup deve incluir a tabela `documents`).

## Self-Review

- **Cobertura da spec:** §12.3 — 10 relatórios, filtros combinados, PDF e CSV no backend, financeiros só administrador → T1/T2/T3/T4/T5; §10 competência × pagamento → T2 (Despesas da pousada); §9 débito e crédito separados → T2 (Recebimentos, Fechamentos); §17 — 4 documentos, impressão/PDF, instantâneo e versões, reimpressão pela origem, falha sem repetir a operação → T6/T7; critério 11 (PDFs e CSVs refletem filtros e permissões) → T1/T2/T4; testes-chave do plano-completo (conciliação, CSV = filtro, recepção sem financeiro, reimpressão devolve a emitida, volume de 5 anos) → T2/T1/T4/T6/T8.
- **Consistência de tipos:** `ReportTable`/`Cell`/`ColumnKind` (T1) usados por T2, T3 e T5; `PdfDoc` (T3) usado por T6; `ExportResult` (T4) usado por T7; `DocumentKind` e `DocumentView` (T6) usados por T7.
- **Migração:** `0013` (`documents`); contagem de migrações do teste existente sobe de 12 para 13.
- **Proporção:** corpos de função ficam com o implementador; os testes trazem valores exatos (366 dias, 23:30/02:30Z, 5.000 linhas, 7 colunas, 2 s).
- **Fora deste plano:** logotipo no PDF (v1 só texto), assinatura digital, envio por e-mail, relatórios agendados, gráficos nos relatórios.

## Revisão final

Revisor independente (modelo mais capaz). Nenhum Crítico. Corrigidos com teste:

- **Importante** — "Pago no período" (Despesas da pousada) ignorava os filtros de texto e situação, mostrando um número que contradizia os filtros exibidos; agora usa os mesmos (`paid_in_period_follows_the_text_and_status_filters_too`).
- **Importante** — clicar de novo em **Gerar relatório** com os mesmos filtros não consultava de novo (a URL não mudava), e o CSV/PDF passavam a divergir da tela; agora refaz a consulta.
- **Menores corrigidos:** total de Reservas sem canceladas e no-shows; rodapé de Ocupação só com a média; ficha e comprovante sem campos que o PDF não mostra (reemitir não cria versão sem mudança visível); frase de vazio do PDF por contexto ("Nenhum registro para estes filtros."); larguras de texto mais conservadoras no PDF (negrito e acentos); mensagem própria para arquivo aberto em outro programa; caminhos relativos recusados na exportação e na abertura; falha do diálogo "Salvar como" vira mensagem; documentação ajustada (exceção do documento do ocupante na ficha).

**Menores adiados:** busca por texto sem acento em fornecedor/descrição/código (documentado); mensagem "Documento emitido" pode errar se a lista de versões não carregou (o backend poderia devolver `created`); `fit` do PDF é O(n²) em descrições muito longas; mensagem de truncamento da tela, do CSV e do PDF tem textos diferentes; o plano previa **Abrir** em cada versão (hoje só **Abrir arquivo** depois de salvar).
