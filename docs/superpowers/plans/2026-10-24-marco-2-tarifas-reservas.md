# Marco 2 — Tarifas e reservas Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Criar, consultar, alterar, cancelar e marcar no-show de reservas com preço calculado pela tabela de tarifas e disponibilidade validada no backend, mais o mapa de quartos por dia.

**Architecture:** Mesmas camadas dos marcos anteriores. Regras de preço, disponibilidade, arredondamento e política de cancelamento ficam em **funções puras** (`domain/`), testadas sem banco e chamadas pelos serviços **dentro da transação de gravação** (revalidação). O preço acordado é congelado por noite (`reservation_nights`). Toda política comercial é **configuração provisória** (Anexo A da spec), nunca constante no código.

**Tech Stack:** o dos marcos anteriores (Rust/rusqlite/SQLCipher, React/TS/Vitest). Sem dependências novas.

**Spec:** `local/spec.md` §4, §5, §19.2 e Anexo A (valores provisórios D-02 a D-14). Decisões para aprovação: `local/revisao-decisoes-marco-2.md`. Roteiro: `local/plano-completo.md` §6.

**Branch:** `marco-2-reservas`, a partir de `dev` (Marcos 0 e 1 integrados).

## Global Constraints

- Regras críticas, permissões e validações revalidadas no backend no momento da gravação; a disponibilidade é checada **na mesma transação** que grava a reserva (spec §2.2, §2.3, §4.1).
- Toda reserva criada **bloqueia o quarto imediatamente**, inclusive pendente (spec §4.1).
- Noite sem tarifa impede a criação e lista as datas sem preço; nunca tarifa zero (spec §4.1).
- Prioridade de tarifa: exceção de data > feriado > temporada > dia da semana; **a data de saída não gera diária**; preço acordado congelado por noite e não muda quando a tabela muda (spec §5).
- Alteração de datas/quarto exige `ChangeReservation`, mostra preço anterior, novo e diferença, só grava com confirmação explícita e registra antes/depois, usuário, data/hora e justificativa (spec §4.2).
- Cancelamento preserva o registro; exceção de multa só com `CancellationException` e justificativa auditada, mantendo o valor calculado (spec §4.3).
- No-show e liberação do quarto são decisões **manuais**; sinal vencido só **gera alerta**; nada é cancelado ou liberado automaticamente (spec §4.3).
- Desconto/ajuste exige `ApplyDiscount` e registra valor original, final, justificativa, usuário e data/hora (spec §5).
- Excesso de capacidade exige `ExceedCapacity` (spec §13.1).
- Valores em centavos inteiros, aritmética checada; percentuais arredondados ao centavo mais próximo (meio centavo sobe), aplicados **uma vez sobre o total** (D-12; spec §19.2).
- Datas de calendário `YYYY-MM-DD`; instantes UTC; relógios injetados (proibido `Utc::now()` fora de `SystemUtc`).
- Reserva e hospedagem conservam o mesmo código; sequência `reservation` (`RES-000001`), sem reuso de números confirmados (spec §3.3).
- Toda edição compara `version` (`Conflict`); listagens paginadas com colunas de ordenação em lista fixa.
- Mensagens em português simples; CPF só mascarado em auditoria; nenhum dado pessoal em logs.
- Valores provisórios (D-xx) sempre em `settings`/tabelas, marcados como provisórios na interface e na documentação.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/desenvolvimento/`, este plano e o grafo a cada ciclo (regra do `CLAUDE.md`).
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Corrida de reservas:** duas reservas simultâneas para o mesmo quarto e período; exatamente uma é gravada (a checagem de disponibilidade e o `INSERT` estão na mesma transação).
2. **Fronteiras de data:** saída e entrada no mesmo dia (permitida só se saída prevista + intervalo de limpeza ≤ entrada seguinte), reserva de 1 noite, virada de ano/mês, ano bissexto (29/02/2028), `check_out <= check_in` recusado.
3. **Preço:** tarifa de dia de semana vs. feriado vs. temporada vs. exceção de data na mesma noite; temporada que cruza o ano (15/12 a 28/02); noite sem tarifa; tarifa mudada depois não altera reserva existente.
4. **Dinheiro:** percentual sobre valor com fração de centavo (30% de R$ 1.003,35 = R$ 301,01); soma das noites = total; estouro de inteiro recusado; desconto maior que o total recusado.
5. **Alteração sem confirmação:** a prévia nunca grava; confirmar com preço desatualizado (tabela mudou entre a prévia e a confirmação) é recusado e pede nova prévia.
6. **Permissões:** recepção sem permissão não altera, não dá desconto, não aplica exceção de cancelamento nem excede capacidade, mesmo chamando o comando pelo nome.
7. **Estados:** transições inválidas (cancelar reserva já com check-in, reativar cancelada, no-show antes do horário) recusadas.

---

## File Structure

```
src-tauri/src/
  migrations/sql/0006_rates_and_policies.sql    # rate_rules, holidays, cancellation_tiers, settings novas
  migrations/sql/0007_reservations.sql          # reservations, reservation_nights, reservation_changes
  domain/money.rs                               # percent_of, checked ops
  domain/pricing.rs                             # price_stay (puro)
  domain/availability.rs                        # conflitos e turnover (puro)
  domain/cancellation.rs                        # multa/devolução pela política (puro)
  models/rate.rs, reservation.rs
  repositories/rate_repo.rs, reservation_repo.rs
  services/rate_service.rs, reservation_service.rs
  commands/rates.rs, reservations.rs
src/
  pages/rates/RatesPage.tsx (+ forms)
  pages/reservations/ReservationsPage.tsx, ReservationFormPage.tsx, ReservationPage.tsx, ChangeDialog.tsx, CancelDialog.tsx, DiscountDialog.tsx
  pages/calendar/RoomMapPage.tsx
  lib/api.ts (+ comandos)
docs/usuario/{tarifas,reservas,mapa-de-quartos}.md   docs/desenvolvimento/{tarifas-e-preco,reservas-e-disponibilidade}.md
```

---

### Task 1: Acesso ao banco sem pânico (mutex envenenado)

**Files:**
- Modify: `src-tauri/src/services/mod.rs` (`Ctx`), todos os serviços e `app.rs`/`auth/session.rs` que usam `.lock().unwrap()`
- Test: `services/mod.rs`

**Interfaces:**
- Produces: `Ctx::conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError>`; mutex envenenado vira `AppError::Database` (mensagem genérica) em vez de pânico. `SessionManager` usa `lock().unwrap_or_else(|e| e.into_inner())` (estado de sessão é reconstruível; nunca derruba o app).

- [ ] **Step 1: Testes que falham**: `poisoned_db_mutex_returns_error_not_panic` (envenenar o mutex com uma thread que entra em pânico segurando o guard; `guests.list` devolve `Err(AppError::Database(_))`); `poisoned_session_manager_still_answers` (`touch` continua funcionando após pânico em outra thread).
- [ ] **Step 2: Run** `cargo test poisoned` → FAIL.
- [ ] **Step 3: Implementar** `Ctx::conn` e substituir os ~60 `self.ctx.db.lock().unwrap()` por `self.ctx.conn()?`.
- [ ] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` e `clippy -D warnings` → PASS.
- [ ] **Step 5: Commit** `fix: no panic on poisoned database mutex`

---

### Task 2: Dinheiro e arredondamento (domínio puro)

**Files:**
- Create: `src-tauri/src/domain/money.rs`; Modify: `domain/mod.rs`
- Test: `domain/money.rs`

**Interfaces:**
- Produces: `percent_of(cents: i64, basis_points: u32) -> Result<i64, AppError>` (centavo mais próximo, meio centavo sobe; `basis_points` 10000 = 100%); `add(a, b)`, `sub(a, b)`, `sum(iter)` com `checked_*` ⇒ `AppError::Validation("Valor muito grande.")` no estouro; `percent_to_bp(percent: f64)` não é oferecido (percentuais entram sempre como `u32` de pontos-base, ex.: 3000 = 30%).

- [ ] **Step 1: Testes que falham**: `percent_of(100335, 3000) == 30101` (30% de R$ 1.003,35 = R$ 301,01); `percent_of(100334, 3000) == 30100`; meio centavo sobe (`percent_of(5, 5000) == 3`); `percent_of(0, 3000) == 0`; `percent_of(i64::MAX, 10000)` ok e `percent_of(i64::MAX, 20000)` ⇒ erro; `sum` com estouro ⇒ erro.
- [ ] **Step 2: Run** `cargo test money` → FAIL.
- [ ] **Step 3: Implementar** com `i128` internamente e conversão checada.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: money helpers with half-up percent rounding`

---

### Task 3: Tarifas — migração, repositório e serviço

**Files:**
- Create: `migrations/sql/0006_rates_and_policies.sql`, `models/rate.rs`, `repositories/rate_repo.rs`, `services/rate_service.rs`, `commands/rates.rs`
- Modify: `migrations/mod.rs`, `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs` (+`test_support` com `rates`), `app.rs`, `lib.rs`, `commands/mod.rs`
- Test: `services/rate_service.rs`

**Interfaces:**
- Produces (tabelas): `rate_rules(id, category_id → room_categories, rule_type CHECK IN ('weekday','season','holiday','date_override'), weekday 0–6 NULL, start_date NULL, end_date NULL, date NULL, amount_cents > 0, label, active, version, created_at, updated_at)`; `holidays(id, date UNIQUE, name, active)`.
- Produces (modelos): `RateRule`, `NewRateRule` (campos por tipo), `Holiday`.
- Produces (serviço, escritas exigem `ManageCatalog`; leitura qualquer sessão): `RateService::{list_rules(token, category_id: Option<i64>), create_rule(token, NewRateRule) -> i64, update_rule(token, id, NewRateRule, expected_version), set_rule_active(token, id, active), delete_rule(token, id), list_holidays(token, year: Option<i32>), create_holiday(token, date, name) -> i64, delete_holiday(token, id)}`.
- Regras: `weekday` único por (categoria, dia) ativo; `season` com `start_date <= end_date` e **sem sobreposição** com outra temporada ativa da mesma categoria; `holiday` e `date_override` exigem data; valor em centavos > 0 e ≤ R$ 99.999,99; tudo auditado com antes/depois.

- [ ] **Step 1: Testes que falham**: `weekday_rule_is_unique_per_category_and_day`; `season_requires_ordered_dates_and_rejects_overlap` (15/12–28/02 × 01/02–31/03 ⇒ `Validation`; períodos adjacentes ok); `amount_must_be_positive_and_bounded`; `recepcao_reads_but_cannot_write`; `stale_version_conflicts`; `holiday_date_unique`; `writes_are_audited`; `deleting_unused_rule_is_allowed`.
- [ ] **Step 2: Run** `cargo test rate_service` → FAIL.
- [ ] **Step 3: Implementar** migração (registrar versão 6), repositório e serviço (padrão de `catalog_service`).
- [ ] **Step 4: Run** `cargo test`, `clippy` → PASS.
- [ ] **Step 5: Commit** `feat: rate rules and holidays (backend)`

---

### Task 4: Motor de preço (função pura + cotação)

**Files:**
- Create: `domain/pricing.rs`
- Modify: `services/rate_service.rs` (+ `quote`), `commands/rates.rs`
- Test: `domain/pricing.rs`, `services/rate_service.rs`

**Interfaces:**
- Produces:
  - `domain::pricing::{RuleInput, NightPrice { date: String, amount_cents: i64, rule: AppliedRule }, Quote { nights: Vec<NightPrice>, total_cents: i64, missing_dates: Vec<String> }, price_stay(rules: &[RuleInput], check_in: NaiveDate, check_out: NaiveDate) -> Result<Quote, AppError>}` com `AppliedRule = DateOverride | Holiday | Season | Weekday`.
  - `RateService::quote(token, category_id: i64, check_in: &str, check_out: &str) -> Result<Quote, AppError>` (qualquer sessão); `Quote.missing_dates` não vazio ⇒ a criação de reserva é recusada (Task 8).
- Regras: noites `check_in ..= check_out - 1 dia` (saída não gera diária); `check_out <= check_in` ⇒ `Validation`; máximo de 366 noites.

- [ ] **Step 1: Testes que falham** (função pura, sem banco): `priority_date_override_beats_holiday_beats_season_beats_weekday` (mesma noite com os 4 tipos); `checkout_night_is_not_charged` (3 noites para check-in 10 e saída 13); `season_crossing_year_applies_on_both_sides` (28/12/2026–03/01/2027 em temporada 15/12–28/02); `weekend_vs_weekday_rates` (sexta/sábado diferentes); `missing_tariff_lists_dates_never_zero` (noite sem regra ⇒ `missing_dates` com a data e nenhuma diária de valor 0); `leap_day_2028_is_a_valid_night`; `invalid_range_rejected` (saída = entrada, saída < entrada); `inactive_rules_are_ignored`; `total_is_sum_of_nights`. Serviço: `quote_uses_category_rules_only` e `quote_available_to_recepcao`.
- [ ] **Step 2: Run** `cargo test pricing quote` → FAIL.
- [ ] **Step 3: Implementar** (`chrono::NaiveDate`; dia da semana 0 = domingo).
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: pricing engine with rate priority`

---

### Task 5: Tarifas — interface

**Files:**
- Create: `src/pages/rates/RatesPage.tsx`, `RuleForm.tsx`, `HolidaysPanel.tsx`; Modify: `src/lib/api.ts`, `routes.tsx`, `Sidebar.tsx`
- Test: `RatesPage.test.tsx`

**Interfaces:**
- Consumes: comandos da Task 3 e `quote` da Task 4.
- Produces: rota `/tarifas` (visível a todos, edição só para administrador): escolhe a categoria e mostra quatro blocos — **Dia da semana** (tabela de 7 linhas), **Temporadas**, **Feriados e exceções de data**; botão "Simular preço" (categoria + entrada + saída) mostrando noite a noite o valor e a regra aplicada.

- [ ] **Step 1: Testes que falham**: lista as regras da categoria escolhida com a regra em texto; cria temporada válida (envia centavos a partir de "R$ 450,00"); mostra o erro de sobreposição do backend sem perder o formulário; recepção não vê botões de escrita; "Simular preço" lista as noites e destaca datas sem tarifa; fechar formulário com dados pede confirmação (`useGuardedClose`).
- [ ] **Step 2: Run** `npx vitest run src/pages/rates` → FAIL.
- [ ] **Step 3: Implementar**; entrada de dinheiro via `parseMoney`, exibição via `formatMoney`.
- [ ] **Step 4: Run** `npx vitest run`, `npm run build` → PASS.
- [ ] **Step 5: Commit** `feat: rates screen with price simulation`

---

### Task 6: Disponibilidade (função pura)

**Files:**
- Create: `domain/availability.rs`
- Test: `domain/availability.rs`

**Interfaces:**
- Produces: `Booking { id: i64, check_in: NaiveDate, check_out: NaiveDate, expected_in: NaiveTime, expected_out: NaiveTime }`; `find_conflicts(existing: &[Booking], wanted_in: NaiveDate, wanted_in_time: NaiveTime, wanted_out: NaiveDate, wanted_out_time: NaiveTime, cleaning_minutes: u32, ignore_id: Option<i64>) -> Vec<i64>` (ids em conflito).
- Regras: intervalo semiaberto de noites; sobreposição de noites ⇒ conflito; saída e entrada no mesmo dia só sem conflito se `saída_prevista + limpeza ≤ entrada_seguinte` (nos dois sentidos); `ignore_id` exclui a própria reserva em alterações.

- [ ] **Step 1: Testes que falham**: `overlapping_nights_conflict`; `adjacent_days_do_not_conflict` (saída dia 10, entrada dia 11); `same_day_turnover_allowed_when_cleaning_fits` (sai 12:00, limpeza 60, entra 14:00); `same_day_turnover_refused_when_cleaning_does_not_fit` (sai 12:00, limpeza 120, entra 13:30); `turnover_checked_in_both_directions`; `one_night_booking_fully_inside_another_conflicts`; `ignore_id_excludes_self`; `month_and_year_boundaries`.
- [ ] **Step 2: Run** `cargo test availability` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: availability conflict rules`

---

### Task 7: Configurações e política de cancelamento

**Files:**
- Modify: `migrations/sql/0006_rates_and_policies.sql` (settings e `cancellation_tiers`), `models/settings.rs`, `repositories/settings_repo.rs`, `services/settings_service.rs`, `src/pages/settings/SystemForm.tsx`, `src/lib/api.ts`
- Create: `domain/cancellation.rs`, `services/cancellation_service.rs` (ou métodos em `RateService`: `list_tiers`, `replace_tiers`), `src/pages/settings/CancellationPolicyForm.tsx`
- Test: `domain/cancellation.rs`, `settings_service.rs`, `SettingsPage.test.tsx`

**Interfaces:**
- Produces (`AppSettings`, valores provisórios): `deposit_percent_bp: u32` (3000), `deposit_due_days_after_booking: u32` (3), `deposit_due_days_before_arrival: u32` (1), `min_stay_nights: u32` (0 = sem mínimo; **sem uso nesta versão**, reservado). `no_show_tolerance_minutes` e `cleaning_interval_minutes` já existem. Faixas: sinal 0–10000 pontos-base; prazos 0–60 dias.
- Produces (tabela): `cancellation_tiers(id, min_days_before INTEGER, refund_bp INTEGER 0..=10000)` ordenada; padrão: `(7, 10000)`, `(3, 5000)`, `(0, 0)`; validação: `min_days_before` únicos, deve existir faixa `0`, `refund_bp` não crescente à medida que `min_days_before` diminui.
- Produces (domínio): `cancellation::compute(tiers: &[Tier], days_before: i64, deposit_paid_cents: i64, no_show: bool) -> Result<Outcome { refund_cents, retained_cents, tier_min_days: Option<u32> }, AppError>`; `no_show` ⇒ devolve 0 (reten 100%); reembolso = `percent_of(deposit_paid, refund_bp)`; `retained = paid - refund`.
- Produces (cálculo do prazo): `deposit_due_date(booked_on, check_in, after_days, before_days) -> NaiveDate` = menor entre `booked_on + after_days` e `check_in - before_days`, nunca antes de `booked_on`.

- [ ] **Step 1: Testes que falham**: `compute` com 8, 7, 6, 3, 2, 0 dias e no-show (valores exatos sobre sinal de R$ 450,05 → arredondamento); faixas inválidas recusadas (sem faixa 0, duplicadas, reembolso crescente); `deposit_due_date` (reserva feita 2 dias antes da chegada ⇒ vence 1 dia antes; feita 30 dias antes ⇒ 3 dias depois; chegada amanhã ⇒ hoje); `settings` novas validadas e auditadas; tela salva a política e rejeita faixas inconsistentes mostrando o erro.
- [ ] **Step 2: Run** `cargo test cancellation settings` e `npx vitest run src/pages/settings` → FAIL.
- [ ] **Step 3: Implementar**; atualizar o teste `defaults_are_the_provisional_values`; marcar "valores provisórios" na tela.
- [ ] **Step 4: Run** suítes completas → PASS.
- [ ] **Step 5: Commit** `feat: deposit and cancellation policy settings`

---

### Task 8: Reservas — criar e consultar (backend)

**Files:**
- Create: `migrations/sql/0007_reservations.sql`, `models/reservation.rs`, `repositories/reservation_repo.rs`, `services/reservation_service.rs`, `commands/reservations.rs`
- Modify: registros em `migrations/mod.rs`, `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `test_support.rs` (`reservations`), `app.rs`, `lib.rs`, `commands/mod.rs`
- Test: `services/reservation_service.rs`

**Interfaces:**
- Produces (tabelas): `reservations(id, code UNIQUE, room_id → rooms, guest_id → guests, check_in_date, check_out_date, expected_check_in_time, expected_check_out_time, occupants INTEGER ≥ 1, status CHECK IN ('pending','confirmed','checked_in','completed','cancelled','no_show'), list_total_cents, discount_cents, discount_reason, discount_applied_by, agreed_total_cents, deposit_expected_cents, deposit_due_date, notes, cancelled_at, cancellation_fee_cents, version, created_by, created_at, updated_at)`; `reservation_nights(reservation_id, date, amount_cents, rule, PRIMARY KEY(reservation_id, date))`; `reservation_changes(id, reservation_id, changed_at, changed_by, kind, before_json, after_json, price_before_cents, price_after_cents, justification)`; índice `(room_id, check_in_date, check_out_date)`.
- Produces: `NewReservation { room_id, guest_id, check_in_date, check_out_date, expected_check_in_time: Option<String>, expected_check_out_time: Option<String>, occupants: u32, notes, exceed_capacity: bool }`.
- `ReservationService::create(token, NewReservation) -> Created { id, code, agreed_total_cents, deposit_expected_cents, deposit_due_date }`: tudo numa transação — valida quarto **ativo**, hóspede **ativo**, datas (entrada **não no passado**: D-03; saída > entrada), ocupantes ≤ capacidade (`exceed_capacity = true` exige `ExceedCapacity`), cotação sem `missing_dates`, **disponibilidade** (somente reservas `pending/confirmed/checked_in` bloqueiam), calcula sinal (`percent_of(total, deposit_percent_bp)`) e prazo, sequência `reservation`, grava reserva + noites congeladas, audita.
- `ReservationService::{get(token, id) -> ReservationDetail (com noites, histórico e saldo do sinal), list(token, ReservationQuery { text, status: Option<Vec<String>>, room_id, from, to, page, page_size, sort, direction }) -> Page<ReservationRow>, check_availability(token, room_id, check_in, check_out) -> AvailabilityResult { available: bool, conflicts: Vec<ConflictInfo> }}`.
- Permissões: criar/consultar qualquer sessão.

- [ ] **Step 1: Testes que falham**: `creates_pending_reservation_with_frozen_nights_and_deposit`; `second_reservation_for_same_room_and_period_is_refused`; `concurrent_creation_only_one_wins` (duas threads com `Ctx` compartilhado, uma única gravada); `cancelled_and_no_show_do_not_block`; `same_day_turnover_respects_cleaning_interval`; `missing_tariff_blocks_creation_and_lists_dates`; `past_check_in_is_refused`; `inactive_room_or_guest_refused`; `capacity_exceeded_needs_permission` (Forbidden sem a permissão; com `extra` passa); `later_rate_change_does_not_alter_existing_reservation`; `code_sequence_has_no_gaps_on_rollback` (criação que falha não consome número); `audit_has_after_json`; `list_filters_by_status_room_period_and_text` (código, nome do hóspede); `check_availability_reports_conflicts`.
- [ ] **Step 2: Run** `cargo test reservation_service` → FAIL.
- [ ] **Step 3: Implementar** (migração versão 7); montar `Booking` a partir do banco e chamar `find_conflicts`.
- [ ] **Step 4: Run** `cargo test`, `clippy` → PASS.
- [ ] **Step 5: Commit** `feat: create and list reservations (backend)`

---

### Task 9: Alterar datas ou quarto (prévia e confirmação)

**Files:**
- Modify: `services/reservation_service.rs`, `repositories/reservation_repo.rs`, `commands/reservations.rs`
- Test: `services/reservation_service.rs`

**Interfaces:**
- Produces: `preview_change(token, id, ChangeRequest { room_id: Option<i64>, check_in_date: Option<String>, check_out_date: Option<String> }) -> ChangePreview { feasible: bool, conflicts: Vec<ConflictInfo>, missing_dates: Vec<String>, price_before_cents, price_after_cents, difference_cents, nights: Vec<NightPrice>, fingerprint: String }` — **não grava nada**; `fingerprint` é um hash do preço calculado + versão da reserva.
- `apply_change(token, id, ChangeRequest, expected_version, justification: String, fingerprint: String) -> ()`: exige `ChangeReservation` e justificativa não vazia; recalcula tudo na transação; se o `fingerprint` não conferir (tabela de preços ou reserva mudou) ⇒ `Conflict` ("revise a nova prévia"); noites inalteradas **mantêm o preço combinado**, noites novas usam a tabela vigente; se a categoria do quarto muda, **todas** as noites são recalculadas (D-13); ajusta o sinal esperado; só status `pending`/`confirmed` podem mudar; grava `reservation_changes` (antes/depois, preços) e audita.

- [ ] **Step 1: Testes que falham**: `preview_never_writes` (tabelas intactas após a prévia); `preview_shows_before_after_and_difference`; `unchanged_nights_keep_agreed_price_when_table_changed`; `new_nights_use_current_table`; `category_change_reprices_all_nights`; `apply_requires_permission_and_justification`; `apply_with_stale_fingerprint_is_refused`; `apply_with_stale_version_conflicts`; `conflict_with_other_reservation_blocks_change`; `change_not_allowed_for_checked_in_cancelled_or_completed`; `history_records_user_time_values_and_justification`; `deposit_expected_follows_new_total`.
- [ ] **Step 2: Run** `cargo test change` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: reservation change with price preview and confirmation`

---

### Task 10: Cancelamento, no-show e revisão

**Files:**
- Modify: `services/reservation_service.rs`, `commands/reservations.rs`
- Test: `services/reservation_service.rs`

**Interfaces:**
- Produces: `preview_cancellation(token, id) -> CancellationPreview { days_before: i64, deposit_paid_cents, refund_cents, retained_cents, tier_min_days, can_cancel }` (usa a política da Task 7, **sem gravar**); `cancel(token, id, expected_version, override_: Option<CancellationOverride { retained_cents: i64, justification: String }>) -> CancellationResult { refund_cents, retained_cents }`: valor padrão é o calculado; `override_` exige `CancellationException`, justificativa e `0 ≤ retained ≤ sinal pago`, e a auditoria guarda também o valor calculado; só `pending`/`confirmed` cancelam; o status vira `cancelled` e o quarto fica livre **após a gravação**; o registro é preservado.
- `mark_no_show(token, id, expected_version)`: só se `check_in_date`/horário previsto + `no_show_tolerance_minutes` já passou e status `pending`/`confirmed`; retém o sinal (política: no-show).
- `list_needing_review(token) -> Vec<ReservationRow>` com `reason: "no_show_candidate" | "deposit_overdue"` (sinal vencido: reserva `pending` com `deposit_due_date < hoje` e sinal pago < esperado). **Nenhuma ação automática.**
- Observação: devolução/recebimento financeiros reais chegam no Marco 5; aqui o resultado é **registrado** na reserva (`cancellation_fee_cents`) e na auditoria.

- [ ] **Step 1: Testes que falham**: `preview_uses_policy_tiers_and_never_writes`; `cancel_defaults_to_calculated_values`; `override_requires_permission_justification_and_bounds`; `override_audit_keeps_calculated_value`; `cancel_preserves_record_and_frees_the_room`; `cannot_cancel_checked_in_completed_or_already_cancelled`; `no_show_refused_before_tolerance` e `no_show_allowed_after_tolerance`; `review_list_flags_candidates_and_overdue_deposits_without_changing_them`; `cancel_with_stale_version_conflicts`.
- [ ] **Step 2: Run** `cargo test cancel` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: reservation cancellation policy, no-show review`

---

### Task 11: Desconto, confirmação do sinal e observações

**Files:**
- Modify: `services/reservation_service.rs`, `commands/reservations.rs`
- Test: `services/reservation_service.rs`

**Interfaces:**
- Produces: `apply_discount(token, id, DiscountInput { kind: "amount" | "percent", amount_cents: Option<i64>, percent_bp: Option<u32>, justification }, expected_version)`: exige `ApplyDiscount` e justificativa; percentual = `percent_of(list_total, bp)`; desconto ≤ `list_total`; grava `discount_cents`, `agreed_total_cents = list_total - discount`, recalcula o sinal esperado; histórico/auditoria com **valor original, final**, usuário e data; só `pending`/`confirmed`.
- `confirm_reservation(token, id, expected_version)`: `pending → confirmed` manual (D-06); `set_notes(token, id, notes, expected_version)`.

- [ ] **Step 1: Testes que falham**: `discount_requires_permission_and_justification`; `percent_discount_rounds_half_up_once_on_total`; `discount_cannot_exceed_total`; `discount_records_original_and_final`; `manual_confirmation_moves_pending_to_confirmed`; `confirmation_refused_for_other_statuses`; `notes_update_with_version_check`.
- [ ] **Step 2: Run** `cargo test discount confirm` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** `cargo test`, `clippy` → PASS.
- [ ] **Step 5: Commit** `feat: reservation discount, manual confirmation and notes`

---

### Task 12: Reservas — lista e nova reserva (interface)

**Files:**
- Create: `src/pages/reservations/ReservationsPage.tsx`, `ReservationFormPage.tsx`, `AvailabilityPanel.tsx`; Modify: `src/lib/api.ts`, `routes.tsx`, `Sidebar.tsx`, `src/lib/format.ts` (status em português)
- Test: `ReservationsPage.test.tsx`, `ReservationForm.test.tsx`

**Interfaces:**
- Produces: rotas `/reservas`, `/reservas/nova`, `/reservas/:id`; lista com filtros (texto, situação, quarto, período), situação em **texto + ícone** (Pendente, Confirmada, Hospedado, Concluída, Cancelada, No-show) e destaque "Sinal vencido"/"Para revisar"; formulário: hóspede (busca com a `GlobalSearch`/lista de hóspedes), quarto, entrada, saída, ocupantes; ao preencher, mostra **disponibilidade e cotação ao vivo** (noite a noite, total, sinal esperado e prazo); datas sem tarifa e conflitos listados; botão **"Criar reserva"**; excesso de capacidade só aparece com permissão; aviso ao sair com alterações.

- [ ] **Step 1: Testes que falham**: lista com status em texto e filtros enviados ao backend; formulário valida campos com `*`; mostra cotação e sinal esperado; mostra conflito e desabilita o botão; lista as datas sem tarifa; erro do backend mantém o formulário preenchido; reserva criada leva à tela dela com a mensagem "Reserva RES-000001 criada."; sair com formulário alterado pede decisão; recepção sem permissão não vê a opção de exceder capacidade.
- [ ] **Step 2: Run** `npx vitest run src/pages/reservations` → FAIL.
- [ ] **Step 3: Implementar**; consultas com `useQuery` (chave inclui parâmetros); `formatMoney`/`formatDate`.
- [ ] **Step 4: Run** `npx vitest run`, `npm run build` → PASS.
- [ ] **Step 5: Commit** `feat: reservations list and creation screens`

---

### Task 13: Tela da reserva — alterar, cancelar, desconto

**Files:**
- Create: `src/pages/reservations/ReservationPage.tsx`, `ChangeDialog.tsx`, `CancelDialog.tsx`, `DiscountDialog.tsx`, `HistoryList.tsx`; Modify: `routes.tsx`
- Test: `ReservationPage.test.tsx`, `ChangeDialog.test.tsx`, `CancelDialog.test.tsx`

**Interfaces:**
- Produces: tela com dados, noites e valores, sinal (esperado/prazo/situação) e histórico; ações com rótulos específicos: **"Alterar datas ou quarto"** (prévia → mostra **preço anterior, novo e diferença**, conflitos e datas sem tarifa; justificativa obrigatória; "Confirmar alteração"), **"Cancelar reserva"** (mostra o cálculo: dias de antecedência, sinal pago, devolução e retenção; exceção só para quem tem permissão, com justificativa), **"Aplicar desconto"**, **"Confirmar reserva"**, **"Marcar no-show"** (quando liberado); `Conflict` explica e oferece recarregar; fingerprint desatualizado pede nova prévia.

- [ ] **Step 1: Testes que falham**: prévia mostra anterior/novo/diferença e não chama `apply` sem confirmar; confirmar envia justificativa, versão e fingerprint; `Conflict` por fingerprint mostra "Os preços mudaram; revise a nova prévia" e refaz a prévia; cancelamento mostra devolução/retenção calculadas e só habilita a exceção com permissão; desconto valida justificativa e teto; botões de ação ausentes para status que não permitem; histórico lista valores antes/depois.
- [ ] **Step 2: Run** `npx vitest run src/pages/reservations` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** `npx vitest run`, `npm run build` → PASS.
- [ ] **Step 5: Commit** `feat: reservation screen with change, cancellation and discount`

---

### Task 14: Mapa de quartos (grade quarto × dia)

**Files:**
- Create: `src-tauri/src/services/room_map_service.rs` (+ comando), `src/pages/calendar/RoomMapPage.tsx`
- Test: `room_map_service.rs`, `RoomMapPage.test.tsx`

**Interfaces:**
- Produces: `room_map(token, from: &str, days: u32 /* 1..=31 */) -> RoomMap { rooms: Vec<RoomRow>, cells: Vec<{ room_id, date, reservation_id, code, guest_name, status, is_check_in, is_check_out }> }`, somente leitura, só reservas que bloqueiam (`pending/confirmed/checked_in/completed`); quartos inativos aparecem esmaecidos. Tela com período padrão **hoje + 6 dias**, navegação anterior/próximo/hoje, células com **texto do hóspede e da situação** (não só cor), clique abre `/reservas/:id`; células vazias abrem "Nova reserva" já com quarto e data.

- [ ] **Step 1: Testes que falham**: serviço devolve só reservas do período e ignora canceladas/no-show; marca entrada e saída no mesmo dia; período limitado a 31 dias; interface mostra grade com texto e situação, navega entre períodos, clique na célula livre pré-preenche a nova reserva.
- [ ] **Step 2: Run** `cargo test room_map` e `npx vitest run src/pages/calendar` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Run** suítes completas → PASS.
- [ ] **Step 5: Commit** `feat: room map by day`

---

### Task 15: Fechamento do marco

**Files:**
- Create: `docs/usuario/{tarifas,reservas,mapa-de-quartos}.md`, `docs/desenvolvimento/{tarifas-e-preco,reservas-e-disponibilidade}.md`
- Modify: `CHANGELOG.md`, `README.md`, índices de docs, este plano, `local/plano-completo.md`, `local/mocks/gerar_mock.py` (campos novos de reserva), `docs/desenvolvimento/permissoes.md`

**Interfaces:**
- Produces: documentação de usuário em linguagem simples (como criar reservas, ler a prévia de alteração, cancelar com devolução, o que é "sinal vencido"/"para revisar") e de desenvolvimento (prioridade de tarifas, arredondamento, política, fingerprint, transação de disponibilidade, estados e transições).

- [ ] **Step 1:** Escrever a documentação e o `CHANGELOG.md` (`Added`/`Changed`).
- [ ] **Step 2: Run** `cargo test`, `cargo clippy -D warnings`, `npm test`, `npm run build`; `graphify update .`.
- [ ] **Step 3:** Atualizar a seção Status deste plano (testes, desvios, pendências) e o roteiro.
- [ ] **Step 4:** Revisão final da branch por revisor independente; corrigir Críticos/Importantes (TDD); registrar menores.
- [ ] **Step 5: Commit** `docs: close marco 2`

---

## Status

**Implementado** na branch `marco-2-reservas` (T1 a T15). Migrações `0006` (tarifas), `0007` (política) e `0008` (reservas) — renumeradas em relação ao rascunho (`0006`/`0007`) porque a política ganhou migração própria.

- **Testes ao fechar:** `cargo test` 220 unitários + 4 de integração; `npm test` 195; `cargo clippy -D warnings` e `npm run build` limpos.
- **Desvios (rulings):** `record_deposit` interino e `my_permissions` antecipados para destravar o cálculo do cancelamento e as dicas de permissão na tela; `AppError::StalePreview` para prévia desatualizada; `testTimeout` de 20 s no Vitest (suíte lenta sob carga); roteador de dados (`createHashRouter`) por causa do `useBlocker`.
- **Pendências para outros marcos:** recebimentos e devoluções reais (5); hospedagem e `checked_in`/`completed` (3); bloqueios e manutenção na disponibilidade (4); alertas persistentes e relatórios (6). Valores comerciais continuam **provisórios** até a aprovação de `local/revisao-decisoes-marco-2.md`.
- **Revisão final (revisor independente):** nenhum Crítico. Importantes corrigidos com teste RED→GREEN: botão de no-show só aparecia depois do dia da chegada (agora a partir do dia; a tolerância é aplicada no backend); registro de sinal sem confirmação (agora pede confirmação com o valor). Suíte: vitest 196, cargo 220+4.
- **Menores adiados:** (a) prévia de cancelamento pode ficar desatualizada se a faixa mudar com a virada do dia; (b) data do histórico usa o dia UTC; (c) vencimento do sinal não é recalculado ao mudar as datas; (d) total combinado pode ficar abaixo do sinal já pago após desconto; (e) desconto percentual vira valor fixo após alterar datas; (f) duplo clique em "Confirmar reserva" mostra sucesso e erro juntos; (g) política de cancelamento rejeita vírgula decimal; (h) mutex envenenado exige reiniciar; (i) desativar quarto/reduzir capacidade não verifica reservas futuras (código do Marco 1); (j) **sem caminho de correção do sinal registrado** até o Marco 5 (estorno/ajuste); (k) D-04 (sinal por reserva) não implementado.
- **Verificação manual:** `npm run tauri dev` ainda não foi exercitado pela interface real; fica para o responsável.

## Self-Review

- **Cobertura da spec (Marco 2):** §4.1 criação (T8, T12), §4.2 alteração com prévia (T9, T13), §4.3 cancelamento/no-show/sinal vencido (T10, T13), §5 tarifas e preço (T3, T4, T5), desconto (T11), §19.2 dinheiro (T2). Anexo A D-02 a D-14 → T7 (configurações), T4/T6 (D-10, D-14), T8 (D-03, D-09, D-06), T9 (D-13), T10 (D-05, D-07, D-08), T11 (D-11), T2 (D-12). Ficam para outros marcos: walk-in e hospedagem (3), bloqueios/manutenção na disponibilidade (4), recebimento/devolução reais (5), relatórios e alertas persistentes (6).
- **Consistência de tipos:** `Quote`/`NightPrice` (T4) usados por T8, T9 e T12; `Booking`/`find_conflicts` (T6) por T8 e T9; `percent_of` (T2) por T7, T8, T10 e T11; `deposit_due_date` e `cancellation::compute` (T7) por T8 e T10; `ChangePreview.fingerprint` (T9) por T13.
- **Valores provisórios:** nenhum número comercial é constante no código; os padrões entram só na migração/seed de `settings` e `cancellation_tiers`, e a interface os rotula como provisórios.
- **Migrações:** `0006` (tarifas, feriados, política) e `0007` (reservas); as seguintes do roteiro avançam.
- **Proporção:** corpos de função a cargo do implementador; testes trazem valores exatos (R$ 1.003,35 → R$ 301,01, faixas de cancelamento, 29/02/2028).
