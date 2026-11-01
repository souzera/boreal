# Marco 6.2 — Alertas persistentes e busca global Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar à recepção um painel de alertas persistentes e acionáveis (com prioridade, histórico e link para a origem) e uma busca global por reservas, hóspedes, hospedagens e movimentos financeiros que respeita permissões.

**Architecture:** Alertas são **fatos persistidos** (`alerts`, `alert_events`) mantidos por *detectores*: funções que olham o estado atual (reservas, hospedagens, limpeza, manutenção, contas, caixa) e devolvem "achados"; o motor `AlertService::refresh` sincroniza os achados com a tabela (cria, atualiza, resolve sozinho quando a causa some, nunca apaga). Alertas críticos só se resolvem quando o problema é resolvido. A busca é um serviço de leitura (`SearchService`) com consultas parametrizadas por grupo e filtros de permissão no backend.

**Tech Stack:** Rust/rusqlite/SQLCipher, React/TS/TanStack Query/Vitest. Sem dependências novas.

**Spec:** `local/spec.md` §20 (pesquisa e alertas), §13 (permissões e auditoria); `local/brief.md` §3.1, §4. Roteiro: `local/plano-completo.md` §10, tarefas 6.1 (alertas) e 6.4 (pesquisa). Parte **6.2 de 4** do Marco 6.

**Branch:** `marco-6-2-alertas`, a partir de `marco-6-1-visual` (6.1 ainda não integrada). Migração **`0012`**.

## Decisões deste plano (chamadas minhas; revisar antes ou depois)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| A1 | Alertas vêm de **detectores** que rodam em `refresh` (ao abrir o painel/contagem e a cada 60 s na tela). Não há ganchos em cada operação. | Simples, idempotente e sem acoplar os serviços já prontos. Custo: um alerta pode demorar até 60 s para aparecer; o `refresh` roda também quando a tela abre. |
| A2 | **Tipos e prioridades:** `arrival_today` normal; `departure_today` normal; `arrival_overdue` alta (possível no-show); `deposit_overdue` alta; `departure_late` alta; `departure_risk` **crítico**; `maintenance_conflict` **crítico**; `cleaning_urgent` alta; `payable_overdue` alta (só com `ManagePayables`); `checkout_balance` alta (hospedagem em andamento, saída hoje ou antes, saldo ≠ 0); `closing_pending` normal. `backup_missing`, `disk_low`, `update_failed` ficam **registrados como tipos críticos**, sem detector: o Marco 7 os emite. | Cobre a lista da spec §20.2; backup/disco/atualização dependem de funcionalidades que não existem. |
| A3 | **Resolução:** automática quando a causa some (`resolved`, com evento). **Dispensa manual** (`dismissed`) só para alertas **não críticos**, com justificativa (≤ 200), qualquer usuário que enxerga o alerta; auditada em `audit_log`. Alerta crítico nunca é dispensado: só some quando a causa é resolvida. | Spec §20.2: crítico não é "simplesmente apagado". |
| A4 | **Nada é apagado.** Alerta resolvido/dispensado fica como histórico. Se a causa volta depois de resolvida, nasce um **novo** alerta. Um alerta **dispensado** não reaparece enquanto a mesma causa continuar; se a causa sumir e voltar, nasce outro. | Evita o alerta que "volta" a cada minuto e preserva o histórico. |
| A5 | **Visibilidade por permissão no backend:** `payable_overdue` só aparece (lista, contagem e histórico) a quem tem `ManagePayables`; os demais, a todos os autenticados. | Spec §13/§20.1: resultados respeitam permissões. |
| A6 | **Busca:** `LIKE` parametrizado sobre campos já indexados/normalizados (código, `name_key`, telefone e CPF só dígitos, código do quarto), com **escape** de `%`, `_` e `\`; mínimo de 2 caracteres; até 8 resultados por grupo mais o total. Sem FTS5. Grupos: hóspedes, reservas, hospedagens, movimentos (recebimentos `PAG-`, devoluções `DEV-` e, com `ManagePayables`, contas a pagar `CAP-`). Teste de volume: 5 anos fictícios. | FTS5 não é necessário no volume de uma pousada e evita depender do suporte no SQLCipher; se o teste de volume falhar, indexa-se. |
| A7 | O painel **Alertas** substitui o "Em breve" do menu e mostra a contagem (crítico/total) ao lado do item; a tela **Recepção** (atual `/`) mostra os alertas mais urgentes com link para a origem. A Recepção definitiva é a 6.3. | Fecha a promessa de §20.2 ("principais também aparecem no dashboard e na recepção"). |

## Global Constraints

- Alertas persistem até a resolução; crítico não pode ser apagado nem dispensado; o histórico é preservado (spec §20.2).
- Cada alerta diz **o que ocorreu**, **por que importa** e abre o **registro relacionado** (brief §4).
- Operações relacionadas em transação; dispensa manual auditada na mesma transação (CLAUDE.md).
- Permissões verificadas no backend, inclusive nas contagens e na busca (CLAUDE.md, spec §13).
- Datas `DD/MM/AAAA`, horas `HH:MM`, valores `R$ 150,00`; "hoje" é a data **local** (`ctx.today()`); relógios injetados.
- Cor nunca é o único sinal: prioridade com rótulo + ícone (6.1).
- Sem dados pessoais em logs; a busca por CPF compara só dígitos e o resultado mostra o CPF mascarado como o restante do app.
- Rótulos: `Dispensar alerta`, `Abrir registro`; prioridades `Crítico`, `Alto`, `Normal`.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano e o grafo.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Crítico não some por engano:** dispensar crítico é recusado; `refresh` com a causa ainda presente nunca resolve; nenhuma linha de `alerts`/`alert_events` é apagada em nenhum caminho (T2).
2. **Idempotência:** dois `refresh` seguidos (e dois cliques em `Dispensar alerta`) não duplicam alerta nem evento (T2).
3. **Ciclo da causa:** some ⇒ resolve; volta ⇒ novo alerta; dispensado + causa contínua ⇒ não reaparece; some e volta ⇒ novo alerta (T2).
4. **Permissão:** quem não tem `ManagePayables` não vê `payable_overdue` na lista, na contagem nem no histórico; a busca não devolve contas a pagar para essa pessoa (T2, T6).
5. **Virada de dia e fuso:** chegada de hoje às 23:30 locais (já dia seguinte em UTC) ainda é "hoje"; no dia seguinte, sem check-in, o alerta `arrival_today` se resolve e nasce `arrival_overdue` (T3).
6. **Busca:** `%`, `_` e `\` no texto são tratados como letras; texto de 1 caractere é recusado; CPF com ou sem máscara; código em minúsculas; resultados limitados; volume de 5 anos dentro do limite (T6).

---

## File Structure

```
src-tauri/src/
  migrations/sql/0012_alerts.sql                 # alerts, alert_events
  domain/alert.rs                                # AlertKind, Severity, Finding, regras de dispensa (puro)
  models/alert.rs, search.rs
  repositories/alert_repo.rs, search_repo.rs
  services/alert_service.rs                      # motor (refresh/sync), lista, resumo, dispensa, histórico
  services/alert_detectors.rs                    # um detector por tipo (usa repositórios e regras existentes)
  services/search_service.rs
  commands/alerts.rs, search.rs
  services/stay_service.rs                       # assess_departure -> pub(crate)
src/
  lib/api.ts                                     # tipos e chamadas de alertas e busca
  lib/alerts.ts                                  # rótulos, ícones e ordem
  pages/alerts/AlertsPage.tsx, AlertList.tsx, DismissDialog.tsx
  pages/Search.tsx                               # resultados agrupados
  pages/Dashboard.tsx                            # alertas mais urgentes
  components/layout/{NavItems.ts,Sidebar.tsx}    # Alertas habilitado, com contagem
  routes.tsx
docs/usuario/{alertas,busca}.md   docs/desenvolvimento/{alertas,busca}.md
```

---

### Task 1: Migração `0012` e regras puras de alerta

**Files:**
- Create: `src-tauri/src/migrations/sql/0012_alerts.sql`, `src-tauri/src/domain/alert.rs`
- Modify: `src-tauri/src/migrations/mod.rs` (versão 12; ajustar os testes que afirmam a versão 11), `src-tauri/src/domain/mod.rs`
- Test: `migrations/mod.rs`, `domain/alert.rs`

**Interfaces:**
- Produces (tabelas): `alerts(id, kind, subject_key, severity 'critical'|'high'|'normal', title, detail, link, status 'active'|'resolved'|'dismissed', still_present 0|1, first_seen_at, last_seen_at, resolved_at NULL, resolved_by NULL, resolution_note NULL, version)`; índice único parcial `idx_alerts_one_active ON alerts(kind, subject_key) WHERE status IN ('active')`; índice único parcial `idx_alerts_one_dismissed ON alerts(kind, subject_key) WHERE status = 'dismissed' AND still_present = 1`; `alert_events(id, alert_id, at, user_id NULL, event 'raised'|'updated'|'resolved'|'dismissed', note NULL)` ligada com `ON DELETE RESTRICT` e sem `DELETE` permitido por gatilho (`BEFORE DELETE ... RAISE(ABORT, 'histórico de alertas é imutável')`) em ambas as tabelas.
- Produces (domínio): `enum AlertKind { ArrivalToday, DepartureToday, ArrivalOverdue, DepositOverdue, DepartureLate, DepartureRisk, MaintenanceConflict, CleaningUrgent, PayableOverdue, CheckoutBalance, ClosingPending, BackupMissing, DiskLow, UpdateFailed }` com `as_str()` (`arrival_today`…), `parse`, `severity() -> Severity`, `label() -> &'static str` (rótulo curto em português), `requires_permission() -> Option<Permission>` (`PayableOverdue` ⇒ `ManagePayables`), `managed_by_detectors() -> bool` (falso para `BackupMissing | DiskLow | UpdateFailed`); `enum Severity { Critical, High, Normal }` com `rank()` (0, 1, 2) e `as_str()`; `struct Finding { kind: AlertKind, subject_key: String, title: String, detail: String, link: String }`; `fn can_dismiss(severity: Severity) -> bool` (só `!= Critical`).

- [x] **Step 1: Write the failing tests**
  - `migration_0012_creates_alert_tables`: tabelas existem, versão 12; dois `alerts` ativos com o mesmo `(kind, subject_key)` ⇒ erro; um ativo + um resolvido do mesmo par ⇒ ok; `DELETE FROM alerts` e `DELETE FROM alert_events` ⇒ erro com "imutável"; `status='x'` rejeitado.
  - `alert.rs`: `critical_kinds_cannot_be_dismissed` (`DepartureRisk`, `MaintenanceConflict`, `BackupMissing`, `DiskLow`, `UpdateFailed` ⇒ `can_dismiss` falso); `other_kinds_can_be_dismissed`; `kinds_roundtrip_through_str`; `only_payable_overdue_requires_a_permission`; `external_kinds_are_not_managed_by_detectors`; `severity_ranks_critical_first`.
- [x] **Step 2: Run** `cargo test migration_0012 domain::alert` — Expected: FAIL.
- [x] **Step 3: Implement** a migração (índices parciais e gatilhos `RAISE(ABORT, ...)` como o da auditoria em `0001`) e `domain/alert.rs`; registrar `Migration { version: 12, ... }`.
- [x] **Step 4: Run** `cargo test migration domain::alert` — Expected: PASS.
- [x] **Step 5: Commit** `feat: migration 0012 alerts and pure alert rules`

---

### Task 2: Motor de alertas (`AlertService`)

**Files:**
- Create: `src-tauri/src/models/alert.rs`, `repositories/alert_repo.rs`, `services/alert_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `app.rs`, `services/test_support.rs` (campo `alerts` no `Harness`)
- Test: `services/alert_service.rs`

**Interfaces:**
- Consumes: `domain::alert::{AlertKind, Severity, Finding, can_dismiss}` (T1), `Permission::ManagePayables`, `audit::record`.
- Produces:
  - `struct AlertView { id, kind, kind_label, severity, severity_label, title, detail, link, status, first_seen_at, last_seen_at, resolved_at: Option<String>, resolved_by_name: Option<String>, resolution_note: Option<String>, version }`; `struct AlertSummary { critical: i64, high: i64, normal: i64, total: i64, top: Vec<AlertView> }`; `struct AlertQuery { status: Option<String> /* active|history */, kind: Option<String>, severity: Option<String>, page: u32, page_size: u32 }`.
  - `AlertService::sync(&self, findings: Vec<Finding>, scope: &[AlertKind]) -> Result<(), AppError>` (**pública no crate**): em **uma transação**, para cada achado cria ou atualiza o alerta ativo do par `(kind, subject_key)` (evento `raised` ou `updated` só se título/detalhe/prioridade mudaram); para `dismissed` com causa presente marca `still_present=1` e **não** recria; para alertas ativos de `scope` sem achado, resolve (`resolved`, evento `resolved`, `resolved_at`); para dispensados de `scope` sem achado, `still_present=0`. Tipos fora de `scope` não são tocados.
  - `AlertService::refresh(&self, token: &str) -> Result<(), AppError>` (roda todos os detectores da T3/T4 e chama `sync` com `scope` = tipos gerenciados), `list(token, AlertQuery) -> Page<AlertView>`, `summary(token) -> AlertSummary` (contagens e os 5 mais urgentes ativos), `history(token, alert_id) -> Vec<AlertEventView>`, `dismiss(token, alert_id, note: String, expected_version: i64) -> Result<(), AppError>`.
  - Visibilidade: `list`, `summary` e `history` omitem tipos cuja `requires_permission()` o usuário não tem.

- [x] **Step 1: Write the failing tests** (com `Finding` sintéticos e o `Harness`; relógio com `FakeUtc`):
  - `sync_creates_an_alert_with_a_raised_event`; `sync_twice_with_the_same_finding_does_not_duplicate_alert_or_event`; `changed_detail_updates_the_alert_and_logs_one_updated_event`.
  - `alert_resolves_itself_when_the_cause_disappears` (status `resolved`, `resolved_at`, evento `resolved`, linha **preservada**); `cause_returning_creates_a_new_alert_and_keeps_the_old_one_as_history` (2 linhas).
  - `sync_never_touches_kinds_outside_its_scope` (alerta `backup_missing` ativo sobrevive a um `sync` de escopo gerenciado sem achados).
  - `critical_alert_cannot_be_dismissed` (`Validation`, estado inalterado); `dismiss_requires_a_note_up_to_200_chars` (vazio/espaços/201 ⇒ `Validation`); `dismissed_alert_does_not_come_back_while_the_cause_continues` (novo `sync` com o mesmo achado ⇒ nenhuma linha ativa, uma dispensada); `dismissed_then_cause_gone_then_back_creates_a_new_alert`.
  - `dismiss_is_audited_with_user_and_note` (`audit_log`: módulo `alerts`, operação `dismiss_alert`, justificativa) e `double_dismiss_conflicts` (segunda chamada com a versão antiga ⇒ `Conflict`; um só evento `dismissed`).
  - `no_path_deletes_alert_rows` (contagem de `alerts` e `alert_events` só cresce ao longo de uma sequência de operações).
  - `payable_overdue_is_invisible_without_manage_payables` (lista, resumo e histórico; recepção sem a permissão não vê nem conta; com a permissão concedida vê).
  - `summary_counts_by_severity_and_orders_critical_first`; `list_filters_by_status_kind_and_severity_and_pages`.
- [x] **Step 2: Run** `cargo test alert_service` — Expected: FAIL.
- [x] **Step 3: Implement** repositório (SQL à mão; sem `DELETE`) e serviço; `refresh` fica com o corpo `todo` até a T3 (apenas chama `detectors::collect` vazio por ora) — **não** implementar detectores aqui.
- [x] **Step 4: Run** `cargo test alert_service` e `cargo test` — Expected: PASS.
- [x] **Step 5: Commit** `feat: persistent alert engine with history and permission-aware views`

---

### Task 3: Detectores de reservas e hospedagens

**Files:**
- Create: `src-tauri/src/services/alert_detectors.rs`
- Modify: `services/mod.rs`, `services/alert_service.rs` (`refresh` passa a coletar), `services/stay_service.rs` (`assess_departure` → `pub(crate)`), `services/reservation_service.rs` (extrair a regra de "chegada vencida" para função `pub(crate)` reutilizável)
- Test: `services/alert_detectors.rs`

**Interfaces:**
- Consumes: `AlertService::sync` (T2), `reservation_repo::review_candidates`, `stay_repo`, `StayService::assess_departure`, configurações (`no_show_tolerance_minutes`, tolerância de saída).
- Produces: `pub(crate) fn collect_stay_findings(conn: &Connection, now_local: NaiveDateTime, settings: &AppSettings) -> Result<Vec<Finding>, AppError>` com os tipos `arrival_today`, `departure_today`, `arrival_overdue`, `deposit_overdue`, `departure_late`, `departure_risk`. `subject_key` = `reservation:<id>` ou `stay:<id>`; `link` = `/reservas/<id>` ou `/hospedagens/<id>`; `title` e `detail` em português claro (por exemplo "Chegada hoje: Ana Souza, quarto 101, previsto 14:00" / "Isso exige check-in até o fim do dia").

- [x] **Step 1: Write the failing tests** (cada um chama `refresh`, depois `list`):
  - `reservation_arriving_today_without_check_in_raises_arrival_today` e some após o check-in; cancelada não gera.
  - `stay_leaving_today_raises_departure_today` e some no check-out.
  - `arrival_at_23_30_local_is_still_today_even_though_it_is_next_day_in_utc` (relógio 02:30Z do dia seguinte = 23:30 local).
  - `next_day_without_check_in_turns_arrival_today_into_arrival_overdue` (o antigo fica `resolved`, nasce `arrival_overdue` depois da tolerância de no-show).
  - `unpaid_deposit_past_due_raises_deposit_overdue` e some ao receber o sinal; reserva já confirmada não gera.
  - `stay_past_departure_tolerance_raises_departure_late_with_minutes_in_the_detail`; dentro da tolerância não gera.
  - `late_departure_that_delays_the_next_reservation_raises_a_critical_departure_risk` (severidade `critical`, link da hospedagem).
  - `refresh_is_idempotent` (duas chamadas, mesmas linhas).
- [x] **Step 2: Run** `cargo test alert_detectors` — Expected: FAIL.
- [x] **Step 3: Implement** `collect_stay_findings` reutilizando os repositórios e a avaliação de saída; `refresh` chama `sync(findings, kinds_managed)`.
- [x] **Step 4: Run** `cargo test` — Expected: PASS (suíte inteira).
- [x] **Step 5: Commit** `feat: reservation and stay alert detectors`

---

### Task 4: Detectores de quartos e financeiro

**Files:**
- Modify: `src-tauri/src/services/alert_detectors.rs`, `services/alert_service.rs`
- Test: `services/alert_detectors.rs`

**Interfaces:**
- Consumes: `maintenance_repo::conflicts`, `cleaning_repo::queue` + `domain::cleaning::suggest_priority`, `payable_repo::list`, `closing_repo::pending_dates`, `stay_repo::balance`.
- Produces: `pub(crate) fn collect_room_findings(conn, now_local) -> Vec<Finding>` (`maintenance_conflict` por conflito aberto, `subject_key = conflict:<id>`, link `/manutencao/<order>`; `cleaning_urgent` por tarefa de limpeza aberta com prioridade `urgent` efetiva, link `/limpeza`) e `pub(crate) fn collect_finance_findings(conn, today, now_local) -> Vec<Finding>` (`payable_overdue` por conta vencida com saldo, `subject_key = payable:<id>`, link `/contas-a-pagar`; `checkout_balance` por hospedagem em andamento com saída hoje ou antes e saldo ≠ 0, link `/hospedagens/<id>`; `closing_pending` por dia sem fechamento, `subject_key = closing:<data>`, link `/fechamento/<data>`).

- [x] **Step 1: Write the failing tests:**
  - `open_maintenance_conflict_raises_a_critical_alert_and_resolves_when_the_reservation_is_resolved`.
  - `urgent_cleaning_raises_cleaning_urgent` (chegada em ≤ 4 h) e some ao concluir a limpeza; limpeza de prioridade normal não gera.
  - `overdue_payable_raises_payable_overdue_only_for_users_with_the_permission` e resolve ao pagar; vencimento hoje não gera.
  - `in_house_stay_leaving_today_with_balance_raises_checkout_balance` (valor no detalhe em `R$`) e resolve ao quitar; saldo zero não gera; hospedagem que sai amanhã não gera.
  - `day_with_movements_and_no_closing_raises_closing_pending` e resolve ao fechar (link com a data).
  - `refresh_runs_all_detectors_and_keeps_external_kinds` (alerta `backup_missing` criado à mão sobrevive).
- [x] **Step 2: Run** `cargo test alert_detectors` — Expected: FAIL.
- [x] **Step 3: Implement** os coletores e ligá-los em `refresh`.
- [x] **Step 4: Run** `cargo test` e `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: room and finance alert detectors`

---

### Task 5: Comandos Tauri e cliente da API (alertas)

**Files:**
- Create: `src-tauri/src/commands/alerts.rs`
- Modify: `commands/mod.rs`, `lib.rs`, `src/lib/api.ts`, `src/lib/api.test.ts`

**Interfaces:**
- Produces: comandos `refresh_alerts`, `list_alerts`, `alert_summary`, `alert_history`, `dismiss_alert`; TS: `refreshAlerts(token)`, `listAlerts(token, query)`, `alertSummary(token)`, `alertHistory(token, id)`, `dismissAlert(token, id, note, expectedVersion)` e os tipos `AlertView`, `AlertSummary`, `AlertQuery`, `AlertEvent`, `AlertSeverity`, `AlertKind`.

- [x] **Step 1: Write the failing test** `api.test.ts`: cada função chama o comando certo com argumentos em camelCase (`alertId`, `expectedVersion`).
- [x] **Step 2: Run** `npx vitest run src/lib/api.test.ts` — Expected: FAIL.
- [x] **Step 3: Implement** comandos finos e funções TS.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run src/lib/api.test.ts`, `npx tsc --noEmit` — Expected: PASS.
- [x] **Step 5: Commit** `feat: alert commands and api client`

---

### Task 6: Interface — painel de Alertas, menu e Recepção

**Files:**
- Create: `src/lib/alerts.ts`, `src/pages/alerts/{AlertsPage,AlertList,DismissDialog}.tsx` e os `*.test.tsx`
- Modify: `src/routes.tsx` (`/alertas`), `src/components/layout/{NavItems.ts,Sidebar.tsx}` (Alertas deixa de ser "Em breve"; contagem), `src/pages/Dashboard.tsx` (+ teste)

**Interfaces:**
- Consumes: API da T5, `Icon` (6.1), `usePermissions`.
- Produces: `severityLabel`/`severityIcon`/`severityClass` e `kindIcon` em `lib/alerts.ts` (`Crítico` ⇒ ícone `error`, `Alto` ⇒ `warning`, `Normal` ⇒ `info`; classes de token de 6.1); página `/alertas` com abas **Ativos** e **Histórico**, filtros por tipo e prioridade, lista ordenada por prioridade, cada item com título, detalhe, "visto desde", link **Abrir registro** e, quando não crítico, **Dispensar alerta** (diálogo com justificativa obrigatória); crítico mostra "Só se resolve quando o problema for resolvido". `Sidebar` mostra um selo com o total (e destaque textual "N críticos") no item **Alertas** (consulta `alertSummary` a cada 60 s e `refreshAlerts` ao montar). `Dashboard` (Recepção) mostra até 5 alertas ativos mais urgentes com link.

- [x] **Step 1: Write the failing tests:**
  - `AlertsPage`: lista por prioridade (crítico primeiro) com rótulo **e** ícone; filtro por tipo/prioridade chama a API com os parâmetros; aba Histórico mostra resolvidos/dispensados com quem, quando e a justificativa; item crítico **não** oferece `Dispensar alerta` e explica; `Dispensar alerta` exige justificativa (erro de campo, sem chamada) e, preenchida, chama `dismissAlert` com a versão e some da lista; erro do backend preserva o texto digitado; estado vazio explica ("Nenhum alerta ativo") e diz o que fazer.
  - `Sidebar`: o item Alertas é link para `/alertas` com selo "3" e texto acessível "3 alertas ativos, 1 crítico"; sem alertas, sem selo; falha na consulta não quebra o menu.
  - `Dashboard`: mostra os alertas mais urgentes com link para a origem; sem alertas, diz que está tudo em ordem.
  - `NavItems.test.ts`: Alertas tem `to: "/alertas"` e não é mais `soon`.
- [x] **Step 2: Run** `npx vitest run src/pages/alerts src/components/layout src/pages/Dashboard.test.tsx` — Expected: FAIL.
- [x] **Step 3: Implement** as telas e o selo; polling com `refetchInterval: 60_000`.
- [x] **Step 4: Run** `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: alerts panel, menu badge and reception highlights`

---

### Task 7: Busca global — serviço e comando

**Files:**
- Create: `src-tauri/src/models/search.rs`, `repositories/search_repo.rs`, `services/search_service.rs`, `commands/search.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `commands/mod.rs`, `lib.rs`, `app.rs`, `services/test_support.rs`, `src/lib/api.ts`, `src/lib/api.test.ts`
- Test: `services/search_service.rs`, `src/lib/api.test.ts`

**Interfaces:**
- Consumes: tabelas existentes; `Permission::ManagePayables`; `domain::text::fold`; máscara de CPF já usada nos modelos de hóspede.
- Produces: `SearchService::search(token, text: &str) -> Result<SearchResults, AppError>` com `SearchResults { query, guests: Group<GuestHit>, reservations: Group<ReservationHit>, stays: Group<StayHit>, movements: Group<MovementHit> }`, `Group<T> { items: Vec<T>, total: i64 }` (até 8 itens); `GuestHit { id, code, name, phone, cpf_masked, active }`, `ReservationHit { id, code, guest_name, room_code, check_in_date, check_out_date, status }`, `StayHit { id, code, guest_name, room_code, status }`, `MovementHit { kind: "payment"|"refund"|"payable", code, label, amount_cents, at, link }`. TS: `globalSearch(token, text)` e os tipos.
- Regras: texto aparado; **mínimo de 2 caracteres** (`Validation` "Digite pelo menos 2 letras ou números."); escape de `%`, `_` e `\` com `ESCAPE '\'`; comparação sem acento/caixa (`name_key`/`fold`); dígitos para telefone e CPF (`123.456.789-09` e `12345678909` casam); código em qualquer caixa; texto igual ao código do quarto casa reservas e hospedagens daquele quarto; resultado exato de código vem primeiro; movimentos: `PAG-`/`DEV-` por código ou nome do hóspede, `CAP-` e fornecedor **só com `ManagePayables`** (sem a permissão o grupo não traz contas a pagar nem as conta no total).

- [x] **Step 1: Write the failing tests:**
  - `finds_guest_by_name_without_accents_phone_and_masked_or_plain_cpf`.
  - `finds_reservation_and_stay_by_shared_code_in_any_case` (`res-000001`) e por código do quarto.
  - `finds_payment_and_refund_by_code_and_by_guest_name`; `payable_results_need_manage_payables` (recepção sem a permissão: grupo sem `CAP-` e total sem eles; com a permissão: aparece).
  - `percent_underscore_and_backslash_are_literals` (`%` não casa tudo; `a_b` não casa `axb`).
  - `one_character_is_refused_and_two_are_accepted`; `results_are_limited_to_eight_with_the_full_total`; `exact_code_match_comes_first`.
  - `search_over_five_years_of_data_stays_fast`: popula 5 anos fictícios por SQL direto numa transação (≈ 8 quartos × 365 × 5 reservas com hóspede, hospedagem e recebimento, ≈ 15 mil linhas) e afirma que uma busca por nome, uma por código e uma por CPF terminam cada uma em menos de 500 ms (build de teste).
  - `api.test.ts`: `globalSearch` chama `global_search` com `{ token, text }`.
- [x] **Step 2: Run** `cargo test search_service` e `npx vitest run src/lib/api.test.ts` — Expected: FAIL.
- [x] **Step 3: Implement** repositório (uma consulta por grupo, `LIMIT 8` e `count(*)`), serviço e comando `global_search`.
- [x] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS.
- [x] **Step 5: Commit** `feat: global search over guests, reservations, stays and movements`

---

### Task 8: Interface — resultados da busca agrupados

**Files:**
- Modify: `src/pages/Search.tsx`, `src/pages/Search.test.tsx`, `src/components/layout/GlobalSearch.tsx` (+ teste)

**Interfaces:**
- Consumes: `globalSearch` (T7).
- Produces: a página `/busca?q=` mostra quatro grupos (Hóspedes, Reservas, Hospedagens, Movimentos), cada um com título, contagem ("3 de 12"), itens com link para o registro (`/hospedes/:id`, `/reservas/:id`, `/hospedagens/:id`, movimento ⇒ `link`) e o texto do que casou; grupo sem resultado some; sem nenhum resultado, mensagem com próximos passos; texto curto mostra a orientação sem chamar o backend; `GlobalSearch` envia a busca com 2+ caracteres e mantém o texto ao navegar.

- [x] **Step 1: Write the failing tests:** agrupamento e contagens; links certos por tipo; movimento de conta a pagar não aparece quando a API não o devolve; "Mostrando 8 de 12" quando há mais; texto de 1 caractere ⇒ orientação e nenhuma chamada; estado vazio com sugestões; erro do backend mostra a mensagem e mantém o campo; `GlobalSearch` não navega com 1 caractere.
- [x] **Step 2: Run** `npx vitest run src/pages/Search.test.tsx src/components/layout` — Expected: FAIL.
- [x] **Step 3: Implement.**
- [x] **Step 4: Run** `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: grouped global search results`

---

### Task 9: Seed, documentação e encerramento

**Files:**
- Modify: `src-tauri/src/seed.rs` (+ teste), `docs/desenvolvimento/seed.md`, `CHANGELOG.md`, `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/README.md`, `local/plano-completo.md`, `local/spec.md` (Anexo A.6 com A1–A7), este plano
- Create: `docs/usuario/{alertas,busca}.md`, `docs/desenvolvimento/{alertas,busca}.md`

- [x] **Step 1: Seed:** o cenário existente já produz alertas (conta vencida, sinal, limpeza, conflito de manutenção, chegada/saída); `run` chama `app.alerts.refresh` ao final e o teste afirma que há ao menos um crítico (`maintenance_conflict`), um alto e um normal, e que a busca por "Ana" e por `PAG-` devolve resultados.
- [x] **Step 2: Docs:** guias de usuário (o que é cada alerta, como dispensar, por que crítico não dispensa, como abrir o registro; como buscar por nome, telefone, CPF, código, quarto; limites e permissões), desenvolvimento (detectores, motor, índices, decisões A1–A7, limitações: atraso de até 60 s, backup/disco/atualização só no Marco 7, sem FTS5, `payable_overdue` por permissão), `CHANGELOG.md` (`Added`), roteiro de treinamento (módulo de alertas e de busca; marcar ⚠ o que é provisório), Anexo A.6 da spec.
- [x] **Step 3: Verificação completa:** `cargo test`, `cargo test --features dev-seed`, `cargo clippy --all-targets --features dev-seed -- -D warnings`, `npx vitest run`, `npx tsc --noEmit`, `npm run build`; registrar contagens no Status.
- [x] **Step 4:** `graphify update .`; anotar no Status.
- [x] **Step 5:** Revisão final da branch por revisor independente (modelo mais capaz), com o Review Focus; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [x] **Step 6: Commit** `docs: close marco 6.2`

---

## Status

**Implementado e revisado** na branch `marco-6-2-alertas` (a partir de `marco-6-1-visual`). T1 a T9 concluídas; revisão final independente feita (abaixo). Migração `0012`.

- **Testes (última execução completa):** `cargo test --features dev-seed` 522 + 4 de integração; `cargo clippy --all-targets -- -D warnings` limpo com e sem `--features dev-seed`; `npx vitest run` 447 testes em 59 arquivos; `npx tsc --noEmit` limpo; `npm run build` ok; `graphify update .` executado.
- **Pendências de verificação (manuais, responsável):** `npm run tauri dev` com a interface real; o tempo real de `refresh` com meses de dados (hoje há N+1 consultas por hospedagem e por tarefa de limpeza, desprezíveis numa pousada, mas não medidas); se o WebView mantém o `refetchInterval` de 60 s com a janela minimizada.

## Desvios e decisões da execução

- `AlertService::refresh` ficou como esqueleto até a T3 (nada o chamava antes).
- `arrival_overdue` substitui `arrival_today` depois da tolerância de no-show; `departure_late` substitui `departure_today` depois da tolerância de saída; `closing_pending` nunca inclui hoje.
- O teste "código exato primeiro" virou "nome exato primeiro" (um código por `LIKE` contém não permite montar o caso com uma só reserva).
- A busca de contas a pagar compara fornecedor e descrição com o padrão cru (sem normalizar acentos).
- A Recepção deixou de mostrar os links "Fechamento pendente" (o alerta `closing_pending` os substitui); o seed ganhou uma chegada para hoje no quarto 201 e reavalia os alertas no fim.
- Slip de processo: os testes da correção da revisão foram escritos antes da implementação, mas o `cargo test` entre os dois não foi executado (a trava de build estava ocupada pela verificação em segundo plano); a regressão coberta é a de cada teste novo nomeado abaixo.

## Revisão final

Revisor independente (modelo mais capaz). Nenhum Crítico. Corrigidos em uma passada, cada um com teste:

- **Importante** — o texto de "Saída atrasada" muda a cada minuto (minutos de atraso) e cada rodada subia a `version`, então dispensar com a versão vista dava `Conflict` até recarregar, além de encher o histórico de eventos `updated`. Agora atualizar só o detalhe não muda a versão nem gera evento (`detail_only_changes_update_the_row_without_event_or_version_bump`, `dismissing_with_the_version_seen_still_works_after_a_text_only_refresh`).
- **Importante** — entre a meia-noite e o fim da tolerância, o alerta de chegada sumia (e ia ao histórico como "resolvido"). `arrival_today` agora vale para `data <= hoje` até a tolerância acabar (`late_arrival_alert_survives_midnight_until_the_tolerance_ends`).
- **Menores corrigidos (baratos):** dispensar alerta que já se resolveu diz "Este alerta já foi resolvido." (e o duplo clique idem); achado repetido na mesma rodada não derruba a rodada; crédito do hóspede na saída tem título próprio; `refresh` roda detectores e sincronização sob a mesma trava; a busca do topo navega com 1 caractere e a página explica a regra; a lista de alertas também se atualiza a cada 60 s.

**Menores adiados:** uma linha malformada derruba a rodada inteira dos detectores (`?` em vez de pular e registrar a linha); `last_seen_at` é gravado em toda rodada (relevante para o "mudou desde" do backup do Marco 7); N+1 consultas por hospedagem e por tarefa não medidas; a prioridade Alta de `checkout_balance` e `cleaning_urgent` é decisão de produto não confirmada.

## Marcos seguintes

- **6.3 — Recepção e visão gerencial:** Recepção como tela inicial (resumo, mapa de 7 dias, chegadas, saídas, risco, painel lateral, ações rápidas, alertas críticos) e área gerencial só para administrador.
- **6.4 — Relatórios e documentos:** 10 relatórios, CSV, PDFs e documentos internos versionados.
- **Marco 7:** emite os alertas `backup_missing`, `disk_low` e `update_failed` (tipos e prioridade já registrados aqui).

## Self-Review

- **Cobertura da spec §20:** pesquisa global de reservas, hóspedes, hospedagens e movimentos respeitando permissões → T7/T8; alertas persistentes até a resolução, link ao registro, painel central por prioridade e tipo, destaque na recepção, crítico não apagável e histórico → T1/T2/T6; tipos previstos (chegadas/saídas, reservas pendentes e sinais vencidos, conflitos de disponibilidade e limpeza, contas vencidas e saldos, fechamentos pendentes) → T3/T4; backup/disco/atualização → tipos registrados (A2), emissão no Marco 7. Filtros combinados "por módulo" ficam como estão nas listas de cada módulo.
- **Consistência de tipos:** `AlertKind`/`Finding` (T1) usados por T2–T4; `AlertView`/`AlertSummary` (T2) refletidos em T5/T6; `SearchResults` (T7) em T8.
- **Migração:** `0012` única; 6.4 não precisa de migração de alertas.
- **Proporção:** corpos de função ficam com o implementador; testes trazem valores exatos (tipos, prioridades, limites de 8 resultados, 2 caracteres, 200 caracteres, 500 ms).
