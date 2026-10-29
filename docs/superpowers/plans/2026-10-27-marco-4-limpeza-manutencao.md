# Marco 4 — Limpeza, manutenção e bloqueios Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar ao quarto um ciclo de estado (ocupado → aguardando limpeza → em limpeza → limpo aguardando intervalo → disponível), com fila de limpeza, bloqueios e ordens de manutenção integrados à disponibilidade, e fechar as pendências M3-10/D-19 (check-in só em quarto limpo).

**Architecture:** O estado do quarto é **calculado** por uma função pura (`domain/room_state.rs`) a partir de hospedagens, tarefas de limpeza, bloqueios e relógio; **não há coluna "estado do quarto"**. Fatos persistidos: `cleaning_tasks` (criada na mesma transação do check-out), `room_blocks` e `maintenance_orders`/`maintenance_conflicts`. Bloqueios entram na disponibilidade como mais uma fonte de conflito, lida pelos mesmos pontos que hoje chamam `find_conflicts` (reserva, alteração, consulta, check-in, walk-in). Tempo nunca muda estado: só ações de usuário.

**Tech Stack:** o dos marcos anteriores (Rust/rusqlite/SQLCipher, React/TS/Vitest). Sem dependências novas.

**Spec:** `local/spec.md` §7, §8, §13, §18 e Anexo A. Roteiro: `local/plano-completo.md` §8.

**Branch:** `marco-4-limpeza`, a partir de `marco-3-hospedagem` (o Marco 3 ainda não foi integrado em `dev`; ver "Pré-requisito").

## Pré-requisito

Marco 3 está só na branch `marco-3-hospedagem` (não está em `origin`). Decidir antes de começar: integrar o Marco 3 em `dev` e ramificar dali, **ou** ramificar de `marco-3-hospedagem`. O plano vale para ambos; a migração é a `0010`.

## Decisões deste plano (chamadas minhas; revisar antes de executar)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| M4-1 | Migração é a **`0010`** (o roteiro dizia `0005`). | Numeração avançou nos marcos anteriores. |
| M4-2 | **"Liberação" do quarto após limpeza é calculada:** limpeza concluída + `concluída_em + intervalo ≤ agora` + sem bloqueio ⇒ *Disponível*. Não existe botão "liberar" para limpeza; o que o tempo **não** faz é concluir a limpeza. `ReleaseRoom` (spec §8) fica para sair de **bloqueio/manutenção**. | Leitura mais simples de spec §7 ("tempo não libera sozinho" = sem limpeza confirmada, nada libera). Se a pousada quiser um passo explícito de liberação após a limpeza, acrescenta-se um estado e uma ação. |
| M4-3 | O intervalo de limpeza passa a ter **valor por quarto** (`rooms.cleaning_interval_minutes`, nulo = padrão das configurações, 60). Roteiro dizia "quarto/categoria"; spec diz "cada quarto". | Por categoria seria só outra coluna. |
| M4-4 | **Check-in** é recusado se o quarto tem tarefa de limpeza aberta (aguardando/em limpeza) ou se a última limpeza concluiu há menos que o intervalo. Quarto sem histórico de limpeza (nunca ocupado) está livre. Mensagem diz o estado e o horário em que fica livre. | Fecha M3-10/D-19. Custo: recepção precisa registrar a limpeza para hospedar; é a regra da spec. |
| M4-5 | **Quem faz o quê:** qualquer usuário autenticado inicia/conclui limpeza, ajusta prioridade (auditado) e abre/avança ordens de manutenção; **criar/encerrar bloqueio manual e liberar** exige `ReleaseRoom` (já existente, concedível). Nenhuma permissão nova. | Limpeza é rotina da recepção; indisponibilizar/liberar quarto é sensível (spec §13.1). |
| M4-6 | **Período do bloqueio** segue a lógica das noites: `início` inclusivo, `fim` exclusivo, `fim` nulo = até liberação manual. Bloqueio não usa intervalo de limpeza nas bordas. | Coerente com reservas (saída não gera diária). |
| M4-7 | **Bloqueio manual sobre reservas ativas é recusado** listando as reservas; o caminho com reservas afetadas é a ordem de manutenção. Bloqueio sobre quarto ocupado ou com hospedagem em curso no período também é recusado. | Evita indisponibilidade que contradiz reserva. |
| M4-8 | **Manutenção com conflito:** ao abrir a ordem com reservas afetadas, cria-se um bloqueio **preventivo** (novas reservas recusadas no período) e uma linha em `maintenance_conflicts` por reserva (`open`). O conflito é `resolved` quando a reserva deixa de estar ativa ou de tocar o período (verificado ao pedir `Aplicar indisponibilidade`), ou manualmente com nota. Com zero abertos, o bloqueio vira **efetivo** (`active`). | Spec §8. Nada é cancelado/alterado automaticamente. |
| M4-9 | **Concluir a ordem não libera:** o bloqueio vai a `awaiting_release` e o quarto continua *Em manutenção/bloqueado* até `ReleaseRoom` (spec §8). | — |
| M4-10 | **Prioridade de limpeza sugerida** (função pura): `urgente` se há chegada prevista para o quarto em ≤ 4 h (ou já passou o horário de entrada); `alta` se hoje; `normal` se amanhã ou depois; `baixa` sem reserva futura. Valores 4 h / hoje são **provisórios** (entram no Anexo A.4). Ajuste manual guarda `priority_override` + justificativa e é auditado. | Spec §7 não dá números. |
| M4-11 | **Saída atrasada:** a tarefa de limpeza nasce com a hora **efetiva** do check-out (`ready_since = checked_out_at`); a prioridade é recalculada a cada leitura. Alertas persistentes ficam no Marco 6. | Spec §7. |

## Global Constraints

- Estado do quarto **calculado**, nunca gravado; o tempo não conclui limpeza nem libera bloqueio (spec §7, §8).
- Limpeza concluída registra **usuário e data/hora** (spec §7). Tarefa de limpeza é criada **na mesma transação** do check-out (CLAUDE.md: operações relacionadas em transação; auditoria na mesma transação).
- Quarto só fica disponível com limpeza confirmada **e** intervalo cumprido **e** sem bloqueio ou manutenção (spec §7).
- Mapa distingue: Ocupado, Aguardando limpeza, Em limpeza, Limpo aguardando intervalo, Disponível, Em manutenção/bloqueado — **cor e rótulo** (e ícone), nunca só cor (spec §7, CLAUDE.md).
- Saída atrasada não cancela nem altera reservas (spec §7).
- Ordem de manutenção: estados `open → in_progress → awaiting_resolution → completed`; campos quarto, descrição, prioridade, datas previstas e efetivas, responsável, observações (spec §8).
- Indisponibilidade efetiva só após resolver todos os conflitos; preventivo bloqueia novas reservas no período (spec §8).
- Conclusão da ordem não libera o quarto; liberação exige `ReleaseRoom` (spec §8, §13.1).
- Registros históricos (tarefas, bloqueios, ordens) nunca são apagados; encerrar = mudar situação (CLAUDE.md).
- Comparação de `version` (`Conflict`) em toda mudança de estado; duplo clique não duplica tarefa, início nem conclusão.
- Datas `YYYY-MM-DD`, instantes UTC, relógios injetados (`ctx.utc.now_utc()`, `ctx.local_now()`); datas na interface `DD/MM/AAAA`, horas `HH:MM`.
- Rótulos de ação: `Iniciar limpeza`, `Concluir limpeza`, `Criar bloqueio`, `Encerrar bloqueio`, `Abrir ordem de manutenção`, `Iniciar manutenção`, `Concluir manutenção`, `Liberar quarto`.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/desenvolvimento/`, este plano e o grafo a cada ciclo.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Duplo clique** em `Concluir limpeza`, `Iniciar limpeza`, `Concluir manutenção`, `Liberar quarto`: a segunda chamada é `Conflict`; nada duplica nem sobrescreve o usuário/hora da primeira.
2. **Relógio avançando** (horas ou dias) sem ninguém concluir a limpeza: quarto continua *Aguardando limpeza*; nunca vira *Disponível*.
3. **Check-out repetido ou falho:** exatamente uma tarefa por hospedagem; falha no check-out não deixa tarefa órfã.
4. **Bloqueio** com início depois do fim, no passado distante, em quarto inativo, inexistente ou sem motivo: recusado sem gravação.
5. **Reserva/alteração/walk-in sobre período bloqueado** (manual, preventivo, efetivo, aguardando liberação): recusada com a lista do que bloqueia; reserva que **termina** no dia em que o bloqueio começa passa.
6. **Manutenção concluída com bloqueio ainda aguardando liberação:** quarto continua bloqueado; `Liberar quarto` sem `ReleaseRoom` é recusado.
7. **Ordem com conflito:** abrir, cancelar a reserva afetada por fora, aplicar — o conflito some e o bloqueio vira efetivo; reserva ainda ativa mantém o preventivo.
8. **Prioridade** com chegada cruzando a meia-noite ou já atrasada, e override sem justificativa: sugestão correta; override recusado.

---

## File Structure

```
src-tauri/src/
  migrations/sql/0010_cleaning_maintenance.sql   # rooms.cleaning_interval_minutes; cleaning_tasks, room_blocks, maintenance_orders, maintenance_conflicts
  domain/room_state.rs                           # RoomState + room_state() (puro)
  domain/cleaning.rs                             # Priority + suggest_priority() (puro)
  models/cleaning.rs, block.rs, maintenance.rs
  repositories/cleaning_repo.rs, block_repo.rs, maintenance_repo.rs   # + room_repo helper de intervalo
  services/cleaning_service.rs, block_service.rs, maintenance_service.rs, room_status_service.rs
  commands/cleaning.rs, blocks.rs, maintenance.rs, room_status.rs
src/
  pages/cleaning/CleaningQueuePage.tsx, PriorityDialog.tsx
  pages/blocks/BlocksPage.tsx, BlockDialog.tsx
  pages/maintenance/MaintenancePage.tsx, MaintenanceOrderPage.tsx, OrderDialog.tsx
  pages/calendar/RoomMapPage.tsx                 # estados por cor + rótulo
  lib/roomState.ts, api.ts, routes.tsx, components/layout/Sidebar.tsx
docs/usuario/{limpeza,bloqueios-e-manutencao}.md   docs/desenvolvimento/{limpeza-e-estado-do-quarto,bloqueios-e-manutencao}.md
```

---

### Task 1: Migração `0010`

**Files:**
- Create: `src-tauri/src/migrations/sql/0010_cleaning_maintenance.sql`
- Modify: `src-tauri/src/migrations/mod.rs` (versão 10)
- Test: `src-tauri/src/migrations/mod.rs` (testes existentes de esquema)

**Interfaces:**
- Produces (tabelas): `rooms.cleaning_interval_minutes INTEGER NULL CHECK (BETWEEN 0 AND 1440)`; `cleaning_tasks(id, room_id, stay_id UNIQUE NULL, status 'awaiting'|'in_progress'|'done', ready_since, priority_override NULL 'urgent'|'high'|'normal'|'low', priority_justification, started_at, started_by, completed_at, completed_by, version, created_at, updated_at)`; `room_blocks(id, room_id, kind 'manual'|'preventive'|'maintenance', status 'preventive'|'active'|'awaiting_release'|'released', start_date, end_date NULL, reason, maintenance_order_id NULL, created_by, released_at, released_by, release_justification, version, created_at, updated_at)`; `maintenance_orders(id, code, room_id, description, priority, status 'open'|'in_progress'|'awaiting_resolution'|'completed', planned_start, planned_end, actual_start, actual_end, responsible, notes, version, created_by, created_at, updated_at)`; `maintenance_conflicts(id, order_id, reservation_id, status 'open'|'resolved', resolution_note, resolved_at, resolved_by)`. Sequência `maintenance` (`MAN-000001`) em `sequences`.

- [x] **Step 1: Write the failing test** em `migrations/mod.rs`: `migration_0010_creates_cleaning_and_maintenance_tables` (as quatro tabelas existem; `rooms` tem `cleaning_interval_minutes`; `cleaning_tasks.stay_id` rejeita duplicata; `room_blocks` rejeita `status='x'`; `current_version == 10`).
- [x] **Step 2: Run** `cargo test migration_0010` — Expected: FAIL (versão 9).
- [x] **Step 3: Write** `0010_cleaning_maintenance.sql` com as tabelas acima, índices em `(room_id, status)` e `(order_id)`, e registrar `Migration { version: 10, ... }`.
- [x] **Step 4: Run** `cargo test migration` — Expected: PASS (inclui testes de migrações anteriores).
- [x] **Step 5: Commit** `feat: migration 0010 cleaning and maintenance`

---

### Task 2: Domínio puro — estado do quarto e prioridade

**Files:**
- Create: `src-tauri/src/domain/room_state.rs`, `src-tauri/src/domain/cleaning.rs`
- Modify: `src-tauri/src/domain/mod.rs`

**Interfaces:**
- Produces:
  - `enum RoomState { Occupied, AwaitingCleaning, Cleaning, CleanWaitingInterval { free_at: NaiveDateTime }, Available, Blocked }` com `as_str()` (`occupied`, `awaiting_cleaning`, `cleaning`, `clean_waiting_interval`, `available`, `blocked`).
  - `struct RoomFacts { in_house: bool, cleaning: Option<CleaningFact>, blocked_now: bool }`; `struct CleaningFact { status: CleaningStatus, completed_at: Option<NaiveDateTime> }`.
  - `fn room_state(facts: &RoomFacts, interval_minutes: u32, now: NaiveDateTime) -> RoomState` — precedência: `in_house` ⇒ Occupied; `blocked_now` ⇒ Blocked; limpeza `awaiting` ⇒ AwaitingCleaning; `in_progress` ⇒ Cleaning; `done` com `completed_at + intervalo > now` ⇒ CleanWaitingInterval; senão Available.
  - `enum Priority { Urgent, High, Normal, Low }` (`as_str`: `urgent|high|normal|low`, `parse`).
  - `fn suggest_priority(next_arrival: Option<NaiveDateTime>, now: NaiveDateTime) -> Priority` — `Urgent` se chegada ≤ `now + 4h` (inclui já passada); `High` se mesma data de `now`; `Normal` se depois; `Low` se `None`.

- [x] **Step 1: Write the failing tests** (`room_state.rs`): `occupied_wins_over_everything_else`; `blocked_wins_over_cleaning`; `awaiting_and_in_progress_map_to_their_states`; `done_cleaning_waits_for_the_interval` (concluída 10:00, intervalo 60: às 10:59 ⇒ `CleanWaitingInterval{free_at 11:00}`, às 11:00 ⇒ `Available`); `no_cleaning_history_is_available`; `time_alone_never_clears_awaiting_cleaning` (limpeza `awaiting` e `now` +30 dias ⇒ ainda `AwaitingCleaning`); `interval_zero_is_available_right_after_cleaning`. (`cleaning.rs`): `arrival_in_4h_is_urgent` (exato 4h ⇒ urgent; 4h01 no mesmo dia ⇒ high); `arrival_already_late_is_urgent`; `arrival_today_is_high`; `arrival_tomorrow_across_midnight_is_normal` (now 23:30, chegada 00:30 do dia seguinte, 1h ⇒ urgent; chegada 14:00 do dia seguinte ⇒ normal); `no_arrival_is_low`.
- [x] **Step 2: Run** `cargo test domain::room_state domain::cleaning` — Expected: FAIL (módulos inexistentes).
- [x] **Step 3: Implement** as funções acima; sem banco, sem relógio global.
- [x] **Step 4: Run** os mesmos testes — Expected: PASS; `cargo clippy --all-targets -- -D warnings` limpo.
- [x] **Step 5: Commit** `feat: pure room state and cleaning priority`

---

### Task 3: Repositórios e modelos (limpeza, bloqueio, manutenção)

**Files:**
- Create: `models/cleaning.rs`, `models/block.rs`, `models/maintenance.rs`, `repositories/cleaning_repo.rs`, `repositories/block_repo.rs`, `repositories/maintenance_repo.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `repositories/catalog_repo.rs` (ler/gravar `cleaning_interval_minutes` do quarto), `models/catalog.rs`

**Interfaces:**
- Produces (todas em `&Connection`/`&Transaction`, `version` checada em escritas):
  - `cleaning_repo::insert_awaiting(tx, room_id, stay_id, ready_since, at) -> i64`; `find(conn, id)`; `open_for_room(conn, room_id) -> Option<CleaningTask>` (status ≠ done); `last_done_for_room(conn, room_id) -> Option<CleaningTask>`; `start(tx, id, user, expected_version, at)`; `complete(tx, id, user, expected_version, at)`; `set_priority(tx, id, Option<Priority>, justification, expected_version, at)`; `queue(conn) -> Vec<CleaningTask>` (abertas, com código/categoria do quarto).
  - `block_repo::insert(tx, NewBlock, at) -> i64`; `find`; `overlapping(conn, room_id, start, end: Option<NaiveDate>) -> Vec<RoomBlock>` (blocos `preventive|active|awaiting_release` que tocam `[start, end)` com `end` nulo = infinito); `active_now(conn, room_id, today) -> bool` (`active|awaiting_release` cobrindo `today`); `set_status(tx, id, status, expected_version, at)`.
  - `maintenance_repo::insert_order`, `find_order`, `set_status`, `insert_conflict`, `open_conflicts(conn, order_id)`, `resolve_conflict(tx, id, user, note, at)`, `list(conn, filter)`.
  - Em cada `models/*`: structs `Serialize` (`CleaningTask`, `RoomBlock`, `MaintenanceOrder`, `MaintenanceConflict`) com campos de nome do quarto para exibição.

- [x] **Step 1: Write the failing tests** em cada repositório (banco em memória via `test_support`): `cleaning_repo` — `insert_awaiting_twice_for_same_stay_fails`, `complete_with_stale_version_is_conflict`, `queue_orders_open_tasks_only`; `block_repo` — `overlapping_uses_half_open_periods` (bloqueio 10→13: período 13→15 não toca; 12→14 toca; sem fim toca tudo depois), `released_blocks_are_ignored`, `active_now_ignores_preventive`; `maintenance_repo` — `open_conflicts_lists_only_open`.
- [x] **Step 2: Run** `cargo test repositories::cleaning_repo repositories::block_repo repositories::maintenance_repo` — Expected: FAIL.
- [x] **Step 3: Implement** modelos e repositórios (SQL escrito à mão, mesmo estilo de `stay_repo`).
- [x] **Step 4: Run** os testes — Expected: PASS.
- [x] **Step 5: Commit** `feat: cleaning, block and maintenance repositories`

---

### Task 4: `CleaningService` e geração da tarefa no check-out

**Files:**
- Create: `services/cleaning_service.rs`
- Modify: `services/mod.rs`, `app.rs` (campo `cleaning`), `services/stay_service.rs` (`checkout`: inserir tarefa na transação, auditar)

**Interfaces:**
- Consumes: `cleaning_repo::*`, `domain::cleaning::suggest_priority`, `reservation_repo` (próxima chegada do quarto em `pending|confirmed`).
- Produces:
  - `CleaningService::queue(&self, token) -> Result<Vec<CleaningItem>, AppError>` — `CleaningItem { task, room_code, priority: String, suggested_priority: String, next_arrival: Option<String> }`; `priority` = override se houver, senão sugerida; ordenação: urgente → baixa, depois `ready_since` mais antigo.
  - `start(&self, token, task_id, expected_version) -> Result<(), AppError>` (`awaiting → in_progress`, registra usuário/hora).
  - `complete(&self, token, task_id, expected_version) -> Result<(), AppError>` (`awaiting|in_progress → done`; permite concluir direto sem iniciar, registrando início = conclusão? **não**: exige `in_progress`; mensagem "Inicie a limpeza antes de concluir").
  - `set_priority(&self, token, task_id, priority: Option<String>, justification: String, expected_version) -> Result<(), AppError>` (justificativa obrigatória; `None` remove o ajuste).
  - Auditoria `module: "cleaning"`, operações `cleaning_started|cleaning_completed|cleaning_priority_changed`, na mesma transação.

- [x] **Step 1: Write the failing tests** (`cleaning_service`, mesmo `World` do `stay_service`): `checkout_creates_one_awaiting_task_with_the_effective_time`; `checkout_failure_leaves_no_task` (saldo ≠ 0 sem exceção); `double_checkout_does_not_duplicate_task`; `start_then_complete_records_user_and_time`; `complete_without_start_is_refused`; `double_complete_is_conflict_and_keeps_first_user`; `queue_orders_by_priority_then_age` (quarto com chegada em 2 h antes de quarto sem reserva); `late_departure_task_uses_the_real_time_and_urgent_priority_follows_the_arrival`; `override_needs_a_justification_and_is_audited`; `clearing_override_returns_to_the_suggestion`; `recepcao_can_start_and_complete`; `check_out_audit_is_in_the_same_transaction`.
- [x] **Step 2: Run** `cargo test cleaning_service` — Expected: FAIL.
- [x] **Step 3: Implement** o serviço; em `checkout`, após `reservation_repo::set_status(... "completed" ...)`, chamar `cleaning_repo::insert_awaiting(&tx, room_id, stay.id, at, at)`.
- [x] **Step 4: Run** `cargo test cleaning_service stay_service` — Expected: PASS (testes do Marco 3 continuam verdes).
- [x] **Step 5: Commit** `feat: cleaning queue and task created at checkout`

---

### Task 5: Intervalo por quarto e portão "quarto limpo" no check-in

**Files:**
- Create: `src-tauri/src/services/room_rules.rs` (ou função em `room_status_service.rs`; ficar com um só lugar)
- Modify: `services/reservation_service.rs` (create, `plan_change`, `check_availability`), `services/stay_service.rs` (`check_in`, `walk_in`), `services/catalog_service.rs` (editar intervalo do quarto, `ManageCatalog`), `repositories/catalog_repo.rs`

**Interfaces:**
- Produces: `effective_cleaning_minutes(conn, room_id, settings) -> Result<u32, AppError>` (override do quarto ou padrão); `ensure_room_clean(conn, room_id, interval, now: NaiveDateTime) -> Result<(), AppError>` — recusa com mensagem "O quarto {código} está aguardando limpeza." / "…está em limpeza." / "…foi limpo e fica livre às HH:MM de DD/MM/AAAA (intervalo mínimo)."
- Consumes: `cleaning_repo::open_for_room`, `last_done_for_room`.

- [x] **Step 1: Write the failing tests:** `reservation_service` — `room_interval_override_changes_the_same_day_turnover` (quarto com 180 min: saída 12:00 e entrada 14:00 do mesmo dia conflita; com padrão 60 passa); `stay_service` — `check_in_is_refused_while_cleaning_is_awaiting`, `…while_cleaning_is_in_progress`, `…before_the_interval_after_cleaning_is_done` (concluída 10:00, intervalo 60, check-in 10:30 recusado e 11:00 aceito), `check_in_allowed_in_a_room_never_used`, `walk_in_is_refused_in_a_dirty_room`, `refused_check_in_writes_nothing`; `catalog_service` — `room_interval_needs_manage_catalog_and_accepts_0_to_1440`.
- [x] **Step 2: Run** `cargo test room_interval check_in_is walk_in_is` — Expected: FAIL.
- [x] **Step 3: Implement** os helpers e trocar `settings.cleaning_interval_minutes` por `effective_cleaning_minutes(...)` em todas as chamadas de `find_conflicts`; chamar `ensure_room_clean` no check-in e no walk-in, dentro da transação, com `now = self.ctx.local_now()`.
- [x] **Step 4: Run** `cargo test` completo — Expected: PASS (os 295 existentes + novos; ajustar fixtures do Marco 3 que fazem check-out e depois novo check-in no mesmo quarto, concluindo a limpeza no teste).
- [x] **Step 5: Commit** `feat: per-room cleaning interval and clean-room gate at check-in`

---

### Task 6: `BlockService` e bloqueios na disponibilidade

**Files:**
- Create: `services/block_service.rs`
- Modify: `services/mod.rs`, `app.rs`, `services/reservation_service.rs` (create, `plan_change`, `check_availability`), `services/stay_service.rs` (`check_in`, `walk_in`)

**Interfaces:**
- Produces:
  - `BlockService::create(&self, token, NewBlockInput { room_id, start_date, end_date: Option<String>, reason }) -> Result<i64, AppError>` — exige `ReleaseRoom` (M4-5); motivo 1–500; início ≥ hoje; fim > início; quarto ativo; recusa com a lista de reservas ativas que tocam o período (M4-7) e se há hospedagem em curso no período.
  - `release(&self, token, block_id, justification, expected_version)` — exige `ReleaseRoom`; `active|awaiting_release|preventive → released`.
  - `list(&self, token, include_released: bool) -> Vec<RoomBlock>`.
  - `pub(crate) fn blocks_in_period(conn, room_id, start, end) -> Result<Vec<RoomBlock>, AppError>` e `blocks_message(&[RoomBlock]) -> String` ("O quarto 101 está bloqueado de 10/11/2026 a 13/11/2026 (manutenção)."), usados por reserva, alteração, consulta, check-in e walk-in antes de gravar.

- [x] **Step 1: Write the failing tests:** `create_needs_release_room_and_valid_input` (sem permissão; motivo vazio/só espaços; fim ≤ início; início no passado; quarto inativo/inexistente — nada gravado); `create_over_an_active_reservation_is_refused_listing_it`; `create_over_a_stay_in_house_is_refused`; `reservation_over_a_block_is_refused`; `reservation_ending_the_day_the_block_starts_is_allowed`; `open_ended_block_blocks_every_future_date`; `change_into_a_blocked_period_is_unfeasible`; `check_availability_reports_the_block`; `check_in_and_walk_in_respect_a_block_starting_today`; `release_needs_permission_and_a_justification_and_double_release_is_conflict`; `released_block_stops_blocking`; `block_events_are_audited` (`block_created`, `block_released`).
- [x] **Step 2: Run** `cargo test block_service` — Expected: FAIL.
- [x] **Step 3: Implement** o serviço e plugar `blocks_in_period` nas cinco chamadas; a mensagem de bloqueio vem **antes** da de conflito de reserva.
- [x] **Step 4: Run** `cargo test` — Expected: PASS.
- [x] **Step 5: Commit** `feat: room blocks integrated with availability`

---

### Task 7: `MaintenanceService` e processo de conflitos

**Files:**
- Create: `services/maintenance_service.rs`
- Modify: `services/mod.rs`, `app.rs`, `repositories/sequence_repo.rs` (sequência `maintenance`, se o padrão exigir cadastro)

**Interfaces:**
- Produces:
  - `open(&self, token, NewOrderInput { room_id, description, priority, planned_start, planned_end: Option<String>, responsible, notes }) -> Result<OpenedOrder, AppError>` — `OpenedOrder { id, code, conflicts: Vec<ConflictInfo> }`; cria bloqueio `preventive` (se há conflitos) ou `active` (se não há) com `kind='maintenance'`; sem conflito e sem hospedagem no período o bloqueio já é efetivo.
  - `apply_unavailability(&self, token, order_id, expected_version) -> Result<ApplyResult, AppError>` — reavalia conflitos (M4-8); `ApplyResult { still_open: Vec<ConflictInfo>, effective: bool }`; com zero abertos promove o bloqueio a `active`.
  - `resolve_conflict_manually(&self, token, conflict_id, note)` — nota obrigatória, auditado.
  - `start(&self, token, order_id, expected_version)` (`open → in_progress`, `actual_start`; recusa se há conflitos abertos); `request_resolution(…)` (`in_progress → awaiting_resolution`); `complete(…)` (`awaiting_resolution → completed`, `actual_end`, bloqueio → `awaiting_release`; **não libera**); `list`, `get`.
  - Auditoria `module: "maintenance"`; códigos `MAN-000001`.

- [x] **Step 1: Write the failing tests:** `open_without_conflict_blocks_effectively_and_returns_a_code`; `open_with_conflicts_creates_a_preventive_block_and_lists_the_reservations`; `preventive_block_refuses_new_reservations_in_the_period`; `applying_with_open_conflicts_keeps_the_preventive_block`; `cancelling_the_affected_reservation_then_applying_makes_it_effective`; `manual_resolution_needs_a_note_and_is_audited`; `start_is_refused_with_open_conflicts`; `state_machine_rejects_skipped_steps` (`open → completed`, voltar atrás); `complete_does_not_release_the_room` (estado continua bloqueado; check-in recusado); `release_after_completion_needs_release_room` (integra com `BlockService::release`); `double_complete_is_conflict`; `invalid_input_writes_nothing` (descrição vazia, prioridade desconhecida, período invertido, quarto inativo); `order_with_a_stay_in_house_in_the_period_lists_it_as_a_conflict`; `all_transitions_are_audited_in_the_same_transaction`.
- [x] **Step 2: Run** `cargo test maintenance_service` — Expected: FAIL.
- [x] **Step 3: Implement** a máquina de estados da ordem como função pura em `domain/maintenance.rs` (`transition(from, to) -> Result<(), AppError>`, mesmo padrão de `domain/stay.rs`) e o serviço sobre ela.
- [x] **Step 4: Run** `cargo test maintenance` — Expected: PASS; `cargo clippy --all-targets -- -D warnings` limpo.
- [x] **Step 5: Commit** `feat: maintenance orders with conflict resolution`

---

### Task 8: `RoomStatusService` — estado por quarto e mapa

**Files:**
- Create: `services/room_status_service.rs`, `models/room_status.rs`
- Modify: `services/mod.rs`, `app.rs`, `models/room_map.rs` (`RoomRow.state`, `RoomRow.free_at`), `services/room_map_service.rs`

**Interfaces:**
- Consumes: `domain::room_state::room_state`, `effective_cleaning_minutes`, `cleaning_repo`, `block_repo::active_now`, `stay_repo` (hospedagem em curso do quarto).
- Produces: `RoomStatusService::list(&self, token) -> Vec<RoomStatus>` com `RoomStatus { room_id, code, category_name, state: String, free_at: Option<String>, label_pt: String, cleaning_task_id: Option<i64>, next_arrival: Option<String> }`; `RoomRow` do mapa ganha `state` e `free_at`. Rótulos: `Ocupado`, `Aguardando limpeza`, `Em limpeza`, `Limpo — aguardando intervalo`, `Disponível`, `Em manutenção ou bloqueado`.

- [x] **Step 1: Write the failing tests:** matriz ponta a ponta com o relógio injetado — `state_matrix_follows_the_whole_cycle` (reserva → check-in ⇒ `occupied` → check-out ⇒ `awaiting_cleaning` → iniciar ⇒ `cleaning` → concluir 10:00 ⇒ `clean_waiting_interval` com `free_at` 11:00 → 11:00 ⇒ `available`); `advancing_the_clock_never_frees_a_dirty_room`; `an_effective_block_today_shows_blocked_and_a_future_block_does_not`; `completed_maintenance_still_shows_blocked_until_release`; `inactive_rooms_are_listed_as_inactive_state_none` (decisão: inativo não entra na lista de estado; o mapa continua mostrando-o como já faz); `room_map_rows_carry_the_state`.
- [x] **Step 2: Run** `cargo test room_status` — Expected: FAIL.
- [x] **Step 3: Implement** a montagem dos fatos por quarto (uma consulta por tabela, sem N+1 por quarto) e a chamada à função pura.
- [x] **Step 4: Run** `cargo test` — Expected: PASS.
- [x] **Step 5: Commit** `feat: calculated room state in the room map`

---

### Task 9: Comandos Tauri e `api.ts`

**Files:**
- Create: `commands/cleaning.rs`, `commands/blocks.rs`, `commands/maintenance.rs`, `commands/room_status.rs`
- Modify: `commands/mod.rs`, `lib.rs` (`generate_handler!`), `src/lib/api.ts`, `src/lib/api.test.ts`

**Interfaces:**
- Produces comandos (token em todos): `cleaning_queue`, `start_cleaning(task_id, expected_version)`, `complete_cleaning(task_id, expected_version)`, `set_cleaning_priority(task_id, priority, justification, expected_version)`, `list_blocks(include_released)`, `create_block(input)`, `release_block(block_id, justification, expected_version)`, `list_maintenance`, `get_maintenance(id)`, `open_maintenance(input)`, `apply_maintenance_unavailability(id, expected_version)`, `resolve_maintenance_conflict(conflict_id, note)`, `start_maintenance`, `request_maintenance_resolution`, `complete_maintenance`, `list_room_status`, `set_room_cleaning_interval(room_id, minutes, expected_version)`. Em `api.ts`, funções e tipos TS espelhando os modelos Rust (mesmo estilo dos de `stays`).

- [x] **Step 1: Write the failing tests** em `api.test.ts`: cada função chama `invoke` com o nome e os argumentos esperados (`completeCleaning(7, 3)` ⇒ `invoke("complete_cleaning", { token, taskId: 7, expectedVersion: 3 })`), e erros `Conflict` chegam traduzidos como nos demais.
- [x] **Step 2: Run** `npm test -- api` — Expected: FAIL.
- [x] **Step 3: Implement** comandos finos (sem regra) e funções em `api.ts`; registrar tudo em `generate_handler!`.
- [x] **Step 4: Run** `npm test -- api` e `cargo build` — Expected: PASS / compila.
- [x] **Step 5: Commit** `feat: cleaning, block and maintenance commands`

---

### Task 10: UI — fila de limpeza

**Files:**
- Create: `src/pages/cleaning/CleaningQueuePage.tsx`, `PriorityDialog.tsx`, `CleaningQueuePage.test.tsx`; `src/lib/roomState.ts`, `roomState.test.ts`
- Modify: `src/routes.tsx` (`/limpeza`), `src/components/layout/Sidebar.tsx`

**Interfaces:**
- Produces: `roomState.ts` — `ROOM_STATE_LABEL: Record<RoomStateKey, string>`, `ROOM_STATE_STYLE` (classe de cor **e** ícone `lucide-react` por estado), usado também pelo mapa (T12). `CleaningQueuePage`: lista por prioridade (rótulo `Urgente|Alta|Normal|Baixa` + cor + ícone), mostra quarto, "saiu às HH:MM", próxima chegada, botões `Iniciar limpeza` / `Concluir limpeza` conforme o estado e `Ajustar prioridade` (diálogo com justificativa obrigatória).

- [x] **Step 1: Write the failing tests:** `shows_priority_label_not_only_color`; `awaiting_task_offers_start_and_in_progress_offers_complete`; `completing_calls_the_api_with_the_task_version_and_shows_the_result` ("Limpeza do quarto 101 concluída. Livre às 11:00."); `a_conflict_error_reloads_and_explains`; `priority_dialog_requires_a_justification_and_keeps_the_text_on_failure`; `empty_queue_says_there_is_nothing_to_clean`; `roomState.test` — todo estado tem rótulo, cor e ícone distintos.
- [x] **Step 2: Run** `npm test -- CleaningQueuePage roomState` — Expected: FAIL.
- [x] **Step 3: Implement** página, diálogo (React Hook Form + Zod) e consultas TanStack Query com invalidação da fila e do mapa após cada ação.
- [x] **Step 4: Run** `npm test -- cleaning roomState` — Expected: PASS.
- [x] **Step 5: Commit** `feat: cleaning queue page`

---

### Task 11: UI — bloqueios e manutenção

**Files:**
- Create: `src/pages/blocks/BlocksPage.tsx`, `BlockDialog.tsx`; `src/pages/maintenance/MaintenancePage.tsx`, `MaintenanceOrderPage.tsx`, `OrderDialog.tsx` (+ `*.test.tsx`)
- Modify: `src/routes.tsx` (`/bloqueios`, `/manutencao`, `/manutencao/:id`), `Sidebar.tsx`, `src/pages/catalog/RoomsPage.tsx` (campo "Intervalo de limpeza (min)" do quarto)

**Interfaces:**
- Consumes: funções de `api.ts` (T9), `ROOM_STATE_LABEL`.
- Produces: `BlocksPage` (`Criar bloqueio`, `Encerrar bloqueio`/`Liberar quarto` só para quem tem `ReleaseRoom`, situação com rótulo+ícone); `MaintenanceOrderPage` mostra a máquina de estados, as reservas afetadas (com link para a reserva, sem perder a ordem aberta), botões `Aplicar indisponibilidade`, `Iniciar manutenção`, `Concluir manutenção` e, após concluir, o aviso "O quarto continua bloqueado até ser liberado".

- [x] **Step 1: Write the failing tests:** `block_dialog_keeps_values_and_shows_the_conflicting_reservations_on_refusal`; `release_button_hidden_without_permission_but_backend_error_is_explained`; `maintenance_order_lists_affected_reservations_and_blocks_start_while_open`; `complete_shows_that_the_room_is_still_blocked`; `open_order_dialog_validates_dates_and_description`; `rooms_page_edits_the_cleaning_interval_with_limits_0_to_1440`.
- [x] **Step 2: Run** `npm test -- blocks maintenance RoomsPage` — Expected: FAIL.
- [x] **Step 3: Implement** as páginas e diálogos; datas `DD/MM/AAAA` na tela e `YYYY-MM-DD` na API.
- [x] **Step 4: Run** `npm test` — Expected: PASS.
- [x] **Step 5: Commit** `feat: blocks and maintenance pages`

---

### Task 12: UI — mapa da recepção com estados

**Files:**
- Modify: `src/pages/calendar/RoomMapPage.tsx`, `RoomMapPage.test.tsx`, `src/pages/Dashboard.tsx` (atalho "Quartos aguardando limpeza: N" levando a `/limpeza`)

**Interfaces:**
- Consumes: `RoomRow.state`/`free_at` (T8), `ROOM_STATE_*` (T10).

- [x] **Step 1: Write the failing tests:** `each_room_header_shows_the_state_label_and_icon` (um quarto por estado, rótulos exatos de T8); `clean_waiting_interval_shows_the_free_time`; `legend_lists_all_six_states`; `clicking_a_room_awaiting_cleaning_goes_to_the_queue_without_losing_the_map_date` (preserva a agenda, CLAUDE.md).
- [x] **Step 2: Run** `npm test -- RoomMapPage` — Expected: FAIL.
- [x] **Step 3: Implement** a coluna de estado do quarto e a legenda.
- [x] **Step 4: Run** `npm test` e `npm run build` — Expected: PASS / `tsc` + `vite build` ok.
- [x] **Step 5: Commit** `feat: room map with operational states`

---

### Task 13: Fechamento — docs, changelog, grafo, revisão

**Files:**
- Create: `docs/usuario/limpeza.md`, `docs/usuario/bloqueios-e-manutencao.md`, `docs/desenvolvimento/limpeza-e-estado-do-quarto.md`, `docs/desenvolvimento/bloqueios-e-manutencao.md`
- Modify: `CHANGELOG.md` (`Unreleased`: Added fila de limpeza, estados do quarto, bloqueios e ordens de manutenção; Changed check-in exige quarto limpo), `local/spec.md` (Anexo A.4: M4-10 provisório; D-19 atualizado), `local/plano-completo.md` (marcar Marco 4), `docs/desenvolvimento/hospedagem-e-conta.md` (remover a pendência M3-10), este plano

- [x] **Step 1:** Escrever a documentação de usuário (objetivo, quando usar, passo a passo, resultado, avisos, erros comuns: "quarto aguardando limpeza", "quarto bloqueado") e de desenvolvimento (regras, estado calculado, transições, auditoria, decisões M4-1 a M4-11, limitações, testes).
- [x] **Step 2:** Rodar tudo: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm test`, `npm run build`. Expected: tudo verde; registrar contagens no Status.
- [x] **Step 3:** `graphify update .` e anotar no Status.
- [x] **Step 4:** Revisão final da branch por revisor independente; corrigir Críticos/Importantes (TDD); registrar menores adiados.
- [x] **Step 5: Commit** `docs: close marco 4`

---

## Status

**Implementado** na branch `marco-4-limpeza` (a partir de `marco-3-hospedagem`, que ainda não foi integrada em `dev`). T1 a T13 concluídas; falta a revisão final independente (registrada abaixo quando feita). Migração `0010`.

- **Testes:** `cargo test` 384 unitários + 4 de integração; `cargo clippy --all-targets -- -D warnings` limpo; `npm test` 296 (42 arquivos); `npx tsc --noEmit` limpo e `npm run build` ok; `graphify update .` executado.
- **Desvios do plano:** (a) a numeração do Anexo A ficou D-20 a D-23 (prioridade, disponibilidade, permissões, bloqueios) em vez de uma única linha; (b) `AvailabilityResult` ganhou `blocks`, `Room` ganhou `cleaning_interval_minutes` e `RoomRow` ganhou `state`/`state_label`/`free_at`; (c) o intervalo por quarto é editado por `set_room_cleaning_interval` (não por `UpdateRoom`); (d) consultas de estado por quarto, sem lote; (e) `BlockService::release` recusa liberar bloqueio de manutenção enquanto a ordem não está concluída; (f) a data do mapa passou para o parâmetro `?de=` da URL; (g) o Dashboard ganhou o atalho "Quartos aguardando limpeza". Rulings completos: ledger da execução.
- **Processo:** vários testes foram escritos junto da implementação em funções puras (T2, `roomState.ts`) sem execução RED separada; nas demais tarefas o RED foi observado. `cargo fmt --check` já falhava na base (701 diffs); não foi aplicado.
- **Pendências de verificação:** `npm run tauri dev` com a interface real não foi exercitado (responsável). Testes de interface do primeiro `findByRole("row")` de algumas páginas ficam lentos com a suíte inteira em paralelo (timeout elevado nos novos).

## Marcos seguintes

- **Marco 5 — Financeiro:** recebimentos divididos, devoluções, duplicidade, fechamento de caixa (`payment_components`; migração `0011`).
- **Marco 6 — Alertas, relatórios e documentos:** alertas persistentes (saída atrasada, risco de conflito, limpeza urgente parada), relatórios e documentos internos.

## Self-Review

- **Cobertura da spec:** §7 estados e fila → T2/T4/T8/T10/T12; intervalo por quarto → T5/T11; prioridade e ajuste auditado → T2/T4/T10; saída atrasada → T4 (M4-11); §8 bloqueios → T6/T11; ordens, estados e conflitos → T7/T11; preventivo → T6/T7; conclusão não libera + `ReleaseRoom` → T7 (M4-9); pendência D-19/M3-10 → T5.
- **Consistência de tipos:** `RoomState`/`Priority` (T2) usados por T4, T8, T10; `effective_cleaning_minutes` (T5) por T8; `blocks_in_period` (T6) por T5/T7; `ConflictInfo` já existente reaproveitado em T6/T7.
- **Migrações:** `0010` única; Marco 5 usa `0011`.
- **Valores provisórios:** só a prioridade (4 h / hoje) é número novo; vive em constantes nomeadas em `domain/cleaning.rs` e entra no Anexo A.4 como **aguardando aprovação**.
- **Proporção:** corpos de função a cargo do implementador; testes trazem os valores exatos (intervalo 60 min, 10:00 → 11:00, chegada em 4 h, `MAN-000001`).
