# Marco 3 — Hospedagem e conta do hóspede Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer o check-in (de reserva e walk-in), manter a conta da hospedagem (diárias, consumos, ajustes, estorno + relançamento, recebimentos) e concluir o check-out com demonstrativo, saldo, saída tardia e alerta de risco para a próxima reserva.

**Architecture:** Mesmas camadas dos marcos anteriores. Regras de máquina de estados, saldo e saída tardia ficam em **funções puras** (`domain/`), chamadas pelo `StayService` **dentro da transação de gravação**. A hospedagem (`stays`) é sempre ligada a uma linha de `reservations`, que continua sendo a **única fonte de ocupação do quarto** (disponibilidade, mapa e conflitos não mudam). A conta é um **livro de lançamentos imutável**: `saldo = soma(charges) − soma(payments) + soma(refunds)`; correção = lançamento de estorno (valor negativo) + novo lançamento.

**Tech Stack:** o dos marcos anteriores (Rust/rusqlite/SQLCipher, React/TS/Vitest). Sem dependências novas.

**Spec:** `local/spec.md` §4.4, §6, §9 (esquema apenas), §10.1, §13, §19 e Anexo A. Roteiro: `local/plano-completo.md` §7.

**Branch:** `marco-3-hospedagem`, a partir de `dev` (Marco 2 integrado).

## Decisões deste plano (chamadas minhas; revisar antes de executar)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| M3-1 | **Walk-in cria também uma linha em `reservations`** (`origin = 'walk_in'`, status `checked_in`, código `AVU-000001` da sequência `stay` (prefixo ajustado na revisão final: `HOS-` já é o do hóspede)). A hospedagem reaproveita o código da reserva. | Disponibilidade, mapa e conflitos continuam lendo só `reservations`. Custo: o walk-in aparece na lista de reservas (com rótulo "Sem reserva"). |
| M3-2 | Migração é a **`0009`** (o roteiro dizia `0004`; os números avançaram no Marco 2). Antecipa `payments` e `refunds` **mínimos**; serviços completos ficam no Marco 5. | Marco 5 migra o método para `payment_components`. |
| M3-3 | **Recebimento provisório** (`record_payment`: valor + uma modalidade, sem divisão) para a conta poder ser quitada na recepção. Marcado como provisório na interface e substituído no Marco 5. Não há serviço de devolução; `refunds` só entra no saldo. | Sem isso o check-out só fecharia por exceção. Mesmo precedente do `record_deposit` do Marco 2. |
| M3-4 | O sinal já pago (`deposit_paid_cents`) vira um `payments` com método `prior_deposit` no check-in; o desconto da reserva vira um lançamento `discount` negativo. Assim o saldo só tem uma fonte. | — |
| M3-5 | Check-in aceita reserva **pendente ou confirmada**; check-in em dia posterior ao previsto é permitido (a decisão de no-show é manual). | Pendente sem sinal pode hospedar: a recepção decide. |
| M3-6 | "Autorizador" do check-out com saldo = **o próprio usuário que tem `CheckoutWithBalance`**; não há aprovação por segunda pessoa. | Se a pousada quiser dupla aprovação, é um fluxo novo. |
| M3-7 | **Saída tardia (provisória, D-15):** passou da tolerância (global `late_checkout_tolerance_minutes`, ou a da hospedagem) ⇒ cobrança prevista = `late_checkout_fee_bp` do valor da **última diária** (padrão 5000 = 50%, configurável, 0 = sem cobrança). Aplica-se quando `minutos de atraso > tolerância`. | Número inventado para destravar; entra no Anexo A como **aguardando aprovação**. |
| M3-8 | Tolerância por hospedagem é ajustável por quem tem `ChangeReservation` (altera condição da estadia), com auditoria. | Spec diz só "ajustável por hospedagem". |
| M3-9 | Saldo diferente de zero no check-out (devedor **ou** com crédito) exige `CheckoutWithBalance`. | Devolução do crédito só existe no Marco 5. |
| M3-10 | **Limpeza:** o check-in confere o intervalo mínimo entre hospedagens (`find_conflicts`), mas **não** o estado "limpo" do quarto, que nasce no Marco 4. Ao sair, nada é gerado; o Marco 4 cria a tarefa de limpeza. | Pendência explícita; registrada no Status. |

## Global Constraints

- Check-in é **manual**; registra ocupantes, horário efetivo (UTC) e usuário; revalida disponibilidade, capacidade e intervalo de limpeza na transação (spec §6.1).
- Check-in antecipado (agora < data + horário previsto de entrada) **sempre** exige `EarlyCheckIn`, mesmo com quarto livre; excesso de capacidade exige `ExceedCapacity` (spec §6.1, §13.1).
- Lançamentos confirmados **nunca** são editados nem apagados; correção só por estorno (`ReverseEntry` + justificativa) e relançamento, sem cobrança duplicada (spec §6.2, §10.1; CLAUDE.md).
- Todo lançamento vincula-se a uma hospedagem em andamento e compõe o saldo (spec §10.1).
- Check-out: quitação exigida; com saldo ≠ 0 exige `CheckoutWithBalance`, justificativa, autorizador, data/hora e saldo em aberto; **nada é liquidado ou apagado automaticamente** (spec §6.3).
- Saída tardia **não gera diária automática**; a recepção confirma; dispensar exige `WaiveLateFee` + justificativa e preserva atraso e valor calculado (spec §6.4).
- Saída que atrasa só **alerta** o risco para a próxima reserva; não altera nem cancela nada (spec §4.3, §6.4).
- Operações relacionadas numa transação; auditoria na mesma transação; comparação de `version` (`Conflict`) em toda mudança de estado (spec §2.3, §13.4).
- Valores em centavos inteiros com aritmética checada (`domain::money`); datas `YYYY-MM-DD`; instantes UTC; relógios injetados.
- Rótulos de ação específicos: `Confirmar check-in`, `Concluir check-out`, `Lançar consumo`, `Registrar pagamento`, `Estornar lançamento`. Datas `DD/MM/AAAA`, valores `R$ 150,00`. Cor nunca é o único sinal de estado.
- Mensagens em português simples; nenhum dado pessoal em logs.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/desenvolvimento/`, este plano e o grafo a cada ciclo.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Duplo clique** em `Confirmar check-in` / `Concluir check-out` / `Estornar`: a segunda chamada é recusada (`Conflict`) e não duplica diárias, pagamento do sinal nem estornos.
2. **Reserva em estado errado** (cancelada, no-show, já hospedada, concluída) ou quarto desativado: check-in recusado, nada gravado.
3. **Estornar um estorno** ou estornar o mesmo lançamento duas vezes: recusado; o saldo não muda.
4. **Valores inválidos** em lançamentos e recebimentos (0, negativo onde não cabe, estouro de inteiro, descrição vazia ou só espaços): recusados sem gravação parcial.
5. **Hospedagem encerrada**: lançar, estornar ou receber nela é recusado; consultar continua funcionando.
6. **Check-out com crédito** (recebido > devido): exige a exceção e a mensagem diz que há valor a devolver.
7. **Saída muito atrasada** (dias depois, virada de mês): atraso calculado em minutos sem estouro, cobrança prevista única (nunca uma diária por dia).

---

## File Structure

```
src-tauri/src/
  migrations/sql/0009_stays.sql            # origin em reservations; stays, stay_occupants, catalog_items, charges, payments, refunds
  auth/permissions.rs                      # +EarlyCheckIn, CheckoutWithBalance, WaiveLateFee
  domain/stay.rs                           # máquina de estados (pura)
  domain/account.rs                        # saldo (puro)
  domain/late_checkout.rs                  # atraso, cobrança prevista, risco (puro)
  models/stay.rs, item.rs
  repositories/stay_repo.rs, item_repo.rs  # + reservation_repo::set_status
  services/stay_service.rs, item_service.rs
  commands/stays.rs, items.rs
src/
  pages/stays/StaysPage.tsx, StayPage.tsx, WalkInPage.tsx, CheckInDialog.tsx, ChargeDialog.tsx,
              ReverseDialog.tsx, PaymentDialog.tsx, CheckoutDialog.tsx
  pages/items/ItemsPage.tsx
  lib/api.ts, lib/stay.ts, routes.tsx, components/layout/Sidebar.tsx
docs/usuario/{hospedagem,conta-do-hospede}.md   docs/desenvolvimento/{hospedagem-e-conta,saida-tardia}.md
```

---

### Task 1: Novas permissões

**Files:**
- Modify: `src-tauri/src/auth/permissions.rs`, `src/pages/users/labels.ts`
- Test: os testes existentes em `permissions.rs`; `src/pages/users/` (teste de rótulos, se houver) ou o teste da tela de permissões

**Interfaces:**
- Produces: `Permission::{EarlyCheckIn, CheckoutWithBalance, WaiveLateFee}` com `as_str()` = `"early_check_in"`, `"checkout_with_balance"`, `"waive_late_fee"`; as três entram em `ALL` (no fim, preservando os bits atuais) e em `grantable()`.

- [x] **Step 1: Write the failing test** em `permissions.rs`: `grantable_includes_stay_permissions` (as três estão em `grantable()`), e o `permission_set_roundtrip` existente passa a cobrir `ALL`. Rótulos: o teste da tela de permissões lista "Fazer check-in antecipado", "Concluir check-out com saldo pendente" e "Dispensar cobrança de saída tardia".
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml permissions` — Expected: FAIL (variantes inexistentes).
- [x] **Step 3: Implement** as variantes no fim do enum, em `ALL` e `grantable()`; adicionar os três textos em `permissionLabel`.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml permissions` e `npm test -- users` — Expected: PASS.
- [x] **Step 5: Commit** `feat: add stay permissions`

---

### Task 2: Migração 0009 e esquema da hospedagem

**Files:**
- Create: `src-tauri/src/migrations/sql/0009_stays.sql`
- Modify: `src-tauri/src/migrations/mod.rs` (versão 9), `src-tauri/src/repositories/reservation_repo.rs` (coluna `origin`), `src-tauri/src/models/reservation.rs` (`Reservation.origin: String`, `ReservationDetail.stay_id: Option<i64>`)
- Test: `src-tauri/src/migrations/mod.rs` (testes de esquema existentes) e `reservation_repo`

**Interfaces:**
- Produces (tabelas; todas as datas `TEXT`, dinheiro em centavos `INTEGER`):
  - `ALTER TABLE reservations ADD COLUMN origin TEXT NOT NULL DEFAULT 'booking' CHECK (origin IN ('booking','walk_in'))`.
  - `stays(id, reservation_id UNIQUE NOT NULL → reservations, code UNIQUE, status CHECK IN ('in_house','checked_out'), checked_in_at, checked_in_by → users, early_check_in INTEGER, exceeded_capacity INTEGER, late_tolerance_minutes INTEGER NULL, checked_out_at NULL, checked_out_by NULL, late_minutes NULL, late_fee_calculated_cents NULL, late_fee_decision NULL CHECK IN ('charged','waived','not_applicable'), late_fee_waive_justification NULL, late_fee_waived_by NULL, balance_at_checkout_cents NULL, balance_justification NULL, balance_authorized_by NULL, version DEFAULT 1, created_at, updated_at)`.
  - `stay_occupants(id, stay_id → stays, full_name NOT NULL, guest_id NULL → guests, document TEXT NOT NULL DEFAULT '', is_holder INTEGER NOT NULL)`.
  - `catalog_items(id, name UNIQUE COLLATE NOCASE, default_price_cents CHECK > 0, active DEFAULT 1, version DEFAULT 1, created_at, updated_at)`.
  - `charges(id, stay_id → stays, kind CHECK IN ('night','consumption','extra','discount','adjustment','late_fee','reversal'), description NOT NULL, amount_cents CHECK <> 0, item_id NULL → catalog_items, night_date NULL, reverses_charge_id NULL UNIQUE → charges, replaces_charge_id NULL → charges, justification NULL, created_by → users, created_at)`; índice único parcial `(stay_id, night_date) WHERE kind = 'night'`.
  - `payments(id, code UNIQUE, reservation_id → reservations, stay_id NULL → stays, amount_cents CHECK > 0, method CHECK IN ('cash','pix','debit','credit','transfer','prior_deposit'), received_at, received_by → users, note NOT NULL DEFAULT '')`.
  - `refunds(id, code UNIQUE, payment_id → payments, stay_id NULL → stays, amount_cents CHECK > 0, reason NOT NULL, refunded_at, refunded_by → users)`.
  - `settings`: `late_checkout_fee_bp = '5000'` (provisório).
- `Reservation.origin` vem de `r.origin` no `SELECT` (acrescentar no fim da lista e do `map_reservation`).

- [x] **Step 1: Write the failing test** `migration_0009_creates_stay_schema` (todas as tabelas existem; `schema_migrations` chega a 9; `INSERT` em `charges` com `amount_cents = 0` falha; duas diárias da mesma noite na mesma hospedagem falham; dois estornos do mesmo lançamento falham) e `reservation_has_origin_booking_by_default` (reserva criada pelo serviço volta `origin == "booking"`).
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml migrations reservation` — Expected: FAIL.
- [x] **Step 3: Implement** o SQL acima, registrar `Migration { version: 9, … }`, estender `SELECT`/`map_reservation`/`Reservation`; `get` preenche `stay_id` com `SELECT id FROM stays WHERE reservation_id = ?`. Atualizar `ReservationNight`-consumers que constroem `Reservation` à mão (compilador aponta).
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` — Expected: PASS (suíte inteira).
- [x] **Step 5: Commit** `feat: stay schema (migration 0009)`

---

### Task 3: Funções puras — estados, saldo e saída tardia

**Files:**
- Create: `src-tauri/src/domain/stay.rs`, `domain/account.rs`, `domain/late_checkout.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Test: nos próprios arquivos (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `domain::stay::StayStatus { InHouse, CheckedOut }` com `parse(&str)`, `as_str()`, e `transition(from: StayStatus, to: StayStatus) -> Result<(), AppError>` (só `InHouse → CheckedOut`).
  - `domain::stay::can_check_in(reservation_status: &str) -> bool` (`pending`/`confirmed`).
  - `domain::account::balance(charges_cents: i64, received_cents: i64, refunded_cents: i64) -> Result<i64, AppError>` = `charges − received + refunded`, aritmética checada.
  - `domain::late_checkout::LateAssessment { minutes_late: i64, applies: bool, fee_cents: i64 }` e `assess(expected_out: NaiveDateTime, now: NaiveDateTime, tolerance_minutes: u32, last_night_cents: i64, fee_bp: u32) -> Result<LateAssessment, AppError>`: `minutes_late = max(0, now − expected_out)`; `applies = minutes_late > tolerance`; `fee_cents = percent_of(last_night_cents, fee_bp)` só quando `applies`, senão 0; `fee_bp = 0` ⇒ `applies = false`.
  - `domain::late_checkout::DepartureRisk { minutes_late: i64, ready_at: NaiveDateTime, overrun_minutes: i64 }` e `departure_risk(expected_out: NaiveDateTime, now: NaiveDateTime, cleaning_minutes: u32, next_start: NaiveDateTime) -> Option<DepartureRisk>`: só existe se `now > expected_out`; `ready_at = now + cleaning`; risco quando `ready_at > next_start`; `overrun_minutes = ready_at − next_start`.

- [x] **Step 1: Write the failing tests**
  - `stay`: `in_house_to_checked_out_is_valid`, `checked_out_cannot_go_back`, `same_state_transition_is_refused`, `only_pending_and_confirmed_can_check_in` (recusa `checked_in`, `completed`, `cancelled`, `no_show`).
  - `account`: `balance_is_charges_minus_received_plus_refunded` (40000, 12000, 0 ⇒ 28000; 40000, 52000, 2000 ⇒ −10000), `balance_overflow_is_refused` (`i64::MAX`, `-1`), `random_sequence_matches_sum` (200 operações pseudoaleatórias determinísticas — semente fixa com LCG — mantêm `balance == soma incremental`).
  - `late_checkout`: com saída prevista 23/10 12:00, tolerância 30, última diária 20000, `fee_bp` 5000: 12:30 ⇒ `applies=false, fee=0, minutes_late=30`; 12:31 ⇒ `applies=true, fee=10000`; 11:00 ⇒ `minutes_late=0`; 25/10 09:00 (45 h) ⇒ `minutes_late=2700`, `fee=10000` (uma só cobrança); `fee_bp=0` ⇒ `applies=false`; virada de ano (31/12 12:00 → 01/01 13:00) correta. `departure_risk`: saída 12:00, agora 13:30, limpeza 60, próxima entrada 14:00 ⇒ `ready_at 14:30`, `overrun 30`; agora 12:10, próxima 14:00 ⇒ `None`; agora antes da saída ⇒ `None`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml domain::` — Expected: FAIL.
- [x] **Step 3: Implement** as funções acima em `domain/` usando `domain::money::{percent_of, add, sub}`.
- [x] **Step 4: Run** o mesmo comando — Expected: PASS.
- [x] **Step 5: Commit** `feat: stay state, balance and late checkout rules`

---

### Task 4: Configuração da cobrança de saída tardia (D-15)

**Files:**
- Modify: `src-tauri/src/models/settings.rs`, `repositories/settings_repo.rs`, `services/settings_service.rs` (testes), `src/lib/api.ts` (`AppSettings`), `src/pages/settings/SystemForm.tsx`, fixtures de teste do frontend que montam `AppSettings` (`ReservationForm.test.tsx`, `SettingsPage.test.tsx`)
- Test: `settings_service.rs`, `SettingsPage.test.tsx`

**Interfaces:**
- Produces: `AppSettings.late_checkout_fee_bp: u32` (validação 0–10000, mensagem "Cobrança de saída tardia: informe de 0% a 100%."); na tela, campo percentual "Cobrança de saída tardia (% da última diária) — provisório".
- Consumes: `settings_repo::get_u32` para `"late_checkout_fee_bp"`.

- [x] **Step 1: Write the failing test** `settings_defaults_include_late_fee_5000` (valor inicial 5000), `late_fee_above_100_percent_is_refused` (10001), e no teste da tela: salvar "30" (%) grava `3000`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml settings` e `npm test -- SettingsPage` — Expected: FAIL.
- [x] **Step 3: Implement** campo no modelo, `load`/`save` do repo, validação, campo percentual no `SystemForm` (conversão % ↔ pontos-base como a sinal já faz); atualizar fixtures.
- [x] **Step 4: Run** os mesmos comandos — Expected: PASS.
- [x] **Step 5: Commit** `feat: configurable late checkout fee`

---

### Task 5: Catálogo de itens e serviços

**Files:**
- Create: `models/item.rs`, `repositories/item_repo.rs`, `services/item_service.rs`, `commands/items.rs`, `src/pages/items/ItemsPage.tsx`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `commands/mod.rs`, `app.rs`, `lib.rs`, `services/test_support.rs` (campo `items`), `src/lib/api.ts`, `routes.tsx`, `Sidebar.tsx`
- Test: `item_service.rs`, `ItemsPage.test.tsx`

**Interfaces:**
- Produces:
  - `Item { id, name, default_price_cents, active, version }`, `NewItem { name, default_price_cents }`, `ItemUpdate { name, default_price_cents }`.
  - `ItemService::{list(token, only_active: bool) -> Vec<Item>, create(token, NewItem) -> i64, update(token, id, ItemUpdate, expected_version) -> (), set_active(token, id, active, expected_version) -> ()}`. Criar/alterar/desativar exige `ManageCatalog`; listar, qualquer sessão. Itens nunca são apagados (histórico), só desativados.
  - Comandos `list_items`, `create_item`, `update_item`, `set_item_active`; funções `listItems`, `createItem`, `updateItem`, `setItemActive` em `api.ts`; rota `/itens`, link "Itens e serviços" no menu.

- [x] **Step 1: Write the failing tests** `create_item_requires_manage_catalog` (recepção ⇒ `Forbidden`), `item_name_is_unique_ignoring_case_and_accents_trimmed` (nome "Água mineral" e "  água MINERAL " ⇒ erro de validação), `price_must_be_positive` (0 e −1 recusados), `inactive_item_is_hidden_from_active_list_but_kept` , `stale_version_is_conflict`; tela: lista, cria "Água mineral — R$ 6,00", desativa.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml item_service` e `npm test -- ItemsPage` — Expected: FAIL.
- [x] **Step 3: Implement** seguindo `catalog_service`/`catalog_repo` (auditoria `module: "items"`, `Conflict` por `version`); página em tabela com diálogo de formulário como `RoomsPage`.
- [x] **Step 4: Run** os mesmos comandos — Expected: PASS.
- [x] **Step 5: Commit** `feat: item and service catalog`

---

### Task 6: Check-in de reserva

**Files:**
- Create: `models/stay.rs`, `repositories/stay_repo.rs`, `services/stay_service.rs`, `commands/stays.rs`
- Modify: `repositories/reservation_repo.rs` (`set_status`), `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `commands/mod.rs`, `app.rs`, `lib.rs`, `services/test_support.rs`
- Test: `stay_service.rs`

**Interfaces:**
- Consumes: `reservation_repo::{find, blocking, insert_change}`, `domain::availability::find_conflicts`, `domain::stay::can_check_in`, `sequence_repo`, `audit::record`, `require(&actor, Permission::…)`.
- Produces:
  - `reservation_repo::set_status(tx: &Transaction, id: i64, status: &str, expected_version: i64, at: DateTime<Utc>) -> Result<(), AppError>` (incrementa `version`; `Conflict` se a versão mudou).
  - `models::stay`: `OccupantInput { full_name: String, guest_id: Option<i64>, document: String }`, `CheckInInput { reservation_id: i64, expected_version: i64, occupants: Vec<OccupantInput>, exceed_capacity: bool }`, `Occupant { id, full_name, guest_id, document, is_holder }`, `Stay { id, code, reservation_id, status, room_id, room_code, guest_id, guest_name, check_in_date, check_out_date, expected_check_in_time, expected_check_out_time, checked_in_at, checked_out_at: Option<String>, early_check_in: bool, late_tolerance_minutes: Option<i64>, version }`, `StaySummary { stay: Stay, balance_cents: i64, minutes_late: i64, risk: Option<DepartureRiskView> }`, `StayDetail { stay: Stay, occupants: Vec<Occupant>, account: StayAccount }`, `StayAccount { charges: Vec<ChargeLine>, payments: Vec<PaymentLine>, refunds: Vec<RefundLine>, charges_total_cents, received_cents, refunded_cents, balance_cents }`, `ChargeLine { id, kind, description, amount_cents, night_date: Option<String>, justification: Option<String>, reverses_charge_id: Option<i64>, reversed: bool, created_by_name, created_at }`, `PaymentLine { id, code, amount_cents, method, received_at, received_by_name }`, `RefundLine { id, code, payment_code, amount_cents, reason, refunded_at }`, `StayQuery { text: Option<String>, status: Option<String> /* padrão in_house */, page, page_size }`.
  - `StayService::{check_in(token, CheckInInput) -> i64 /* stay_id */, get(token, id) -> StayDetail, list(token, StayQuery) -> Page<StaySummary>}`; comandos `check_in_reservation`, `get_stay`, `list_stays`.
  - `test_support`: `Harness.stays: StayService`; `pub struct Seed { pub category_id: i64, pub room_id: i64, pub guest_id: i64 }` e `pub fn seed(h: &Harness, admin: &str, nightly_cents: i64, capacity: i64) -> Seed` (categoria, quarto "101", hóspede e tarifa de todos os dias da semana — modele em `reservation_service::tests::{world,new_room,new_guest,weekday_rule}`, sem refatorar esses testes); `pub fn book(h, admin, seed, from, to) -> i64` (cria a reserva e devolve o id).
- Regras de `check_in` (tudo numa transação): reserva `pending|confirmed` e `version` igual; quarto ativo; ao menos 1 ocupante, nomes não vazios; `occupants.len() > max_capacity` ⇒ erro de validação, ou `Forbidden` sem `ExceedCapacity` quando `exceed_capacity = true`; `agora_local < data + horário previsto de entrada` ⇒ exige `EarlyCheckIn` e `early_check_in = 1`; revalida conflitos com `find_conflicts` usando início = `min(início previsto, agora)` e ignorando a própria reserva; cria `stays` (`code` = código da reserva), o titular (`is_holder`) mais os demais ocupantes, uma diária `night` por noite congelada em `reservation_nights`, um `discount` negativo se `discount_cents > 0` (justificativa = `discount_reason`), um `payments` `prior_deposit` com `deposit_paid_cents` se > 0 (código `PAG-`), muda a reserva para `checked_in`, grava `reservation_changes` (`kind = "check_in"`) e auditoria (`operation: "check_in"`).

- [x] **Step 1: Write the failing tests** (relógio de teste: "hoje" 21/10/2026, 09:00 local = 12:00Z; reserva 21→23/10, diária R$ 200,00, capacidade 2)
  - `check_in_creates_stay_nights_and_marks_reservation`: às 14:00 local (`h.utc.set(17:00Z)`), 1 ocupante ⇒ reserva `checked_in`; 2 diárias de 20000; `StayDetail.account.charges_total_cents == 40000`; código da hospedagem = código da reserva; titular `is_holder`.
  - `check_in_converts_discount_and_deposit`: desconto R$ 40,00 + sinal R$ 120,00 pago ⇒ cobranças `night`×2 e `discount −4000`; pagamento `prior_deposit 12000`; `balance_cents == 24000`.
  - `early_check_in_requires_permission`: às 09:00 local, recepção sem permissão ⇒ `Forbidden` e nada gravado; com `EarlyCheckIn` concedida ⇒ ok, `early_check_in == true`; administrador ok.
  - `early_check_in_blocked_while_room_still_occupied`: hospedagem A em andamento até 22/10 12:00, reserva B 22/10 14:00 antecipada para hoje ⇒ erro de validação citando o código de A.
  - `capacity_excess_needs_flag_and_permission`: 3 ocupantes em quarto de 2 ⇒ erro de validação; com `exceed_capacity` sem permissão ⇒ `Forbidden`; com permissão ⇒ ok e `exceeded_capacity = 1`.
  - `check_in_refuses_wrong_status` (`cancelled`, `no_show`, `checked_in`, `completed`) e `check_in_refuses_inactive_room`.
  - `double_submit_is_conflict_and_does_not_duplicate`: a segunda chamada com a mesma `expected_version` ⇒ `Conflict`; 2 diárias e 1 pagamento `prior_deposit` apenas.
  - `invalid_input_writes_nothing`: lista de ocupantes vazia e nome em branco ⇒ erro; contagem de `stays`, `charges`, `payments`, `audit_log` inalterada.
  - `check_in_is_audited_in_same_transaction`: 1 linha em `audit_log` (`module = "stays"`, `operation = "check_in"`).
  - `late_arrival_day_after_is_allowed`: reserva de 21→23/10 com check-in em 22/10 14:00 ⇒ ok.
  - `list_in_house_returns_only_open_stays_with_balance`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml stay_service` — Expected: FAIL.
- [x] **Step 3: Implement** modelos, `stay_repo` (inserts, `find`, listagem com paginação e ordenação em lista fixa, `account(conn, stay_id)`), `StayService::{check_in, get, list}`, comandos e registro em `lib.rs`/`app.rs`/`Harness`. `StaySummary.minutes_late` e `risk` ficam zerados/`None` até a Task 9.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` — Expected: PASS.
- [x] **Step 5: Commit** `feat: check-in with occupants and opening account`

---

### Task 7: Walk-in (hospedagem sem reserva)

**Files:**
- Modify: `services/stay_service.rs`, `repositories/reservation_repo.rs` (`NewRow` ganha `status: &str` e `origin: &str`; `insert` passa a usá-los; ajustar a chamada de `reservation_service::create` para `"pending"`/`"booking"`), `models/stay.rs`, `commands/stays.rs`, `lib.rs`
- Test: `stay_service.rs`, `reservation_service.rs` (regressão)

**Interfaces:**
- Consumes: `rate_service::quote_in`, o miolo de check-in da Task 6 (extrair `fn open_stay(tx, actor, reservation, occupants, early, exceeded, at) -> Result<i64>` e reutilizar).
- Produces: `WalkInInput { room_id: i64, guest_id: i64, check_out_date: String, expected_check_out_time: Option<String>, occupants: Vec<OccupantInput>, exceed_capacity: bool }`; `StayService::walk_in(token, WalkInInput) -> i64 /* stay_id */`; comando `walk_in`.
- Regras: entrada = hoje, horário previsto de entrada = hora local atual `HH:MM`; saída > hoje; sem tarifa em alguma noite ⇒ erro listando as datas (nunca tarifa zero); conflito considera início = agora; reserva criada com `status = 'checked_in'`, `origin = 'walk_in'`, código `AVU-` da sequência `stay`, `deposit_expected_cents = 0`, `deposit_due_date` = hoje; `EarlyCheckIn` não se aplica; capacidade como no check-in; auditoria `operation: "walk_in"`.

- [x] **Step 1: Write the failing tests**
  - `walk_in_creates_reservation_and_stay_with_hos_code`: hoje 09:00, 21→23/10 ⇒ código `HOS-000001`, reserva `origin == "walk_in"` e `checked_in`, 2 diárias, `balance_cents == 40000`.
  - `walk_in_refuses_occupied_room` (hospedagem em andamento) e `walk_in_refuses_missing_tariff` (mensagem contém as datas em `DD/MM/AAAA`).
  - `walk_in_respects_capacity_rules` (igual à Task 6).
  - `walk_in_with_past_or_same_day_checkout_is_refused`.
  - `walk_in_sequence_does_not_consume_reservation_numbers`: depois de um walk-in, a próxima reserva é `RES-000001`.
  - Regressão: `reservation_service::create` continua gravando `pending`/`booking`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml stay_service reservation_service` — Expected: FAIL.
- [x] **Step 3: Implement** a extração de `open_stay`, `walk_in`, comando e registro.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` — Expected: PASS.
- [x] **Step 5: Commit** `feat: walk-in stay`

---

### Task 8: Conta da hospedagem — lançar, ajustar, estornar e receber

**Files:**
- Modify: `services/stay_service.rs`, `repositories/stay_repo.rs`, `models/stay.rs`, `commands/stays.rs`, `lib.rs`
- Test: `stay_service.rs`

**Interfaces:**
- Produces:
  - `ChargeInput { kind: String /* "consumption" | "extra" */, description: String, amount_cents: i64, item_id: Option<i64> }` — com `item_id`, a descrição e o valor padrão vêm do item quando vazios/zero.
  - `AdjustmentInput { description: String, amount_cents: i64 /* ≠ 0, pode ser negativo */, justification: String }`.
  - `PaymentInput { amount_cents: i64, method: String /* cash|pix|debit|credit|transfer */, note: String }`.
  - `StayService::{add_charge(token, stay_id, ChargeInput) -> i64, add_adjustment(token, stay_id, AdjustmentInput) -> i64, reverse_charge(token, stay_id, charge_id, justification: String, replacement: Option<ChargeInput>) -> (), record_payment(token, stay_id, PaymentInput) -> i64 /* payment_id */}`; comandos `add_stay_charge`, `add_stay_adjustment`, `reverse_stay_charge`, `record_stay_payment`.
  - `add_adjustment` exige `ApplyDiscount` e justificativa; `reverse_charge` exige `ReverseEntry` e justificativa; `record_payment` é **provisório** (Marco 5) e grava `PAG-` da sequência `payment`.
- Regras: hospedagem precisa estar `in_house`; descrição `trim()` não vazia (≤ 200); valores via `domain::money` (sem estouro); `reverse_charge` grava uma linha `reversal` de valor `−original` com `reverses_charge_id`, nunca altera a original, recusa estornar `reversal` ou lançamento já estornado, e, com `replacement`, grava o novo lançamento com `replaces_charge_id` — **tudo na mesma transação**; incrementa `stays.version`; auditoria por operação (`add_charge`, `add_adjustment`, `reverse_charge`, `record_payment`) com valores antes/depois do saldo.

- [x] **Step 1: Write the failing tests** (hospedagem: reserva 21→23/10 R$ 200,00/noite, desconto R$ 40,00, sinal R$ 120,00 ⇒ saldo inicial 24000)
  - `add_charge_increases_balance`: consumo "Água" 3500 ⇒ saldo 27500; item do catálogo (R$ 6,00) sem valor informado ⇒ 600.
  - `reverse_and_relaunch_preserves_original`: estorna o consumo 3500 e relança 3000 ⇒ 3 linhas novas na ordem (original, `reversal −3500`, novo 3000); original inalterado e `reversed == true`; saldo 27000.
  - `reverse_twice_or_reverse_a_reversal_is_refused`: saldo e contagem de `charges` inalterados.
  - `reverse_requires_reverse_entry_and_justification`: recepção sem permissão ⇒ `Forbidden`; justificativa em branco ⇒ validação.
  - `adjustment_requires_apply_discount`: recepção ⇒ `Forbidden`; admin ajusta −1000 ⇒ saldo −1000; valor 0 ⇒ validação.
  - `invalid_amounts_and_descriptions_write_nothing`: 0, −1, `i64::MAX` (estoura ao somar), descrição vazia/só espaços, `kind` desconhecido, `item_id` inexistente ou inativo ⇒ erro; `charges` e `audit_log` inalterados.
  - `record_payment_reduces_balance`: 10000 em `pix` ⇒ saldo 14000; método `prior_deposit` ou desconhecido ⇒ recusado; valor ≤ 0 ⇒ recusado.
  - `closed_stay_rejects_new_entries`: após check-out (use `set_status` por SQL de teste) lançar, estornar e receber ⇒ erro; `get` continua funcionando.
  - `double_reverse_in_sequence_does_not_duplicate`: segunda chamada idêntica ⇒ erro e uma só linha `reversal`.
  - `balance_equals_sum_of_ledger_after_mixed_operations`: sequência fixa de 8 operações; `account.balance_cents == charges − received + refunded` recalculado por SQL.
  - `every_operation_is_audited`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml stay_service` — Expected: FAIL.
- [x] **Step 3: Implement** os serviços, repos e comandos; usar a restrição `UNIQUE (reverses_charge_id)` como segunda barreira contra estorno duplicado (traduzir a violação em erro de validação em linguagem simples).
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` — Expected: PASS.
- [x] **Step 5: Commit** `feat: stay account ledger with reversal and provisional payments`

---

### Task 9: Check-out, saída tardia e risco para a próxima reserva

**Files:**
- Modify: `services/stay_service.rs`, `repositories/stay_repo.rs`, `models/stay.rs`, `commands/stays.rs`, `lib.rs`
- Test: `stay_service.rs`

**Interfaces:**
- Consumes: `domain::late_checkout::{assess, departure_risk}`, `domain::account::balance`, `domain::stay::transition`, `settings_repo::load` (`late_checkout_tolerance_minutes`, `late_checkout_fee_bp`, `cleaning_interval_minutes`).
- Produces:
  - `LateInfo { minutes_late: i64, tolerance_minutes: u32, applies: bool, fee_cents: i64 }`. (`DepartureRiskView { next_reservation_id: i64, next_code: String, next_start: String, ready_at: String, overrun_minutes: i64 }` é **declarado na Task 6** em `models/stay.rs`, porque `StaySummary` o referencia; a Task 9 só o preenche.)
  - `CheckoutPreview { stay: Stay, account: StayAccount, late: LateInfo, balance_cents: i64, can_checkout_without_exception: bool, risk: Option<DepartureRiskView> }`.
  - `LateFeeChoice` (enum serde, `snake_case`): `NotApplicable`, `Charge`, `Waive { justification: String }`.
  - `BalanceException { justification: String }`.
  - `CheckoutInput { stay_id: i64, expected_version: i64, late_fee: LateFeeChoice, balance_exception: Option<BalanceException> }`.
  - `StayService::{preview_checkout(token, stay_id) -> CheckoutPreview, checkout(token, CheckoutInput) -> (), set_late_tolerance(token, stay_id, minutes: Option<u32>, expected_version: i64) -> ()}`; comandos `preview_stay_checkout`, `checkout_stay`, `set_stay_late_tolerance`. `StaySummary.minutes_late` e `risk` passam a ser preenchidos (próxima reserva bloqueante do mesmo quarto, por `reservation_repo::blocking`, com início > agora).
- Regras de `checkout` (uma transação): hospedagem `in_house` e `version` igual; agora local → `assess` com a tolerância da hospedagem (ou a global) e a última diária **não estornada**; se `applies`: `late_fee` deve ser `Charge` (grava `late_fee` com o valor calculado) ou `Waive` (exige `WaiveLateFee` + justificativa; **não** grava lançamento; preserva `late_minutes`, `late_fee_calculated_cents`, `late_fee_decision = 'waived'`, justificativa e autor) — `NotApplicable` quando `applies` ⇒ validação; quando não `applies`, `late_fee` deve ser `NotApplicable`; **nenhuma diária é criada**; saldo recalculado depois da cobrança; saldo ≠ 0 sem exceção ⇒ validação (mensagem distinta para saldo devedor e para crédito do hóspede), com exceção exige `CheckoutWithBalance` + justificativa e grava `balance_at_checkout_cents`, justificativa, autorizador; depois `stays` → `checked_out` (`transition`), reserva → `completed` (`set_status`), `reservation_changes` (`kind = "check_out"`), auditoria (`operation: "check_out"`).
- Tolerância: `set_late_tolerance` exige `ChangeReservation`, valor 0–600 ou `None` (volta ao padrão), só em hospedagem aberta.

- [x] **Step 1: Write the failing tests** (reserva 21→23/10, R$ 200,00/noite; saída prevista 23/10 12:00; tolerância 30; taxa 5000; check-in feito às 14:00 do dia 21; sinal R$ 120,00 e desconto R$ 40,00 ⇒ saldo 24000)
  - `checkout_with_zero_balance_completes_stay_and_reservation`: pagamento de 24000 em `cash`, check-out às 12:00 de 23/10 ⇒ hospedagem `checked_out`, reserva `completed`, `checked_out_at` preenchido.
  - `checkout_with_balance_without_exception_is_refused`; `…with_exception_without_permission_is_forbidden` (recepção); `…with_permission_stores_balance_justification_and_authorizer` (`balance_at_checkout_cents = 24000`, nada liquidado, `charges` e `payments` inalterados).
  - `checkout_with_credit_requires_exception_and_says_so`: recebido 30000 além do devido ⇒ mensagem menciona valor a devolver.
  - `late_checkout_within_tolerance_has_no_fee` (12:30) e `…after_tolerance_requires_decision` (12:45: `NotApplicable` ⇒ validação; `Charge` ⇒ lançamento `late_fee 10000` e saldo 34000 antes de quitar).
  - `late_checkout_never_creates_a_night`: depois do check-out tardio o número de lançamentos `night` continua 2.
  - `waive_late_fee_requires_permission_and_keeps_history`: recepção ⇒ `Forbidden`; com `WaiveLateFee` + justificativa ⇒ sem lançamento, `late_minutes = 45`, `late_fee_calculated_cents = 10000`, decisão `waived`.
  - `per_stay_tolerance_overrides_global`: tolerância 120 em 12:45 ⇒ `applies = false`; `set_late_tolerance` sem `ChangeReservation` ⇒ `Forbidden`.
  - `very_late_checkout_charges_once`: 25/10 09:00 ⇒ `minutes_late = 2700`, uma só cobrança de 10000.
  - `double_checkout_is_conflict`: segunda chamada ⇒ `Conflict`; um só `late_fee`.
  - `checkout_failure_rolls_back_everything`: permissão negada depois do cálculo não deixa `late_fee` gravado.
  - `late_departure_flags_risk_without_touching_next_reservation`: próxima reserva no mesmo quarto 23/10 14:00, agora 13:30 ⇒ `risk.overrun_minutes = 30`; status da próxima reserva inalterado; sem próxima reserva ⇒ `risk = None`.
  - `preview_checkout_matches_checkout_values` (a prévia mostra os mesmos números que o check-out grava) e `preview_does_not_write`.
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml stay_service` — Expected: FAIL.
- [x] **Step 3: Implement** `preview_checkout`, `checkout`, `set_late_tolerance`, o preenchimento de `risk`/`minutes_late` na listagem e os comandos. Compartilhar o cálculo entre prévia e gravação numa função privada (`plan_checkout`) para que não divirjam.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` e `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — Expected: PASS, sem avisos.
- [x] **Step 5: Commit** `feat: checkout with balance, late fee and departure risk`

---

### Task 10: Interface — contratos, lista de hospedagens e tela da hospedagem

**Files:**
- Create: `src/lib/stay.ts`, `src/pages/stays/StaysPage.tsx`, `src/pages/stays/StayPage.tsx`
- Modify: `src/lib/api.ts` (tipos e funções de todos os comandos das Tasks 5–9), `src/routes.tsx` (`/hospedagens`, `/hospedagens/:id`), `src/components/layout/Sidebar.tsx` ("Hospedagens"), `src/pages/reservations/ReservationPage.tsx` (link "Ver hospedagem" quando `stay_id`), `src/pages/reservations/ReservationsPage.tsx` (rótulo "Sem reserva" para `origin = 'walk_in'`)
- Test: `StaysPage.test.tsx`, `StayPage.test.tsx`

**Interfaces:**
- Produces: tipos `Stay`, `StayDetail`, `StayAccount`, `ChargeLine`, `PaymentLine`, `StaySummary`, `CheckoutPreview` espelhando o Rust; funções `checkInReservation(token, input)`, `walkIn(token, input)`, `getStay(token, id)`, `listStays(token, query)`, `addStayCharge`, `addStayAdjustment`, `reverseStayCharge`, `recordStayPayment`, `previewStayCheckout`, `checkoutStay`, `setStayLateTolerance`; `lib/stay.ts`: `stayStatusLabel(status)` ("Hospedado" / "Encerrada"), `chargeKindLabel(kind)` ("Diária", "Consumo", "Despesa extra", "Desconto", "Ajuste", "Saída tardia", "Estorno"), `paymentMethodLabel(method)`.
- `StaysPage`: tabela das hospedagens em andamento (código, quarto, hóspede, saída prevista, saldo, aviso "Saída atrasada há 45 min" e "Risco para a próxima reserva" em **texto + ícone**), filtro Em andamento/Encerradas, botão "Hospedagem sem reserva". `StayPage`: dados e horários, ocupantes, conta em ordem de lançamento (estornados riscados **com o rótulo "Estornado"**), recebimentos, saldo em destaque ("Saldo a receber" / "Quitado" / "Crédito do hóspede"), ações com rótulos específicos (as ações abrem os diálogos das Tasks 11 e 12; aqui apenas os botões, habilitados por `can(...)` só para decidir o que mostrar).

- [x] **Step 1: Write the failing tests** `StaysPage`: lista com saldo formatado `R$ 240,00`, mostra "Saída atrasada há 45 min" e "Risco para a próxima reserva" em texto, vazio ⇒ "Nenhuma hospedagem em andamento"; `StayPage`: linha estornada exibe "Estornado", saldo "Saldo a receber R$ 240,00", hospedagem encerrada não mostra botões de lançar/receber/concluir.
- [x] **Step 2: Run** `npm test -- Stay` — Expected: FAIL.
- [x] **Step 3: Implement** tipos, funções, rotas, menu e as duas páginas (padrão de `ReservationsPage`/`ReservationPage` com TanStack Query).
- [x] **Step 4: Run** `npm test` e `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: stays list and stay page`

---

### Task 11: Interface — check-in de reserva e hospedagem sem reserva

**Files:**
- Create: `src/pages/stays/CheckInDialog.tsx`, `src/pages/stays/WalkInPage.tsx`
- Modify: `src/pages/reservations/ReservationPage.tsx` (botão `Confirmar check-in` para `pending|confirmed`), `src/routes.tsx` (`/hospedagens/nova`)
- Test: `CheckInDialog.test.tsx`, `WalkInPage.test.tsx`

**Interfaces:**
- Consumes: `checkInReservation`, `walkIn`, `usePermissions().can`, `useUnsavedChanges`/`UnsavedChangesGuard` como no formulário de reserva.
- Produces: `CheckInDialog({ reservation, open, onClose })` com lista de ocupantes (titular já preenchido, adicionar/remover), aviso destacado "Check-in antes do horário previsto — exige autorização" quando a hora local < previsto (e botão desabilitado com a dica da permissão se `!can("early_check_in")`), aviso de capacidade com opção "Exceder a capacidade" (visível só com `can("exceed_capacity")`), resumo "Saldo inicial: R$ …", botão **Confirmar check-in**; após salvar navega para `/hospedagens/:id` com aviso "Check-in confirmado e salvo neste computador." `WalkInPage`: quarto, hóspede (busca), saída, ocupantes, preço das noites ao vivo (reuso de `quoteStay`), botão **Confirmar check-in**.
- Erros do backend (conflito, tarifa faltando, capacidade) aparecem em texto simples **sem limpar o que foi preenchido**.

- [x] **Step 1: Write the failing tests** `CheckInDialog`: titular pré-preenchido; remover todos os ocupantes mostra "Informe ao menos um ocupante" e não chama a API; check-in antecipado mostra o aviso e, sem a permissão, desabilita o botão com a explicação; erro `validation` do backend mantém os campos; sucesso navega e mostra o aviso. `WalkInPage`: preço total ao vivo `R$ 400,00` para 2 noites; sem tarifa mostra as datas; envia `walk_in` com os ocupantes.
- [x] **Step 2: Run** `npm test -- CheckIn WalkIn` — Expected: FAIL.
- [x] **Step 3: Implement** os componentes seguindo `ReservationFormPage` (React Hook Form + Zod).
- [x] **Step 4: Run** `npm test` e `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: check-in dialog and walk-in page`

---

### Task 12: Interface — lançar consumo, estornar e registrar pagamento

**Files:**
- Create: `src/pages/stays/ChargeDialog.tsx`, `ReverseDialog.tsx`, `PaymentDialog.tsx`
- Modify: `src/pages/stays/StayPage.tsx` (abre os diálogos; linha a linha "Estornar lançamento")
- Test: `ChargeDialog.test.tsx`, `ReverseDialog.test.tsx`, `PaymentDialog.test.tsx`

**Interfaces:**
- Consumes: `addStayCharge`, `addStayAdjustment`, `reverseStayCharge`, `recordStayPayment`, `listItems`, `parseMoney`/`formatMoney`.
- Produces: `ChargeDialog` (item do catálogo **ou** descrição + valor; aba "Ajuste" só com `can("apply_discount")`; botão **Lançar consumo**); `ReverseDialog` (mostra o lançamento, exige justificativa, opção "Relançar com novo valor/descrição", confirma por ser financeiro; botão **Estornar lançamento**; só com `can("reverse_entry")`); `PaymentDialog` (valor, modalidade, observação; selo **"provisório"**; confirmação com o valor, como o `DepositDialog`; botão **Registrar pagamento**).
- Valores digitados como `R$ 35,00` viram centavos; valor inválido ou zero mostra erro e preserva o formulário; após salvar, a conta recarrega e o saldo novo é mostrado.

- [x] **Step 1: Write the failing tests** `ChargeDialog`: escolher item preenche descrição/valor; "0" ⇒ "Informe um valor maior que zero" sem chamar a API; envia centavos (`3500`); `ReverseDialog`: sem justificativa não envia; com "relançar" envia `replacement`; `PaymentDialog`: pede confirmação com `R$ 100,00` antes de enviar; modalidade obrigatória; erro mantém os campos.
- [x] **Step 2: Run** `npm test -- ChargeDialog ReverseDialog PaymentDialog` — Expected: FAIL.
- [x] **Step 3: Implement** os três diálogos reutilizando `ConfirmDialog`/`FormField`.
- [x] **Step 4: Run** `npm test` e `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: charge, reversal and payment dialogs`

---

### Task 13: Interface — Concluir check-out

**Files:**
- Create: `src/pages/stays/CheckoutDialog.tsx`
- Modify: `src/pages/stays/StayPage.tsx` (botão **Concluir check-out**, campo de tolerância da hospedagem)
- Test: `CheckoutDialog.test.tsx`

**Interfaces:**
- Consumes: `previewStayCheckout`, `checkoutStay`, `setStayLateTolerance`, `can(...)`.
- Produces: `CheckoutDialog` com o demonstrativo (diárias, consumos e despesas, descontos e ajustes, recebimentos, devoluções, saldo final); bloco de saída tardia ("Saída 45 min depois do previsto; cobrança prevista R$ 100,00") com a escolha **Cobrar saída tardia** / **Dispensar cobrança** (justificativa; só com `can("waive_late_fee")`) e a nota "Nenhuma diária é criada automaticamente"; bloco de saldo: quitado ⇒ botão **Concluir check-out**; com saldo ⇒ texto "Saldo em aberto: R$ 240,00" e a opção "Concluir com saldo em aberto" (justificativa; só com `can("checkout_with_balance")`, senão explica quem pode autorizar); aviso do risco "Esta saída pode atrasar a chegada da reserva RES-… (≈ 30 min)". Depois de salvar: "Check-out concluído e salvo neste computador."

- [x] **Step 1: Write the failing tests** quitado ⇒ confirma e chama `checkoutStay` com `late_fee: "not_applicable"`; saída tardia ⇒ botão desabilitado até escolher cobrar ou dispensar; dispensar sem justificativa ⇒ erro; sem `waive_late_fee` não mostra "Dispensar"; saldo com permissão ⇒ envia `balance_exception`; saldo sem permissão ⇒ botão desabilitado com explicação; crédito do hóspede mostra "Valor a devolver ao hóspede R$ …"; erro do backend mantém o diálogo aberto com a mensagem.
- [x] **Step 2: Run** `npm test -- CheckoutDialog` — Expected: FAIL.
- [x] **Step 3: Implement** o diálogo e a edição de tolerância (visível com `can("change_reservation")`).
- [x] **Step 4: Run** `npm test` e `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: checkout dialog`

---

### Task 14: Documentação, changelog, spec e fechamento

**Files:**
- Create: `docs/usuario/hospedagem.md`, `docs/usuario/conta-do-hospede.md`, `docs/desenvolvimento/hospedagem-e-conta.md`, `docs/desenvolvimento/saida-tardia.md`
- Modify: `docs/usuario/README.md`, `docs/desenvolvimento/README.md`, `docs/desenvolvimento/permissoes.md` (as três permissões novas), `docs/desenvolvimento/reservas-e-disponibilidade.md` (estados `checked_in`/`completed`, walk-in), `local/spec.md` (Anexo A: D-15 e M3-5/M3-6/M3-8/M3-9 como **provisórios**), `CHANGELOG.md`, este plano

**Interfaces:**
- Produces: documentação em linguagem simples (como fazer check-in, hospedagem sem reserva, lançar consumo, corrigir um lançamento por estorno, registrar pagamento provisório, concluir check-out, o que é saída tardia e quando a cobrança aparece, o que fazer com saldo em aberto) e documentação técnica (modelo `stays`/`charges`/`payments`, livro de lançamentos e fórmula do saldo, decisões M3-1 a M3-10, permissões, auditoria, erros, limitações e testes).

- [x] **Step 1:** Escrever a documentação e o `CHANGELOG.md` (`Added`: hospedagem, walk-in, conta, estorno, recebimento provisório, check-out com saldo e saída tardia, catálogo de itens, novas permissões; `Changed`: reservas passam a `checked_in`/`completed`, configuração de saída tardia).
- [x] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, `npm test`, `npm run build`; depois `graphify update .`.
- [x] **Step 3:** Atualizar a seção Status deste plano (testes, desvios, pendências) e marcar o Marco 3 em `local/plano-completo.md`; listar o Marco 4 em "Marcos seguintes".
- [x] **Step 4:** Revisão final da branch por revisor independente; corrigir Críticos/Importantes (TDD); registrar menores.
- [x] **Step 5: Commit** `docs: close marco 3`

---

## Status

**Implementado e revisado** na branch `marco-3-hospedagem` (T1 a T14 e revisão final). Migração `0009`.

- **Testes ao fechar a implementação:** `cargo test` 295 unitários + 4 de integração (após a revisão final); `cargo clippy --all-targets -- -D warnings` limpo; `npm test` 261 (36 arquivos); `npm run build` ok (`tsc` + `vite build`); `graphify update .` executado.
- **Desvios (rulings):** `catalog_items` usa `name_key UNIQUE` (unicidade sem acento/caixa) em vez de `COLLATE NOCASE`; a lista de ocupantes do check-in/walk-in **deve incluir o titular** (guest_id da reserva) e o titular é fixo na tela, então o aviso "Informe ao menos um ocupante" virou "Informe o nome de todos os ocupantes"; `CheckoutPreview` traz `balance_cents` e `balance_with_late_fee_cents` em vez de `can_checkout_without_exception`; `lançar/estornar/receber` incrementam `stays.version` (prévia de check-out antiga é recusada com `Conflict`); teto de R$ 1.000.000,00 por lançamento/recebimento e R$ 100.000,00 por item.
- **Revisão final (revisor independente, modelo mais capaz):** 1 Crítico e 3 Importantes, todos corrigidos com teste RED→GREEN (suíte 295 + 4): (1) hospedagem em andamento atrasada **não bloqueava** o quarto (`blocking` agora sempre devolve `checked_in` e estende a ocupação até `agora`); (2) check-in depois da saída prevista da própria reserva passa a ser recusado; (3) cobrança de saída tardia de R$ 0,00 deixa de "aplicar" (antes, "Cobrar" falhava com erro técnico); (7) walk-in passa a usar o prefixo `AVU-` (antes colidia com `HOS-` do hóspede). Rulings: o achado 4 (hora real da saída/limpeza) fica para o Marco 4 e D-19/M3-10 foram reescritos.
- **Menores adiados:** (a) check-out: prévia que cruza a tolerância entre ver e confirmar não recarrega sozinha (fechar e reabrir resolve); (b) falha ao carregar a prévia do check-out mostra "Carregando…" sem a mensagem; (c) `guest_id` de ocupante inexistente ou titular repetido gera erro técnico/duplicidade; (d) a tela de hospedagem sem reserva checa disponibilidade a partir das 14:00 padrão, enquanto o backend usa "agora" (o backend é quem decide).
- **Nota de processo:** nas Tasks 11 e 12 a falha RED foi observada por módulo inexistente (import), e na Task 11 a execução RED isolada não foi rodada antes da implementação; os testes cobrem comportamento e passaram depois.
- **Pendências para outros marcos:** estado "quarto limpo" no check-in e tarefa de limpeza ao encerrar (Marco 4, M3-10); recebimentos divididos, devoluções, duplicidade e fechamento de caixa (Marco 5; `record_payment` é provisório e `refunds` só entra no saldo); alertas persistentes de saída atrasada/risco, relatórios e documentos (Marco 6); aprovação dos valores provisórios D-15 a D-19 (`local/spec.md`, Anexo A.3).
- **Verificação manual:** `npm run tauri dev` com a interface real ainda não foi exercitado (fica para o responsável).

## Marcos seguintes

- **Marco 4 — Limpeza, manutenção e bloqueios:** gera a tarefa de limpeza ao encerrar a hospedagem, estado operacional do quarto e liberação (`ReleaseRoom`); passa a validar "quarto limpo" no check-in (M3-10).
- **Marco 5 — Financeiro:** substitui o recebimento provisório por recebimentos divididos, devoluções, duplicidade e fechamento de caixa (`payment_components`).

## Self-Review


- **Cobertura do roteiro §7:** 3.1 → T2/T3/T6; 3.2 → T6; 3.3 → T7; 3.4 → T8; 3.5 → T5; 3.6 → T9; 3.7 → T3/T4/T9; 3.8 → T3/T9; 3.9 → T10–T13. Spec §6.1–6.4 e §10.1 cobertos; §9 apenas no esquema e no recebimento provisório.
- **Consistência de tipos:** `OccupantInput` (T6) usado por T7; `StayAccount`/`ChargeLine` (T6) por T8–T10 e T13; `LateInfo`/`CheckoutPreview` (T9) por T13; `Permission::*` (T1) por T6, T8, T9; `late_checkout_fee_bp` (T2 seed, T4) por T9.
- **Valores provisórios:** nenhum número comercial é constante no código; D-15 vive em `settings` e aparece marcado como provisório na interface.
- **Migrações:** `0009` única neste marco; as seguintes do roteiro avançam (financeiro `0010`, alertas `0011`…).
- **Proporção:** corpos de função a cargo do implementador; testes trazem os valores exatos (saldo 24000, saída tardia 10000, 2700 minutos, `HOS-000001`).
