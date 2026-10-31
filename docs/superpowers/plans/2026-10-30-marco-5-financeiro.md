# Marco 5 — Financeiro Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Substituir os recebimentos provisórios (Marcos 2 e 3) por recebimentos divididos por modalidade, devoluções ligadas ao recebimento original, bloqueio de duplicidade, despesas da pousada com contas a pagar e fechamento diário de caixa.

**Architecture:** Um `PaymentService` único passa a gravar todo recebimento (sinal da reserva ou conta da hospedagem) em `payments` + `payment_components`; `RefundService` grava devoluções em `refunds` + `refund_components`. O saldo da hospedagem continua sendo `lançamentos − recebido + devolvido` (`domain/account.rs`), portanto `payments.amount_cents` permanece o total. Contas a pagar (`payables`, `payable_payments`, `expense_categories`) e fechamento (`cash_closings`, `cash_closing_lines`) são módulos novos e independentes do saldo do hóspede. Regras puras (soma dos componentes, duplicidade, estado da conta, diferença do fechamento) ficam em `domain/`; o backend revalida tudo na gravação.

**Tech Stack:** o dos marcos anteriores (Rust/rusqlite/SQLCipher, React/TS/Vitest). Sem dependências novas.

**Spec:** `local/spec.md` §9, §10.2, §11, §13, §19.2 e Anexo A. Roteiro: `local/plano-completo.md` §9.

**Branch:** `marco-5-financeiro`, a partir da branch atual (`seed-dev`, que contém o Marco 4 e o seed de desenvolvimento). Migração `0011`.

## Status

**Não iniciado.** Plano escrito em 30/10/2026; nenhum passo executado.

## Decisões deste plano (chamadas minhas; revisar antes de executar)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| M5-1 | Migração é a **`0011`** (o roteiro dizia `0006`). `payments` é **reconstruída** (SQLite não remove coluna com `CHECK`): perde `method`, ganha campos de duplicidade; o método vai para `payment_components`. | Numeração avançou. Custo: reconstrução de tabela com cópia dos dados existentes (testada). |
| M5-2 | **Task 5.1 (tipo `Money`) já está cumprida** por `domain/money.rs` (`percent_of` meio-centavo-sobe, `add/sub/sum` com `checked_*`, D-12). Não se cria tipo novo (YAGNI); só entram testes de borda que faltarem. | Um `Money` newtype espalharia mudanças por 20 mil linhas sem ganho. |
| M5-3 | **Task 5.6 (despesas extras no saldo) já está cumprida** pelo livro `charges` do Marco 3. Aqui só há um teste de ponta a ponta (lançamento + recebimento dividido + devolução ⇒ saldo). | — |
| M5-4 | **O sinal da reserva vira recebimento real** (`payments.stay_id` nulo, `kind = 'deposit'`) em vez de só `reservations.deposit_paid_cents`. `deposit_paid_cents` continua existindo como **soma materializada** na mesma transação (cancelamento, alertas e listas já o leem). No check-in os recebimentos da reserva são **vinculados** à hospedagem (`stay_id` preenchido), não copiados; some o método `prior_deposit`. | O fechamento de caixa precisa ver o sinal no dia em que entrou. Custo: `record_deposit` e o check-in mudam. |
| M5-5 | **Dados anteriores ao Marco 5** (só existem em desenvolvimento) migram com modalidade `unspecified`, que o fechamento mostra numa linha própria "Sem modalidade". A interface nunca oferece `unspecified`. | Não inventar uma modalidade para dinheiro já recebido. |
| M5-6 | **Duplicidade:** um recebimento é "possível duplicado" se existe outro **da mesma hospedagem (ou da mesma reserva, para sinal)**, com **mesmo total**, **mesmo conjunto de modalidades** e **mesma data local**. O recebimento duplicado **não é gravado**: a chamada falha com `DuplicatePayment` (lista o recebimento parecido). Quem tem `ReleaseDuplicatePayment` repete a chamada com justificativa; o recebimento é gravado com `duplicate_of_payment_id`, justificativa, responsável e hora (também na auditoria). Quem não tem a permissão vê o aviso e não consegue prosseguir. | Spec §9.2 "bloqueados para revisão autorizada". Alternativa descartada: fila de recebimentos retidos (mais telas e estados). Custo se estiver errada: acrescentar um estado `held`. |
| M5-7 | **Devolução** exige **motivo** (obrigatório) mas **nenhuma permissão extra**; é auditada. A spec (§9.2, §13.1) não pede permissão para devolução (só para estorno de lançamento). Soma dos componentes = total; total ≤ (recebimento − já devolvido); a modalidade da devolução é livre. | Devolução de sinal por cancelamento é rotina da recepção. Se a pousada quiser controle, basta exigir `ReverseEntry` em `RefundService::record` (uma linha). |
| M5-8 | **Cancelamento não cria a devolução sozinho** (task 5.5). Ele continua registrando `retained`/`refund` calculados e passa a expor `refund_due_cents = refund_calculado − já devolvido` na reserva; a recepção usa `Registrar devolução` quando o dinheiro sai, escolhendo recebimento e modalidade. Cancelamento e no-show não alteram recebimentos. | O dinheiro pode ser devolvido depois (transferência externa, spec §9.2); gravar uma saída que não aconteceu distorceria o caixa. |
| M5-9 | **Contas a pagar:** nova permissão **`ManagePayables`** (administrador; **concedível** à recepção). Criar/editar categoria exige `ManageCatalog` (spec §10.2: "criadas pelo administrador"). Estados: `pending`, `partial`, `paid` calculados do saldo; `overdue` **calculado** (`due_date < hoje` e saldo > 0). Pagamento parcial ou integral grava `payable_payments` (modalidade, data/hora, usuário). Conta "à vista" = conta criada já paga (um pagamento do total na mesma transação). | A spec não nomeia quem lança despesas; despesa da pousada é sensível, por isso permissão própria. |
| M5-10 | **Fechamento esperado por modalidade = recebido − devolvido** no dia local (débito e crédito separados, `unspecified` à parte). **Pagamentos a fornecedores não entram** no esperado; aparecem numa linha informativa. | A spec lista as modalidades a conferir mas não diz que o caixa paga fornecedores. Custo: se a pousada paga fornecedor com dinheiro do caixa, subtrai-se `payable_payments` por modalidade (uma consulta). **Pergunta para a pousada.** |
| M5-11 | **Fechamento:** um por data local (`closing_date UNIQUE`), qualquer usuário autenticado, só para datas ≤ hoje, sem abertura formal. Exige o **valor conferido de cada uma das cinco modalidades** (linha `unspecified` só aparece com esperado ≠ 0); diferença ≠ 0 exige justificativa **por linha** mas nunca impede concluir. Grava esperado, conferido, diferença, justificativa, usuário e hora; **não altera recebimentos**. Depois de fechado, a data é imutável; movimentos posteriores no mesmo dia **não são bloqueados**: a tela mostra "movimentos depois do fechamento" (esperado atual − esperado gravado). | Spec §11: pendente não bloqueia o dia seguinte, sem transferência retroativa. Custo se estiver errada: bloquear lançamento em data fechada. |
| M5-12 | **Competência × pagamento** (task 5.8): `payables.competence_date` (data da despesa) × `payable_payments.paid_at` (data efetiva). `PayableService::summary(from, to, basis, category?)` devolve totais por categoria conforme a base; pagamentos parciais contam pelo valor e data de cada um. Os **relatórios completos** continuam no Marco 6. | Spec §10.2 pede a distinção; telas de relatório são do Marco 6. |
| M5-13 | Valores provisórios novos (entram no Anexo A.5, **aguardando aprovação**): categorias iniciais de despesa (Energia, Água, Internet e telefone, Alimentos e bebidas, Limpeza e lavanderia, Manutenção, Pessoal, Impostos e taxas, Outros); teto de um lançamento financeiro de **R$ 1.000.000,00** (já usado em `MAX_ENTRY_CENTS`). | Só a lista de categorias é novidade. |

## Global Constraints

- Valores em **centavos inteiros**, aritmética com `checked_*` (`domain/money.rs`); percentuais arredondados ao centavo mais próximo, meio centavo sobe (D-12).
- **A soma dos componentes deve ser exatamente o total** do recebimento ou da devolução (spec §9.1). Débito e crédito sempre separados.
- Modalidades: `cash`, `pix`, `debit`, `credit`, `transfer` (spec §9.1); `unspecified` só vem da migração.
- Recebimentos e devoluções têm **sequências independentes** (`PAG-000001`, `DEV-000001`), nunca reutilizadas (spec §9.2, §3.3). Contas a pagar usam `CAP-000001` (sequência nova `payable`).
- Devolução **vinculada ao recebimento original**, com motivo, data/hora e usuário; nunca acima do recebido (spec §9.2).
- **Lançamentos confirmados não são editados**; correção = estorno + relançamento (CLAUDE.md). Recebimentos e devoluções não têm edição nem exclusão.
- Operações relacionadas em **uma transação**; auditoria na mesma transação; `version` com `Conflict` onde a entidade é editável (CLAUDE.md, spec §2.3).
- Permissões no backend (`require(&actor, …)`), nunca só ocultando botão.
- Possível duplicidade **bloqueia**; liberar exige `ReleaseDuplicatePayment`, justificativa, responsável e data/hora (spec §9.2).
- Fechamento: diferença exige justificativa mas **não impede** a conclusão; fechamento pendente não bloqueia o dia seguinte; operações mantêm data/hora efetivas (spec §11).
- Contas a pagar: fornecedor, descrição, valor, vencimento, categoria; estados pendente/parcial/pago/vencido; categorias usadas **desativam**, não apagam (spec §10.2).
- Datas `YYYY-MM-DD`, instantes UTC; "dia" do fechamento é a data **local** (`ctx.today()`, `ctx.utc.local_offset()`); relógios injetados; na interface `DD/MM/AAAA`, `HH:MM`, `R$ 150,00`.
- Rótulos de ação: `Registrar pagamento`, `Registrar devolução`, `Liberar recebimento duplicado`, `Registrar conta a pagar`, `Registrar pagamento da conta`, `Concluir fechamento`.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano e o grafo a cada ciclo.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Duplo clique** em `Registrar pagamento` / `Registrar devolução` / `Concluir fechamento`: a segunda chamada vira `DuplicatePayment` (pagamento) ou erro de "já registrado" (fechamento); devolução repetida não passa do recebido. Nada grava duas vezes.
2. **Soma dos componentes ≠ total**, componente zero ou negativo, modalidade repetida, modalidade desconhecida, lista vazia, total acima de R$ 1.000.000,00: recusado **sem gravação parcial**.
3. **Devoluções sucessivas** (parcial + parcial + integral) em recebimento dividido: a terceira que ultrapassa o saldo do recebimento é recusada; devolver recebimento de outra hospedagem é recusado.
4. **Cancelamento com sinal pago e devolução depois:** `refund_due_cents` cai a cada devolução e nunca fica negativo; reserva cancelada não aceita novo sinal; sinal já devolvido não reabre a reserva.
5. **Fechamento** de data futura, de data já fechada, sem as cinco modalidades, com diferença sem justificativa, com recebimentos às 23:30 locais (virada de dia em UTC): data local correta; recebimentos não são alterados.
6. **Conta a pagar:** pagar acima do saldo, pagar conta paga, vencimento anterior à competência, categoria inativa, vencimento hoje (ainda não vencida) e ontem (vencida): recusa ou estado correto.
7. **Check-in de reserva com sinal dividido (Pix + dinheiro):** os recebimentos aparecem na conta com as duas modalidades; o saldo não muda; nenhum recebimento é duplicado; check-in repetido não revincula.

---

## File Structure

```
src-tauri/src/
  migrations/sql/0011_finance.sql              # rebuild payments; payment_components, refund_components; payables, payable_payments, expense_categories; cash_closings(+lines); sequence 'payable'
  domain/payment.rs                            # Method, Component, validate_components, is_possible_duplicate (puros)
  domain/payable.rs                            # PayableStatus + payable_status() (puro)
  domain/closing.rs                            # ExpectedLine, difference, validate_closing (puro)
  models/payment.rs, refund.rs, payable.rs, closing.rs
  repositories/payment_repo.rs, refund_repo.rs, payable_repo.rs, closing_repo.rs
  services/payment_service.rs, refund_service.rs, payable_service.rs, closing_service.rs
  commands/payments.rs, refunds.rs, payables.rs, closing.rs
  auth/permissions.rs                          # + ReleaseDuplicatePayment, ManagePayables
  services/stay_service.rs, reservation_service.rs, repositories/stay_repo.rs   # delegam ao PaymentService; check-in vincula
  seed.rs                                      # recebimentos divididos, devolução, contas a pagar, um fechamento
src/
  pages/payments/PaymentDialog.tsx, ComponentsEditor.tsx, RefundDialog.tsx, DuplicateDialog.tsx
  pages/payables/PayablesPage.tsx, PayableDialog.tsx, PayPayableDialog.tsx, CategoriesPanel.tsx
  pages/closing/ClosingPage.tsx, ClosingDialog.tsx
  pages/stays/StayPage.tsx, pages/reservations/ReservationPage.tsx, Dashboard.tsx   # ajustes
  lib/api.ts, lib/payment.ts, routes.tsx, components/layout/Sidebar.tsx
docs/usuario/{pagamentos-e-devolucoes,contas-a-pagar,fechamento-de-caixa}.md
docs/desenvolvimento/{financeiro,fechamento-de-caixa}.md
```

---

### Task 1: Migração `0011`

**Files:**
- Create: `src-tauri/src/migrations/sql/0011_finance.sql`
- Modify: `src-tauri/src/migrations/mod.rs` (versão 11)
- Test: `src-tauri/src/migrations/mod.rs`

**Interfaces:**
- Produces (tabelas):
  - `payments(id, code UNIQUE, reservation_id, stay_id NULL, kind 'deposit'|'stay', amount_cents > 0, received_at, received_by, note, duplicate_of_payment_id NULL, duplicate_justification NULL, duplicate_released_by NULL)`; **sem** `method`.
  - `payment_components(id, payment_id, method 'cash'|'pix'|'debit'|'credit'|'transfer'|'unspecified', amount_cents > 0, UNIQUE(payment_id, method))`.
  - `refunds(id, code UNIQUE, payment_id, stay_id NULL, reservation_id, amount_cents > 0, reason, refunded_at, refunded_by)` + `refund_components` (mesmas regras de `payment_components`).
  - `expense_categories(id, name, name_key UNIQUE, active, version, created_at, updated_at)` com as 9 categorias de M5-13.
  - `payables(id, code UNIQUE, supplier, description, amount_cents > 0, competence_date, due_date, category_id, notes, created_by, version, created_at, updated_at)`; `payable_payments(id, payable_id, amount_cents > 0, method (5 modalidades), paid_at, paid_by, note)`.
  - `cash_closings(id, closing_date UNIQUE, closed_at, closed_by)`; `cash_closing_lines(id, closing_id, method, expected_cents, counted_cents, difference_cents, justification NULL, UNIQUE(closing_id, method))`.
  - Sequência `payable` em 1; `reservations.cancellation_refund_cents INTEGER NOT NULL DEFAULT 0` (devolução calculada no cancelamento; reservas já canceladas migram com `deposit_paid_cents − cancellation_fee_cents`).
- Migração copia cada `payments` antigo: `kind` = `deposit` se `stay_id` nulo, senão `stay` (**exceto** `method = 'prior_deposit'`, que vira `deposit`); um componente com `method` antigo, e `prior_deposit` ⇒ `unspecified` (M5-5). `refunds` antigos ganham `reservation_id` pela hospedagem e um componente `unspecified`.

- [ ] **Step 1: Write the failing tests** em `migrations/mod.rs`:
  - `migration_0011_creates_finance_tables` (todas as tabelas existem; `payments` não tem coluna `method`; versão 11; sequência `payable`; 9 categorias semeadas).
  - `migration_0011_preserves_existing_payments_as_components`: aplicar até a 0010, inserir um `payments` com `method='pix'` (100 reais) e outro `prior_deposit`, migrar para 11 ⇒ componentes `pix` e `unspecified` com os mesmos valores e `payments.amount_cents` intactos.
  - `payment_components_reject_duplicate_method_and_zero_amount`.
  - `cash_closings_reject_second_closing_for_same_date`.
- [ ] **Step 2: Run** `cargo test migration_0011` — Expected: FAIL (versão 10).
- [ ] **Step 3: Write** `0011_finance.sql` (reconstrução com `PRAGMA foreign_keys` tratada como nas migrações anteriores que reconstroem tabela; copiar dados antes de remover; recriar índice `idx_payments_stay`) e registrar `Migration { version: 11, ... }`.
- [ ] **Step 4: Run** `cargo test migration` — Expected: PASS (inclui as anteriores; os testes de `stay_repo` que inserem `payments.method` serão ajustados na Task 4).
- [ ] **Step 5: Commit** `feat: migration 0011 finance`

---

### Task 2: Domínio puro — componentes, duplicidade, conta a pagar, fechamento

**Files:**
- Create: `src-tauri/src/domain/payment.rs`, `domain/payable.rs`, `domain/closing.rs`
- Modify: `src-tauri/src/domain/mod.rs`

**Interfaces:**
- Produces:
  - `enum Method { Cash, Pix, Debit, Credit, Transfer, Unspecified }` com `as_str`, `parse(&str) -> Option<Method>`, `Method::selectable() -> [Method; 5]`, `label()` (`Dinheiro`, `Pix`, `Cartão de débito`, `Cartão de crédito`, `Transferência bancária`).
  - `struct Component { method: Method, amount_cents: i64 }`.
  - `fn validate_components(items: &[Component], max_total_cents: i64) -> Result<i64, AppError>` — lista não vazia, cada valor > 0, modalidade selecionável (nunca `Unspecified`), sem modalidade repetida, soma com `money::sum` > 0 e ≤ `max_total_cents`; devolve o total.
  - `struct PaymentFacts { total_cents: i64, methods: BTreeSet<Method>, local_date: NaiveDate }`; `fn is_possible_duplicate(candidate: &PaymentFacts, existing: &PaymentFacts) -> bool` (total, conjunto de modalidades e data iguais).
  - `fn refundable_cents(payment_total: i64, already_refunded: i64) -> Result<i64, AppError>`.
  - `enum PayableStatus { Pending, Partial, Paid, Overdue }`; `fn payable_status(amount_cents: i64, paid_cents: i64, due: NaiveDate, today: NaiveDate) -> PayableStatus` — `Paid` se `paid ≥ amount`; senão `Overdue` se `due < today`; senão `Partial` se `paid > 0`; senão `Pending`.
  - `struct ExpectedLine { method: Method, expected_cents: i64 }`, `struct CountedLine { method: Method, counted_cents: i64, justification: Option<String> }`; `fn build_closing_lines(expected: &[ExpectedLine], counted: &[CountedLine]) -> Result<Vec<ClosingLine>, AppError>` — exige conferido para as cinco modalidades selecionáveis (e para `Unspecified` se esperado ≠ 0), valor conferido ≥ 0, justificativa não vazia quando a diferença ≠ 0; `ClosingLine { method, expected_cents, counted_cents, difference_cents, justification }`, diferença = conferido − esperado.

- [ ] **Step 1: Write the failing tests:**
  - `payment.rs`: `components_must_sum_exactly` (Pix 6.000 + dinheiro 4.000 ⇒ total 10.000), `rejects_empty_zero_negative_repeated_and_unspecified`, `rejects_total_above_limit` (100.000.001 > 100.000.000), `debit_and_credit_are_distinct_methods`, `duplicate_needs_same_total_methods_and_local_date` (mesmo total, Pix vs dinheiro ⇒ não; mesmo conjunto em dia diferente ⇒ não), `refundable_is_payment_minus_refunded_and_never_negative`.
  - `payable.rs`: `due_today_is_not_overdue`, `due_yesterday_unpaid_is_overdue`, `partially_paid_and_overdue_is_overdue`, `fully_paid_is_paid_even_if_late`, `partial_before_due_is_partial`.
  - `closing.rs`: `difference_is_counted_minus_expected` (esperado 10.000, conferido 9.500 ⇒ −500), `difference_without_justification_is_refused`, `difference_with_justification_is_accepted`, `missing_method_is_refused`, `unspecified_line_required_only_when_expected_is_not_zero`, `negative_counted_is_refused`.
- [ ] **Step 2: Run** `cargo test domain::payment domain::payable domain::closing` — Expected: FAIL (módulos inexistentes).
- [ ] **Step 3: Implement** os módulos conforme as assinaturas acima (puros, sem banco, erros `AppError::Validation` em português simples).
- [ ] **Step 4: Run** os mesmos testes — Expected: PASS.
- [ ] **Step 5: Commit** `feat: pure domain for payments, payables and closing`

---

### Task 3: Permissões novas

**Files:**
- Modify: `src-tauri/src/auth/permissions.rs`, `src/pages/users/labels.ts`, `src/pages/users/labels.test.ts`, `docs/desenvolvimento/permissoes.md`, `docs/usuario/usuarios.md`
- Test: `src-tauri/src/auth/permissions.rs`

**Interfaces:**
- Produces: `Permission::ReleaseDuplicatePayment` (`"release_duplicate_payment"`) e `Permission::ManagePayables` (`"manage_payables"`), ambas em `ALL` e em `grantable()`; rótulos `Liberar recebimento duplicado` e `Gerenciar contas a pagar`.

- [ ] **Step 1: Write the failing tests:** `new_finance_permissions_are_grantable_and_parse`; teste existente da máscara de bits cobre o novo número de variantes (`PermissionSet::with` para todas, sem estourar `u32`); `labels.test.ts` exige rótulo para toda permissão concedível.
- [ ] **Step 2: Run** `cargo test auth::permissions` e `npx vitest run src/pages/users/labels.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** as duas variantes e `as_str`/`grantable`; rótulos no front; documentação de permissões.
- [ ] **Step 4: Run** os testes — Expected: PASS.
- [ ] **Step 5: Commit** `feat: release-duplicate-payment and manage-payables permissions`

---

### Task 4: `PaymentService` — recebimentos divididos (substitui o provisório)

**Files:**
- Create: `src-tauri/src/models/payment.rs`, `repositories/payment_repo.rs`, `services/payment_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs` (+`AppState`), `services/stay_service.rs` (remove `record_payment`, `PAYMENT_METHODS`; check-in vincula), `services/reservation_service.rs` (`record_deposit`), `repositories/stay_repo.rs` (`insert_payment`, `payments` da conta, vínculo no check-in), `services/test_support.rs`/`repositories/test_db.rs` (inserções de `payments`), `seed.rs`
- Test: `services/payment_service.rs`

**Interfaces:**
- Consumes: `domain::payment::{validate_components, is_possible_duplicate}` (T2), `require`, `sequence_repo::{next, format_code}`, `audit::record`, `ctx.today()`.
- Produces:
  - `struct ComponentInput { method: String, amount_cents: i64 }`; `struct DuplicateRelease { justification: String }`; `struct PaymentInput { components: Vec<ComponentInput>, note: String, duplicate_release: Option<DuplicateRelease> }`.
  - `enum PaymentTarget { Stay { stay_id: i64 }, Reservation { reservation_id: i64, expected_version: i64 } }`.
  - `PaymentService::record(&self, token: &str, target: PaymentTarget, input: PaymentInput) -> Result<PaymentReceipt, AppError>` com `PaymentReceipt { id: i64, code: String, total_cents: i64 }`.
  - `AppError::DuplicatePayment(DuplicateInfo)` (novo; código estável `duplicate_payment`, carrega `code`, `received_at` local, `total_cents` e `methods` do recebimento parecido) em `error.rs`.
  - `payment_repo::{insert, insert_components, find_similar(conn, scope, facts) -> Option<PaymentRow>, link_to_stay(tx, reservation_id, stay_id) -> usize, sum_for_reservation(conn, reservation_id) -> i64}`.
  - `StayAccount.payments: Vec<PaymentLine>` agora com `components: Vec<ComponentLine { method, amount_cents }>` (sem `method` único) e `released_duplicate: bool`.
- Regras: `Stay` exige hospedagem aberta (`in_house`); `Reservation` exige reserva `pending|confirmed`, `expected_version` igual (`Conflict`) e `soma ≤ agreed_total`; ao gravar sinal atualiza `deposit_paid_cents` e confirma a reserva ao atingir `deposit_expected_cents` (lógica hoje em `record_deposit`, movida para cá via `reservation_repo::update_payment_state`); duplicado sem `duplicate_release` ⇒ `DuplicatePayment`; com `duplicate_release` exige `Permission::ReleaseDuplicatePayment` e justificativa não vazia, grava `duplicate_of_payment_id`; tudo (recebimento, componentes, sequência, atualização da reserva ou da hospedagem, auditoria `record_payment`) em **uma transação**.

- [ ] **Step 1: Write the failing tests** (`payment_service.rs`, no estilo dos testes de `stay_service` com o `World`/`FakeUtc` existentes):
  - `split_payment_records_components_and_reduces_balance` (conta R$ 400,00; Pix 6.000 + dinheiro 4.000 ⇒ saldo 30.000; `payments.amount_cents = 10.000`; dois componentes; código `PAG-000001`).
  - `components_not_matching_are_refused_without_writing` (componente 0, repetido, `prior_deposit`, `unspecified`, vazio, total 100.000.001 ⇒ `Validation`; contagens de `payments`/`payment_components`/`sequences` intactas).
  - `same_stay_same_total_methods_and_day_is_blocked_as_duplicate` (segundo idêntico ⇒ `DuplicatePayment` com o código `PAG-000001`; nada gravado; sequência não consumida); `different_day_or_method_is_not_duplicate`.
  - `duplicate_release_requires_permission_and_justification` (recepção sem permissão ⇒ `Forbidden`; admin sem justificativa ⇒ `Validation`; admin com justificativa ⇒ grava com `duplicate_of_payment_id`, auditoria com justificativa e responsável).
  - `deposit_on_reservation_confirms_it_and_keeps_deposit_paid_in_sync` (sinal esperado em dois pagamentos: pendente → confirmada no segundo; `deposit_paid_cents` = soma dos recebimentos; `version` errada ⇒ `Conflict` sem gravar; acima do total da reserva ⇒ `Validation`).
  - `deposit_on_cancelled_or_closed_reservation_is_refused`.
  - `check_in_links_deposit_payments_instead_of_copying_them` (sinal Pix 7.000 + dinheiro 5.000 ⇒ após o check-in a conta lista **os mesmos** recebimentos e componentes, `count(payments)` inalterado, saldo `charges − 12.000`; sem recebimento `prior_deposit`; check-in repetido recusado não revincula).
  - `payment_on_checked_out_stay_is_refused`.
  - `rolled_back_payment_does_not_consume_the_sequence` (falha de auditoria simulada/duplicado ⇒ próximo código continua `PAG-000001`).
  - Substituir os testes antigos de `stay_service` (`record_payment_reduces_balance_and_validates` etc.) pelos equivalentes com `PaymentInput` novo; o teste de propriedade de saldo (`SUM(charges) − SUM(payments) + SUM(refunds)`) segue passando.
- [ ] **Step 2: Run** `cargo test payment_service` — Expected: FAIL (serviço inexistente).
- [ ] **Step 3: Implement** `PaymentService::record` e repositório; fazer `StayService`/`ReservationService::record_deposit` **delegarem** (o comando `record_deposit` deixa de existir: o front chama `record_reservation_payment`); no check-in trocar o bloco `prior_deposit` por `payment_repo::link_to_stay`; remover do esquema do código o método `prior_deposit`; ajustar `seed.rs` e fixtures de teste.
- [ ] **Step 4: Run** `cargo test` — Expected: PASS (suíte inteira, inclusive os testes de check-in dos Marcos 3/4).
- [ ] **Step 5: Commit** `feat: split payments, duplicate guard and deposit as a real payment`

---

### Task 5: `RefundService` — devoluções e devolução pendente do cancelamento

**Files:**
- Create: `src-tauri/src/models/refund.rs`, `repositories/refund_repo.rs`, `services/refund_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `repositories/stay_repo.rs` (devoluções com componentes), `models/reservation.rs` + `repositories/reservation_repo.rs` (`refund_due_cents`), `services/reservation_service.rs` (`get` calcula devolução pendente)
- Test: `services/refund_service.rs`

**Interfaces:**
- Consumes: `domain::payment::{validate_components, refundable_cents}` (T2), `PaymentService` tables (T4).
- Produces:
  - `struct RefundInput { payment_id: i64, components: Vec<ComponentInput>, reason: String }`.
  - `RefundService::record(&self, token: &str, input: RefundInput) -> Result<RefundReceipt, AppError>` (`RefundReceipt { id, code, total_cents }`, código `DEV-000001`).
  - `refund_repo::{insert, insert_components, refunded_for_payment(conn, payment_id) -> i64, refunded_for_reservation(conn, reservation_id) -> i64}`.
  - `ReservationDetail.reservation.refund_due_cents: i64` = `max(0, refund_calculado_do_cancelamento − devolvido_da_reserva)`; zero fora de reservas canceladas. O valor calculado vem da nova coluna `reservations.cancellation_refund_cents` (criada na Task 1, `NOT NULL DEFAULT 0`; gravada por `reservation_repo::set_closed` no cancelamento).
- Regras: motivo obrigatório (≤ 200 caracteres); total ≤ `refundable_cents`; hospedagem do recebimento definida ⇒ `refunds.stay_id` copiado; devolução em hospedagem **encerrada** é permitida (devolução de crédito no pós-check-out); usuário e instante gravados; auditoria `record_refund` na mesma transação; sem permissão extra (M5-7).

- [ ] **Step 1: Write the failing tests:**
  - `partial_then_full_refund_never_exceeds_the_payment` (recebimento 10.000: devolver 3.000, depois 7.000 ok; mais R$ 0,01 ⇒ `Validation`).
  - `refund_split_across_methods_must_sum_exactly` (Pix 2.000 + dinheiro 1.000 = 3.000 ok; soma errada ⇒ recusa sem gravar).
  - `refund_requires_reason_and_existing_payment` (motivo vazio/espaços; `payment_id` inexistente).
  - `refund_raises_the_stay_balance_by_the_refunded_amount` (saldo = lançamentos − recebido + devolvido).
  - `cancellation_exposes_pending_refund_and_it_shrinks_with_each_refund` (sinal 12.000 cancelado 8 dias antes ⇒ `refund_due_cents = 12.000`; após devolução de 5.000 ⇒ 7.000; após o resto ⇒ 0; devolução extra ⇒ recusada).
  - `no_show_has_no_pending_refund` e `cancellation_does_not_create_refund_or_change_payments` (contagens de `refunds` e `payments` iguais antes/depois).
  - `refund_audit_records_user_reason_and_amount`.
  - `double_click_cannot_refund_twice_beyond_the_balance` (duas chamadas idênticas de integral: a segunda `Validation`).
- [ ] **Step 2: Run** `cargo test refund_service` — Expected: FAIL.
- [ ] **Step 3: Implement** serviço, repositório e `refund_due_cents`; atualizar `StayAccount.refunds` com `components`.
- [ ] **Step 4: Run** `cargo test` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: refunds linked to the original payment`

---

### Task 6: Categorias de despesa e contas a pagar

**Files:**
- Create: `src-tauri/src/models/payable.rs`, `repositories/payable_repo.rs`, `services/payable_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`
- Test: `services/payable_service.rs`

**Interfaces:**
- Consumes: `domain::payable::payable_status`, `domain::payment::{validate_components, Method}` (T2), `Permission::{ManagePayables, ManageCatalog}` (T3), `domain::text` (chave de nome sem acento, como `catalog_items.name_key`).
- Produces:
  - `struct NewPayable { supplier, description, amount_cents, competence_date, due_date, category_id, notes, paid_now: Option<PayNow> }`; `PayNow { method: String, paid_date: Option<…> }` — "à vista" grava a conta e um pagamento do total, na mesma transação.
  - `PayableService::{create, list(filters: status?, category?, due_from?, due_to?, page), get, pay(token, payable_id, PayInput { amount_cents, method, note }, expected_version) -> Result<i64, AppError>, update_notes(...)}`; `PayableView` com `paid_cents`, `balance_cents`, `status` (`pending|partial|paid|overdue`), `status_label`.
  - `PayableService::{list_categories, create_category(token, name), rename_category, set_category_active(token, id, active, expected_version)}` — exigem `ManageCatalog`; categoria com contas **não** é apagada (só desativada); nome único sem acento/caixa; categoria inativa não aceita contas novas.
  - `PayableService::summary(&self, token, SummaryQuery { from, to, basis: "competence"|"payment", category_id: Option<i64> }) -> Vec<CategoryTotal { category_id, category_name, total_cents }>` (M5-12).
- Regras: todas as operações de conta exigem `ManagePayables`; valor 1…1.000.000,00; vencimento ≥ competência; fornecedor 1…120 e descrição 1…200 caracteres; pagamento ≤ saldo da conta e conta paga recusa novo pagamento; pagamento parcial conta pelo valor e data próprios; `pay` confere `version` (`Conflict`); tudo auditado na mesma transação; código `CAP-000001`.

- [ ] **Step 1: Write the failing tests:**
  - `create_payable_starts_pending_with_code_and_audit` (`CAP-000001`, status `pending`, auditoria `create_payable`).
  - `requires_manage_payables_for_every_operation` (recepção sem permissão ⇒ `Forbidden` em criar, listar, pagar; recepção **com** a permissão concedida passa).
  - `partial_then_full_payment_moves_pending_partial_paid` (R$ 500: 200 ⇒ `partial`, saldo 300; +300 ⇒ `paid`; novo pagamento ⇒ `Validation`).
  - `payment_above_balance_is_refused_without_writing`.
  - `due_today_is_pending_and_due_yesterday_is_overdue` (relógio injetado).
  - `paid_now_records_payable_and_payment_atomically` (conta à vista nasce `paid`; falha no pagamento não deixa a conta).
  - `due_before_competence_is_refused`; `inactive_category_rejects_new_payables`.
  - `category_with_payables_can_only_be_deactivated` (tentar excluir não existe; desativar ok; reativar ok); `category_names_are_unique_ignoring_accents_and_case` (`Energia` × `energia`); `create_category_requires_manage_catalog`.
  - `summary_by_competence_and_by_payment_differ` (conta de R$ 500 com competência em outubro, pagamentos de 200 em outubro e 300 em novembro: competência outubro = 500; pagamento outubro = 200, novembro = 300).
  - `stale_version_on_pay_is_conflict`.
- [ ] **Step 2: Run** `cargo test payable_service` — Expected: FAIL.
- [ ] **Step 3: Implement** serviço e repositório (SQL à mão, `payable_status` aplicado na leitura; filtro de status vencido por data na consulta).
- [ ] **Step 4: Run** `cargo test payable` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: expense categories and accounts payable`

---

### Task 7: `ClosingService` — fechamento diário de caixa

**Files:**
- Create: `src-tauri/src/models/closing.rs`, `repositories/closing_repo.rs`, `services/closing_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`
- Test: `services/closing_service.rs`

**Interfaces:**
- Consumes: `domain::closing::{build_closing_lines, ExpectedLine, CountedLine}` (T2), tabelas de T4/T5/T6.
- Produces:
  - `ClosingService::preview(&self, token, date: NaiveDate) -> ClosingPreview { date, closed: Option<ClosingView>, expected: Vec<ExpectedLineView { method, received_cents, refunded_cents, expected_cents }>, supplier_payments_cents: i64, after_closing_cents: Option<i64> }`.
  - `ClosingService::close(&self, token, input: CloseInput { date, counted: Vec<CountedInput { method, counted_cents, justification }> }) -> Result<ClosingView, AppError>`.
  - `ClosingService::pending_dates(&self, token) -> Vec<NaiveDate>` — datas com movimento (recebimento ou devolução) e **sem** fechamento, até hoje; usado pelo Dashboard e pela tela.
  - `closing_repo::{movements_for_local_date(conn, day_start_utc, day_end_utc) -> Vec<(Method, received, refunded)>, find_by_date, insert}`; intervalo do dia local calculado de `ctx.utc.local_offset()` (nunca `date()` do UTC).
- Regras: data ≤ hoje; uma por data (`closing_date UNIQUE` ⇒ erro "Esse dia já foi fechado"); esperado **recomputado no servidor** (o front só envia os conferidos); linhas, diferenças e justificativas gravadas; **nenhuma** escrita em `payments`/`refunds`; auditoria `close_cash` na mesma transação; fechar não bloqueia nenhuma operação posterior (M5-11); `after_closing_cents` = esperado atual − esperado gravado, quando fechado.

- [ ] **Step 1: Write the failing tests:**
  - `expected_is_received_minus_refunded_per_method_with_debit_and_credit_apart` (Pix 10.000 recebido, devolvido 2.000 em Pix, débito 5.000, crédito 3.000 ⇒ esperado Pix 8.000, débito 5.000, crédito 3.000).
  - `closing_with_difference_and_justification_completes_and_stores_everything` (conferido −500 com justificativa; usuário/hora/linhas gravados; recebimentos inalterados — comparar `SELECT *` antes/depois).
  - `difference_without_justification_is_refused_without_writing`.
  - `missing_method_or_negative_count_is_refused`.
  - `closing_twice_for_the_same_date_is_refused` (duplo clique).
  - `future_date_is_refused`.
  - `past_date_can_be_closed_late_without_moving_movements` (fechar ontem hoje; o recebimento de hoje não entra).
  - `local_midnight_boundary_uses_the_local_date` (offset −03:00; recebimento às 23:30 locais = 02:30Z do dia seguinte entra no dia local anterior; 00:10 local entra no seguinte).
  - `pending_closing_never_blocks_other_operations` (com ontem pendente, registrar pagamento e devolução hoje funciona).
  - `movements_after_closing_are_reported_but_not_added` (pagamento depois de fechar ⇒ `after_closing_cents` > 0 e o fechamento gravado não muda).
  - `supplier_payments_are_listed_apart_and_not_subtracted` (M5-10).
  - `legacy_unspecified_movements_get_their_own_line`.
- [ ] **Step 2: Run** `cargo test closing_service` — Expected: FAIL.
- [ ] **Step 3: Implement** serviço e repositório.
- [ ] **Step 4: Run** `cargo test closing` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: daily cash closing`

---

### Task 8: Comandos Tauri e cliente da API

**Files:**
- Create: `src-tauri/src/commands/payments.rs`, `refunds.rs`, `payables.rs`, `closing.rs`
- Modify: `commands/mod.rs`, `lib.rs` (registro), `commands/stays.rs` (remove `record_stay_payment` provisório), `commands/reservations.rs` (remove `record_deposit`), `src/lib/api.ts`, `src/lib/api.test.ts`
- Test: `src/lib/api.test.ts`

**Interfaces:**
- Produces (comandos → funções TS): `record_stay_payment`/`recordStayPayment(token, stayId, PaymentInput)`, `record_reservation_payment`/`recordReservationPayment(token, reservationId, expectedVersion, PaymentInput)`, `record_refund`/`recordRefund`, `list_payables`, `get_payable`, `create_payable`, `pay_payable`, `payables_summary`, `list_expense_categories`, `create_expense_category`, `rename_expense_category`, `set_expense_category_active`, `closing_preview`, `close_cash`, `closing_pending_dates`. Tipos TS: `ComponentInput`, `PaymentInput { components, note, duplicate_release? }`, `PayableView`, `ClosingPreview`, etc. `PaymentMethod` perde `prior_deposit` e ganha `unspecified` (só leitura).
- O erro `duplicate_payment` chega ao front como `ApiError` com `code: "duplicate_payment"` e os dados do recebimento parecido (`details`), para o `DuplicateDialog`.

- [ ] **Step 1: Write the failing test** `api.test.ts`: cada função nova chama o comando certo com os argumentos em camelCase; `recordStayPayment` envia `components`; erro `duplicate_payment` preserva `details`.
- [ ] **Step 2: Run** `npx vitest run src/lib/api.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** comandos (finos: só `touch` de sessão + chamada ao serviço) e `api.ts`.
- [ ] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run src/lib/api.test.ts` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: finance commands and api client`

---

### Task 9: Interface — recebimento dividido, duplicidade e devolução

**Files:**
- Create: `src/pages/payments/ComponentsEditor.tsx`, `PaymentDialog.tsx` (substitui `pages/stays/PaymentDialog.tsx`), `DuplicateDialog.tsx`, `RefundDialog.tsx`, `src/lib/payment.ts`
- Modify: `src/pages/stays/StayPage.tsx` (lista de recebimentos com modalidades, devoluções, `Registrar devolução`), `src/pages/reservations/ReservationPage.tsx` (usa `PaymentDialog` no lugar de `DepositDialog`, mostra `Devolução a fazer: R$ …`), remover `pages/reservations/DepositDialog.tsx` e `pages/stays/PaymentDialog.tsx`, `src/lib/stay.ts` (rótulos)
- Test: `ComponentsEditor.test.tsx`, `PaymentDialog.test.tsx`, `RefundDialog.test.tsx`, `StayPage.test.tsx`, `ReservationPage.test.tsx`

**Interfaces:**
- Produces: `<ComponentsEditor value onChange />` (linhas modalidade + valor, botão `Dividir em outra forma`, mostra **soma, total e quanto falta/sobra**); `<PaymentDialog target={{ kind: "stay"|"reservation", … }} suggestedCents />`; `<DuplicateDialog existing canRelease onRelease />` (mostra `PAG-…`, valor, modalidades e hora; `Liberar recebimento duplicado` com justificativa só se o usuário tem `release_duplicate_payment`); `<RefundDialog payment refundableCents />`. `lib/payment.ts`: `sumComponents`, `methodLabel`, `remainingToAllocate`.

- [ ] **Step 1: Write the failing tests:**
  - `ComponentsEditor`: soma e diferença mostradas ("Falta R$ 20,00"); `Confirmar` só habilita com soma = total informado; débito e crédito aparecem como opções separadas; valores digitados em `R$` viram centavos.
  - `PaymentDialog`: envia `components` corretos; falha de validação **preserva os campos** e mostra a mensagem; erro `duplicate_payment` abre o `DuplicateDialog` sem perder os dados; usuário sem permissão não vê o botão de liberar e lê a explicação; resultado "Pagamento PAG-… de R$ … registrado e salvo neste computador."
  - `RefundDialog`: motivo obrigatório; valor acima do disponível mostra "no máximo R$ …"; sucesso recarrega a conta.
  - `StayPage`: recebimento dividido lista "Pix R$ 60,00 + Dinheiro R$ 40,00"; `Registrar devolução` aparece só com valor devolvível.
  - `ReservationPage`: cancelada com sinal mostra `Devolução a fazer: R$ 120,00` e o botão; some quando zero; `Registrar pagamento` substitui `Registrar sinal`.
- [ ] **Step 2: Run** `npx vitest run src/pages/payments src/pages/stays src/pages/reservations` — Expected: FAIL.
- [ ] **Step 3: Implement** os componentes (shadcn, rótulos exatos, estado nunca só por cor: ícone + texto na soma "Confere"/"Falta"/"Sobra").
- [ ] **Step 4: Run** os mesmos testes + `npx tsc --noEmit` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: split payment, duplicate review and refund screens`

---

### Task 10: Interface — contas a pagar e categorias

**Files:**
- Create: `src/pages/payables/PayablesPage.tsx`, `PayableDialog.tsx`, `PayPayableDialog.tsx`, `CategoriesPanel.tsx`
- Modify: `src/routes.tsx` (`/contas-a-pagar`), `src/components/layout/Sidebar.tsx` (item `Contas a pagar` visível com `manage_payables`; "Financeiro" deixa de ser módulo futuro), `src/components/layout/Sidebar.test.tsx`
- Test: `PayablesPage.test.tsx`, `PayableDialog.test.tsx`, `PayPayableDialog.test.tsx`, `CategoriesPanel.test.tsx`

**Interfaces:**
- Consumes: funções de T8; `usePermissions` (já existente).
- Produces: página com `DataTable` (fornecedor, descrição, categoria, vencimento, valor, pago, saldo, situação com **rótulo + ícone**: `Pendente`, `Parcial`, `Paga`, `Vencida`), filtros de situação/categoria/vencimento, alternador `Visão: Competência | Pagamento` com totais por categoria (M5-12), diálogos `Registrar conta a pagar` (com opção "Já paga à vista") e `Registrar pagamento da conta`, painel de categorias (criar, renomear, desativar; só `manage_catalog`).

- [ ] **Step 1: Write the failing tests:** situação com rótulo (não só cor); vencida destacada com texto; `Registrar pagamento da conta` mostra saldo e recusa acima dele preservando o campo; à vista cria a conta já paga; usuário sem `manage_payables` não vê o item de menu nem a rota (mensagem de acesso); painel de categorias oculto sem `manage_catalog`; alternar a visão chama `payablesSummary` com `basis` certo.
- [ ] **Step 2: Run** `npx vitest run src/pages/payables src/components/layout` — Expected: FAIL.
- [ ] **Step 3: Implement** páginas e rota.
- [ ] **Step 4: Run** os testes + `npx tsc --noEmit` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: accounts payable screens`

---

### Task 11: Interface — fechamento de caixa e aviso no Dashboard

**Files:**
- Create: `src/pages/closing/ClosingPage.tsx`, `ClosingDialog.tsx`
- Modify: `src/routes.tsx` (`/fechamento`, `/fechamento/:data`), `Sidebar.tsx` (`Fechamento de caixa`), `src/pages/Dashboard.tsx` (aviso `Fechamento pendente: DD/MM/AAAA` com link; nunca bloqueia)
- Test: `ClosingPage.test.tsx`, `ClosingDialog.test.tsx`, `Dashboard.test.tsx`

**Interfaces:**
- Produces: tela por data (padrão hoje; lista de datas pendentes): tabela por modalidade com **Recebido, Devolvido, Esperado, Conferido, Diferença**; campo de justificativa aparece **por linha** quando a diferença ≠ 0; botão `Concluir fechamento` com confirmação resumindo diferenças; depois de fechado, modo leitura com usuário e hora e, se houver, "Movimentos depois do fechamento: R$ …"; linha informativa "Pagamentos a fornecedores no dia" (M5-10); linha "Sem modalidade" só quando existir.

- [ ] **Step 1: Write the failing tests:** diferença calculada ao digitar o conferido; justificativa obrigatória só nas linhas com diferença e **não impede** concluir (preenchida); erro de gravação preserva todos os campos; fechado vira somente leitura; dia futuro não é oferecido; Dashboard mostra o aviso com a data pendente e some quando não há pendência.
- [ ] **Step 2: Run** `npx vitest run src/pages/closing src/pages/Dashboard.test.tsx` — Expected: FAIL.
- [ ] **Step 3: Implement** as telas.
- [ ] **Step 4: Run** os testes + `npx tsc --noEmit` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: cash closing screen and pending warning`

---

### Task 12: Seed, documentação e encerramento

**Files:**
- Modify: `src-tauri/src/seed.rs` (+ teste do seed), `docs/desenvolvimento/seed.md`, `CHANGELOG.md`, `local/spec.md` (Anexo A.5), `local/plano-completo.md` (marcar Marco 5), `README.md` se listar marcos, este plano
- Create: `docs/usuario/pagamentos-e-devolucoes.md`, `contas-a-pagar.md`, `fechamento-de-caixa.md`, `docs/desenvolvimento/financeiro.md`, `fechamento-de-caixa.md`
- Modify: `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md` (novos módulos: recebimento dividido e devolução; contas a pagar; fechamento; marcar **provisório**: M5-6, M5-7, M5-8, M5-10, M5-11, M5-13), `docs/usuario/conta-do-hospede.md`, `reservas.md`, `docs/desenvolvimento/{hospedagem-e-conta,reservas-e-disponibilidade,permissoes,sequencias}.md` (atualizar o que o Marco 5 substituiu: M3-3, M3-9, `record_deposit`)

- [ ] **Step 1: Seed:** recebimento dividido (Pix + dinheiro), uma devolução parcial, uma reserva cancelada com `refund_due`, três contas a pagar (pendente, parcial, vencida) e **um fechamento de ontem com diferença justificada**, deixando hoje pendente; teste `seed_creates_finance_scenario` (contagens e códigos).
- [ ] **Step 2: Anexo A.5** da spec com M5-6, M5-7, M5-8, M5-10, M5-11, M5-13 como **aguardando aprovação**; `CHANGELOG.md` (`Added`: recebimento dividido, devoluções, bloqueio de duplicidade, contas a pagar, fechamento; `Changed`: sinal vira recebimento, cancelamento passa a mostrar devolução a fazer; `Security`: duas permissões novas).
- [ ] **Step 3: Docs** de usuário (objetivo, quando usar, passo a passo, resultado, avisos, erros comuns), desenvolvimento (regras, permissões, auditoria, efeitos no banco, migração `0011`, limitações, testes), roteiro de treinamento com cenário do seed e quem faz o quê.
- [ ] **Step 4: Verificação completa:** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npm test`, `npx tsc --noEmit`, `npm run build`; registrar contagens no Status. (`cargo fmt --check` já falha na base; não aplicar.)
- [ ] **Step 5:** `graphify update .`; anotar no Status.
- [ ] **Step 6:** Revisão final da branch por revisor independente; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [ ] **Step 7: Commit** `docs: close marco 5`

---

## Marcos seguintes

- **Marco 6 — Alertas, relatórios e documentos:** alertas persistentes (saída atrasada, risco de conflito, limpeza urgente parada, **fechamento pendente**, **conta a pagar vencida**, **sinal vencido**), relatórios financeiros por competência e por pagamento, documentos internos.
- **Marco 7 — Backup e restauração.**

## Pendências conhecidas ao iniciar

- Aprovação dos valores provisórios anteriores (D-15 a D-23) segue aberta; o Marco 5 acrescenta os de A.5.
- Pergunta para a pousada: **fornecedor é pago com o dinheiro do caixa da recepção?** (M5-10).
- Marco 3 e Marco 4 ainda não foram integrados em `dev`; este marco ramifica da branch atual.

## Self-Review

- **Cobertura da spec:** §9.1 pagamentos parciais e divididos, soma exata, débito/crédito separados → T2/T4/T9; §9.2 sequências independentes, devolução vinculada com motivo → T4/T5; duplicidade bloqueada e liberada com justificativa/responsável/hora → T4/T9; §10.1 → M5-3 (já no Marco 3, teste de ponta a ponta em T5); §10.2 contas, estados, categorias, competência × pagamento → T6/T10; §11 fechamento → T7/T11; §13 permissões e auditoria → T3 e todas as tarefas; §19.2 valores/arredondamento → M5-2; roteiro 5.5 cancelamento → T5 (M5-8).
- **Consistência de tipos:** `Method`/`Component`/`validate_components` (T2) usados por T4, T5, T6; `PaymentInput`/`ComponentInput` (T4) refletidos em T8/T9; `AppError::DuplicatePayment` (T4) tratado em T8/T9; `ClosingPreview` (T7) usado por T11; `Permission::{ReleaseDuplicatePayment, ManagePayables}` (T3) usadas em T4/T6/T10.
- **Migrações:** `0011` única; Marco 6 usa `0012`.
- **Proporção:** corpos de função ficam com o implementador; os testes trazem valores exatos (R$ 60,00 + R$ 40,00, `PAG-000001`, `DEV-000001`, `CAP-000001`, −R$ 5,00 de diferença, 23:30 local).
