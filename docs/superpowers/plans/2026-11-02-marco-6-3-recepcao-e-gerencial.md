# Marco 6.3 — Recepção e visão gerencial Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer da tela inicial a mesa de trabalho da recepção ("o que exige atenção agora?"): resumo do dia, mapa de 7 dias, chegadas, saídas e quartos em risco com ações rápidas e um painel lateral que não esconde a agenda; e dar ao administrador uma visão gerencial (ocupação, recebimentos, saldos e contas) com gráficos.

**Architecture:** Dois serviços de **leitura** no backend: `ReceptionService::overview` (uma chamada compõe resumo, chegadas, saídas e riscos a partir dos serviços e repositórios existentes) e `ManagementService::dashboard` (consultas agregadas por período, só com `ViewFinancialReports`). No frontend, a grade do mapa vira o componente `RoomMapGrid` (usado pela página Mapa e pela Recepção), nasce um `SidePanel` não modal cuja seleção vive na URL, e a visão gerencial usa Recharts com uma tabela de dados equivalente para cada gráfico. As ações rápidas levam aos fluxos que já existem, abrindo o diálogo certo por `?abrir=`.

**Tech Stack:** Rust/rusqlite/SQLCipher, React/TS/TanStack Query/Recharts 3/Vitest. Sem dependências novas (Recharts já instalado).

**Spec:** `local/spec.md` §12.1 (Dashboard), §12.2 (Tela principal da recepção), §13 (permissões); `local/brief.md` §1, §3.1, §4, §7 ("Dashboard com gráficos antes das ações da recepção" é o que evitar). Roteiro: `local/plano-completo.md` §10, tarefas 6.2 (Dashboard) e 6.3 (Tela da recepção). Parte **6.3 de 4** do Marco 6.

**Branch:** `marco-6-3-recepcao`, a partir de `marco-6-2-alertas` (6.1 e 6.2 ainda não integradas). **Sem migração** de banco.

## Decisões deste plano (chamadas minhas; revisar na tela pronta)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| R1 | A Recepção é calculada **no backend, numa chamada** (`reception_overview`), reutilizando `StayService::list`, `ReservationService::list`, `RoomStatusService::list` e `AlertService`. Chegadas = reservas pendentes/confirmadas com chegada **hoje ou antes** (as atrasadas aparecem marcadas); saídas = hospedagens em andamento com saída **hoje ou antes**; em risco = alertas críticos ativos (`departure_risk` e `maintenance_conflict`). Cada lista traz até 50 itens e o total. | Uma ida ao backend por minuto e as mesmas regras dos outros módulos. Custo: se a lista passar de 50, a tela diz "Mostrando 50 de N" e manda para a lista completa. |
| R2 | O **painel lateral é não modal**: ocupa a coluna da direita (as listas dão lugar a ele; o **mapa continua visível**), fecha com o botão ou `Esc` e a seleção fica na URL (`?painel=reserva-5`, `hospedagem-3`, `hospede-7`, `quarto-2`), então recarregar ou voltar mantém o contexto. Conteúdo: resumo somente leitura + link para a página completa. | Brief §3.1: abre reserva, hóspede ou quarto sem ocultar a agenda. |
| R3 | **Ações rápidas não criam fluxo novo:** levam à tela do registro e abrem o diálogo certo com `?abrir=checkin` (reserva) e `?abrir=pagamento` / `?abrir=checkout` (hospedagem). O parâmetro abre **uma vez** e é removido; se a ação não cabe mais (reserva já em hospedagem, cancelada, hospedagem encerrada), mostra um aviso em vez de abrir. | Mantém as regras e permissões onde já estão. |
| R4 | Os atalhos de 6.1 mudam: `Alt+C` → `/?aba=chegadas`, `Alt+S` → `/?aba=saidas`; `Alt+R` continua em `/hospedagens`. | Cumpre a promessa de 6.1 ("a Recepção abre o fluxo direto"). |
| R5 | **Visão gerencial** é uma tela à parte, `/gerencial`, **só administrador** (`ViewFinancialReports`, não concedível), item "Visão gerencial" no menu só para administrador. **Período padrão: hoje** (spec §12.1), com atalhos Hoje / Últimos 7 dias / Este mês e datas livres; máximo de 366 dias. A Recepção não mostra gráficos (brief §7). | Roteiro 6.2 pede o dashboard "por permissão"; a recepção fica com números e listas, o gerente com gráficos. |
| R6 | **Métricas gerenciais:** ocupação por dia = noites de reservas ativas (`pending`, `confirmed`, `checked_in`, `completed`) ÷ quartos ativos hoje, em pontos-base; chegadas e saídas do período; hospedagens em andamento agora; recebimentos líquidos (recebido − devolvido) por forma de pagamento e por dia **local**; saldos pendentes (a receber e créditos) das hospedagens em andamento; contas a pagar vencidas e em aberto, por categoria. Valores em centavos. | Cobre a lista do roteiro com o que o sistema já grava. Limitação: o denominador usa os quartos ativos de hoje, não o histórico de ativação. |
| R7 | Cada **gráfico** (linha da ocupação, barras dos recebimentos, pizza das contas por categoria) vem com uma **tabela equivalente** num `<details>` ("Ver os números"); a cor nunca é a única informação (rótulos e valores na tabela e nas dicas). Cores dos tokens `--chart-1..5`. | Acessibilidade (brief §1, WCAG) e testabilidade em jsdom. |
| R8 | O mapa da Recepção é o **mesmo componente** da página Mapa de quartos (`RoomMapGrid`), com a janela fixa de 7 dias a partir de hoje; clicar numa reserva abre o painel (na Recepção) ou a reserva (no Mapa). | Uma só implementação da grade. |

## Global Constraints

- Estado antes de ação: cada item mostra a situação em **rótulo e ícone** (6.1); atraso, risco e saldo em texto (CLAUDE.md, brief §1).
- Datas `DD/MM/AAAA`, horas `HH:MM`, valores `R$ 150,00`; "hoje" é a data **local** (`ctx.today()`); relógios injetados.
- Permissões no backend: a visão gerencial exige `ViewFinancialReports` (administrador); a Recepção usa os dados que a recepção já pode ver; contas a pagar só entram com `ManagePayables`.
- Somente leitura: nenhuma operação nova grava dados nem muda regra de negócio (ações rápidas só navegam).
- Valores monetários em centavos inteiros; percentuais em pontos-base (1 casa decimal na tela).
- Contexto preservado: abrir um registro no painel não troca a agenda (brief §1); a seleção fica na URL.
- Gráficos só na área gerencial; a Recepção prioriza ações (brief §7).
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano e o grafo.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`.

## Review Focus

1. **Chegada atrasada e saída atrasada** aparecem nas listas, marcadas em texto, e não somem à meia-noite; reservas canceladas ou já em hospedagem nunca aparecem em Chegadas (T1).
2. **Painel lateral:** o mapa continua visível com o painel aberto; recarregar a página com `?painel=` reabre o mesmo registro; `Esc` fecha; id inexistente na URL mostra aviso, não quebra (T4, T5).
3. **`?abrir=`:** abre o diálogo uma única vez, é removido da URL (Voltar não reabre) e, quando a ação não cabe mais, avisa em vez de abrir; parâmetro desconhecido é ignorado (T4).
4. **Gerencial:** recepção recebe `Forbidden` no backend e não vê o item de menu; período invertido, vazio ou acima de 366 dias é recusado; **zero quartos ativos** não divide por zero (T2, T6).
5. **Números do período:** recebimentos líquidos por dia respeitam a data **local** (23:30 locais de ontem não cai em hoje); devolução maior que o recebido no dia dá dia líquido negativo e aparece assim (T2).

---

## File Structure

```
src-tauri/src/
  models/reception.rs, management.rs
  repositories/management_repo.rs                 # agregações do período; reservas pendentes de chegada
  services/reception_service.rs                   # overview (usa serviços e repositórios existentes)
  services/management_service.rs                  # dashboard (ViewFinancialReports)
  commands/reception.rs, management.rs
src/
  lib/api.ts                                      # tipos e chamadas
  components/RoomMapGrid.tsx (+ teste)            # grade extraída de RoomMapPage
  components/SidePanel.tsx (+ teste)              # painel lateral não modal
  hooks/useOpenFromQuery.ts (+ teste)             # ?abrir= : abre uma vez e limpa
  hooks/useReceptionOverview.ts
  pages/reception/ReceptionPage.tsx, SummaryStrip.tsx, ArrivalsList.tsx, DeparturesList.tsx, RiskList.tsx,
                  panel/{ReservationPanel,StayPanel,GuestPanel,RoomPanel}.tsx (+ testes)
  pages/management/ManagementPage.tsx, ChartCard.tsx (+ testes)
  pages/Dashboard.tsx                             # removido (substituído por ReceptionPage)
  pages/calendar/RoomMapPage.tsx                  # usa RoomMapGrid
  pages/reservations/ReservationPage.tsx, pages/stays/StayPage.tsx   # ?abrir=
  hooks/useGlobalShortcuts.ts, components/layout/{GlobalActions.tsx,NavItems.ts}, routes.tsx
docs/usuario/{recepcao,visao-gerencial}.md   docs/desenvolvimento/{recepcao,visao-gerencial}.md
```

---

### Task 1: `ReceptionService::overview`

**Files:**
- Create: `src-tauri/src/models/reception.rs`, `services/reception_service.rs`
- Modify: `models/mod.rs`, `services/mod.rs`, `app.rs`, `services/test_support.rs` (campo `reception` no `Harness`), `repositories/reservation_repo.rs` (`pending_arrivals`)
- Test: `services/reception_service.rs`

**Interfaces:**
- Consumes: `StayService::list(token, StayQuery)` (em andamento, com `balance_cents`, `minutes_late`, `risk`), `RoomStatusService::list(token)`, `AlertService::{list, summary}` (6.2), `reservation_repo`, `settings_repo` (tolerância de no-show), `ctx.today()`, `ctx.local_now()`.
- Produces:
  - `struct ReceptionSummary { arrivals: i64, departures: i64, occupied_rooms: i64, available_rooms: i64, total_rooms: i64, critical_alerts: i64 }`;
  - `struct ArrivalItem { reservation_id, code, guest_name, room_code, check_in_date, expected_time, status, overdue: bool, deposit_overdue: bool }`;
  - `struct DepartureItem { stay_id, code, guest_name, room_code, check_out_date, expected_time, balance_cents: i64, minutes_late: i64 }`;
  - `struct RiskItem { kind: String, title: String, detail: String, link: String }`;
  - `struct Group<T> { items: Vec<T>, total: i64 }` (reaproveitar `models::search::Group` se já cobrir; senão mover para `models::page`);
  - `struct ReceptionOverview { date: String, summary: ReceptionSummary, arrivals: Group<ArrivalItem>, departures: Group<DepartureItem>, at_risk: Group<RiskItem> }`;
  - `ReceptionService::overview(&self, token: &str) -> Result<ReceptionOverview, AppError>` (até 50 itens por lista).
  - `reservation_repo::pending_arrivals(conn, today) -> Vec<Reservation>`: `status IN ('pending','confirmed')` e `check_in_date <= today`, por data e horário.
- Regras: `overdue` = data anterior a hoje **ou** horário previsto + tolerância de no-show já passou; `available_rooms` e `occupied_rooms` contam o estado calculado de `RoomStatusService` (só quartos ativos); `at_risk` vem dos alertas críticos ativos (tipos `departure_risk` e `maintenance_conflict`); `critical_alerts` = `AlertService::summary().critical`.

- [ ] **Step 1: Write the failing tests** (`Harness` com um quarto 101; relógio em 21/10/2026 09:00 local):
  - `counts_and_lists_arrivals_today_and_marks_overdue_ones` (hoje às 14:00 pendente; outra reserva de ontem sem check-in marcada `overdue`; `summary.arrivals == 2`).
  - `cancelled_and_checked_in_reservations_are_not_arrivals`.
  - `lists_departures_today_with_balance_and_minutes_late` (hospedagem com saída às 12:00, relógio 13:00 ⇒ `minutes_late == 60`, `balance_cents` da conta).
  - `departure_of_a_past_date_still_shows_as_late`.
  - `rooms_count_uses_the_calculated_state` (um quarto ocupado, um disponível, um bloqueado ⇒ `occupied_rooms == 1`, `available_rooms == 1`, `total_rooms == 3`; quarto inativo fora).
  - `at_risk_lists_only_critical_alerts` (conflito de manutenção aparece; chegada atrasada não) e `critical_alerts_matches_the_alert_summary`.
  - `lists_are_limited_to_fifty_with_the_full_total` (55 reservas pendentes de hoje por SQL ⇒ 50 itens, `total == 55`).
  - `requires_a_session`.
- [ ] **Step 2: Run** `cargo test reception_service` — Expected: FAIL.
- [ ] **Step 3: Implement** modelos, `pending_arrivals` e o serviço; ligar em `app.rs` e no `Harness`.
- [ ] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [ ] **Step 5: Commit** `feat: reception overview service`

---

### Task 2: `ManagementService::dashboard`

**Files:**
- Create: `src-tauri/src/models/management.rs`, `repositories/management_repo.rs`, `services/management_service.rs`
- Modify: `models/mod.rs`, `repositories/mod.rs`, `services/mod.rs`, `app.rs`, `services/test_support.rs` (campo `management`)
- Test: `services/management_service.rs`

**Interfaces:**
- Consumes: `Permission::ViewFinancialReports`, `domain::dates::utc_bounds`, `closing_repo::{received, refunded}` (por forma de pagamento), `payable_repo`, `stay_repo::balance`, `domain::payment::Method`.
- Produces:
  - `struct DayOccupancy { date: String, occupied: i64, rooms: i64, rate_bp: i64 }`;
  - `struct MethodNet { method: String, label: String, received_cents: i64, refunded_cents: i64, net_cents: i64 }`;
  - `struct DayNet { date: String, net_cents: i64 }`;
  - `struct BalanceSummary { to_receive_count: i64, to_receive_cents: i64, credit_count: i64, credit_cents: i64 }`;
  - `struct CategoryOpen { category_name: String, open_cents: i64 }`;
  - `struct PayablesSummary { overdue_count: i64, overdue_cents: i64, open_cents: i64, by_category: Vec<CategoryOpen> }`;
  - `struct ManagementDashboard { from, to, rooms_total: i64, occupancy: Vec<DayOccupancy>, arrivals: i64, departures: i64, in_house_now: i64, receipts_by_method: Vec<MethodNet>, receipts_daily: Vec<DayNet>, balances: BalanceSummary, payables: PayablesSummary }`;
  - `ManagementService::dashboard(&self, token: &str, from: &str, to: &str) -> Result<ManagementDashboard, AppError>`.
- Regras: exige `ViewFinancialReports` (`Forbidden` para recepção); `from ≤ to`, no máximo 366 dias (`Validation`); `rate_bp = occupied × 10000 ÷ rooms` (0 se `rooms == 0`), arredondado ao ponto-base mais próximo; recebimentos líquidos por forma (as cinco + `unspecified` quando houver) e por dia **local** (`date(received_at, <deslocamento>)`), com dias sem movimento presentes com zero; saldos e contas são da **situação atual**, não do período; contas em aberto por categoria só com saldo > 0.

- [ ] **Step 1: Write the failing tests:**
  - `reception_cannot_open_the_dashboard` (`Forbidden`) e `invalid_periods_are_refused` (invertido, > 366 dias, data inválida).
  - `occupancy_counts_active_reservation_nights_and_ignores_cancelled` (2 noites de uma reserva, 1 de uma cancelada ⇒ taxa 10000 pb num quarto; `rooms == total de quartos ativos`).
  - `occupancy_with_no_active_rooms_is_zero_not_a_division_error`.
  - `arrivals_departures_and_in_house_count_the_period`.
  - `receipts_are_net_of_refunds_by_method_and_by_local_day` (Pix 10.000 hoje 23:30 local = 02:30Z do dia seguinte entra no dia local anterior; devolução de 2.000 em dinheiro ⇒ dinheiro líquido −2.000 em um dia sem recebimento).
  - `days_without_movement_are_present_with_zero`.
  - `balances_sum_in_house_stays_to_receive_and_credit_separately`.
  - `payables_summary_has_overdue_open_and_by_category` (paga integralmente não entra; vencimento hoje não é vencida).
  - `default_one_day_period_returns_a_single_day`.
- [ ] **Step 2: Run** `cargo test management_service` — Expected: FAIL.
- [ ] **Step 3: Implement** as consultas agregadas (SQL à mão) e o serviço.
- [ ] **Step 4: Run** `cargo test` e `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [ ] **Step 5: Commit** `feat: management dashboard service`

---

### Task 3: Comandos Tauri e cliente da API

**Files:**
- Create: `src-tauri/src/commands/reception.rs`, `commands/management.rs`
- Modify: `commands/mod.rs`, `lib.rs`, `src/lib/api.ts`, `src/lib/api.test.ts`

**Interfaces:**
- Produces: comandos `reception_overview`, `management_dashboard`; TS: `receptionOverview(token)`, `managementDashboard(token, from, to)` e os tipos `ReceptionOverview`, `ReceptionSummary`, `ArrivalItem`, `DepartureItem`, `RiskItem`, `ManagementDashboard`, `DayOccupancy`, `MethodNet`, `DayNet`, `BalanceSummary`, `PayablesSummary`, `CategoryOpen` (espelham os modelos da T1/T2).

- [ ] **Step 1: Write the failing test** `api.test.ts`: cada função chama o comando certo (`{ token }` e `{ token, from, to }`).
- [ ] **Step 2: Run** `npx vitest run src/lib/api.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** comandos finos e funções TS.
- [ ] **Step 4: Run** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run src/lib/api.test.ts`, `npx tsc --noEmit` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: reception and management commands`

---

### Task 4: Componentes de base — grade do mapa, painel lateral e `?abrir=`

**Files:**
- Create: `src/components/RoomMapGrid.tsx` (+ `.test.tsx`), `src/components/SidePanel.tsx` (+ `.test.tsx`), `src/hooks/useOpenFromQuery.ts` (+ `.test.tsx`), `src/pages/reception/panel/{ReservationPanel,StayPanel,GuestPanel,RoomPanel}.tsx` (+ testes)
- Modify: `src/pages/calendar/RoomMapPage.tsx` (usa a grade), `src/pages/reservations/ReservationPage.tsx`, `src/pages/stays/StayPage.tsx`

**Interfaces:**
- Produces:
  - `RoomMapGrid({ data: RoomMap, from: string, days: number, today: string, onOpenReservation: (id: number) => void, onBook?: (roomId: number, date: string) => void, onOpenRoom?: (roomId: number) => void })` — a mesma tabela quarto × dia da página Mapa (células com situação em ícone e texto, "Livre" reservável, `RoomStateBadge` por quarto).
  - `SidePanel({ title, onClose, children })`: `<aside aria-label={title}>` com botão **Fechar painel**; `Esc` com o foco dentro fecha; não é modal (não usa `role="dialog"`, não prende o foco, não bloqueia a rolagem).
  - `useOpenFromQuery<T extends string>(allowed: readonly T[], onOpen: (action: T) => boolean): void`: lê `?abrir=`, ignora valor fora de `allowed`, chama `onOpen` **uma vez** quando a página tem os dados e apaga o parâmetro (`replace: true`); `onOpen` devolve `false` quando a ação não cabe (a página mostra o aviso).
  - Painéis (cada um recebe `id` e usa `getReservation` / `getStay` / `getGuest` / `listRoomStatus`): resumo somente leitura com situação em ícone e texto, valores em `R$` e o link **Abrir reserva completa** / **Abrir hospedagem completa** / **Abrir cadastro completo** / **Ver na fila de limpeza**; `ReservationPanel` oferece **Confirmar check-in** (`/reservas/:id?abrir=checkin`) só para reserva aberta com chegada até hoje; `StayPanel` oferece **Registrar pagamento** (`?abrir=pagamento`) e **Concluir check-out** (`?abrir=checkout`) só para hospedagem em andamento. Id inexistente ⇒ aviso "Não foi possível carregar este registro."
  - `ReservationPage` aceita `?abrir=checkin|pagamento`; `StayPage` aceita `?abrir=pagamento|checkout`.

- [ ] **Step 1: Write the failing tests:**
  - `RoomMapGrid.test.tsx`: renderiza quartos e dias; reserva mostra hóspede, situação em ícone e texto e chama `onOpenReservation`; "Livre" só em dia de hoje em diante e chama `onBook`; quarto inativo marcado "(inativo)"; sem `onBook` não oferece "Livre". `RoomMapPage.test.tsx` existente segue verde (clicar numa reserva navega).
  - `SidePanel.test.tsx`: tem `aria-label`, **não** tem `role="dialog"`; botão Fechar chama `onClose`; `Esc` dentro dele fecha; `Esc` fora não; o conteúdo atrás continua acessível (não há `inert`/`aria-hidden` nos irmãos).
  - `useOpenFromQuery.test.tsx`: abre uma vez e limpa a URL (Voltar não reabre); valor desconhecido é ignorado e limpo; `onOpen` devolvendo `false` não abre; não abre antes de os dados existirem.
  - Painéis: cada um mostra o resumo e os links certos; ações condicionais (check-in só para reserva aberta com chegada ≤ hoje; pagamento/check-out só para hospedagem em andamento); erro mostra o aviso.
  - `ReservationPage`/`StayPage`: `?abrir=checkin` abre o diálogo "Confirmar check-in"; em reserva cancelada mostra "Esta ação não está mais disponível para esta reserva." e não abre; `?abrir=pagamento` abre o diálogo de pagamento da hospedagem.
- [ ] **Step 2: Run** `npx vitest run src/components src/hooks src/pages/reception src/pages/reservations src/pages/stays src/pages/calendar` — Expected: FAIL.
- [ ] **Step 3: Implement** (extrair a tabela de `RoomMapPage` sem mudar o comportamento; `SidePanel` com `onKeyDown`; `useOpenFromQuery` com `useSearchParams`).
- [ ] **Step 4: Run** a suíte inteira, `npx tsc --noEmit` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: shared room map grid, side panel and open-from-query`

---

### Task 5: Tela da Recepção

**Files:**
- Create: `src/hooks/useReceptionOverview.ts`, `src/pages/reception/{ReceptionPage,SummaryStrip,ArrivalsList,DeparturesList,RiskList}.tsx` e testes
- Modify: `src/routes.tsx` (`/` aponta para `ReceptionPage`), `src/hooks/useGlobalShortcuts.ts` (`SHORTCUT_DESTINATIONS.checkIn = "/?aba=chegadas"`, `checkOut = "/?aba=saidas"`), `src/components/layout/GlobalActions.tsx` (compara `pathname + search`), seus testes; **remover** `src/pages/Dashboard.tsx` e `Dashboard.test.tsx`

**Interfaces:**
- Consumes: `receptionOverview` (T3), `roomMap` (existente), `useAlertSummary` (6.2), `RoomMapGrid`, `SidePanel` e painéis (T4).
- Produces: `useReceptionOverview()` (`queryKey ["reception-overview"]`, refaz a cada 60 s com a sessão ativa, sem repetir tentativa). `ReceptionPage` com: título **Recepção** e "Hoje, DD/MM/AAAA"; faixa de resumo (cartões **Chegadas**, **Saídas**, **Quartos ocupados** "N de T", **Quartos disponíveis**, **Alertas críticos**; Chegadas e Saídas selecionam a aba, Alertas críticos leva a `/alertas?prioridade=critical`); à esquerda o **mapa de 7 dias** a partir de hoje (clicar numa reserva abre o painel); à direita abas **Chegadas · Saídas · Em risco** (`?aba=`) ou, com `?painel=` válido, o painel (o mapa permanece); abaixo, **Alertas urgentes** (até 3, do resumo) e o atalho **Quartos aguardando limpeza: N** (fila de limpeza). Linhas: chegada com **Confirmar check-in** (`/reservas/:id?abrir=checkin`) e **Ver detalhes** (painel), marcada "Atrasada" em texto e ícone quando `overdue`, "Sinal vencido" quando `deposit_overdue`; saída com saldo em palavras (`balanceLabel`), "Atrasada há N min" quando `minutes_late > 0`, **Registrar pagamento** só com saldo > 0 e **Concluir check-out**; risco com **Abrir registro**. Estados vazios explicam ("Nenhuma chegada pendente para hoje.").

- [ ] **Step 1: Write the failing tests** (API simulada):
  - `ReceptionPage`: mostra data e os cinco cartões com os números; clicar em **Saídas** seleciona a aba; `?aba=chegadas` abre Chegadas e `?aba=saidas`, Saídas; aba inválida cai em Chegadas.
  - Chegadas: linha atrasada com texto "Atrasada" e ícone; **Confirmar check-in** aponta para `/reservas/5?abrir=checkin`; **Ver detalhes** define `?painel=reserva-5` e mostra o painel **com o mapa ainda na tela**; fechar o painel remove o parâmetro e devolve a lista na mesma aba.
  - Saídas: saldo em palavras ("Saldo a receber R$ 80,00"), **Registrar pagamento** só quando há saldo, **Concluir check-out** sempre, "Atrasada há 60 min" em texto.
  - Em risco: cada item com **Abrir registro** e prioridade crítica em ícone e texto; vazio explica.
  - `?painel=reserva-999` (inexistente) mostra o aviso do painel e não quebra a tela; `?painel=lixo` é ignorado.
  - Recarregar com `?painel=hospedagem-3` reabre o painel da hospedagem.
  - Alertas urgentes e fila de limpeza continuam presentes (as asserções do antigo `Dashboard.test.tsx` migram para cá).
  - Atalhos: `Alt+C` navega para `/?aba=chegadas` e `Alt+S` para `/?aba=saidas`; repetir na mesma URL não empilha histórico; `useGlobalShortcuts.test.tsx` e `GlobalActions.test.tsx` atualizados para os novos destinos.
  - Falha de rede/erro do backend mostra "Não foi possível carregar a Recepção. Tente de novo em instantes." sem apagar o que já estava na tela.
- [ ] **Step 2: Run** `npx vitest run src/pages/reception src/hooks src/components/layout` — Expected: FAIL.
- [ ] **Step 3: Implement** as telas e a troca de rota.
- [ ] **Step 4: Run** `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: reception home screen with side panel and quick actions`

---

### Task 6: Visão gerencial com gráficos

**Files:**
- Create: `src/pages/management/{ManagementPage,ChartCard}.tsx` e testes
- Modify: `src/routes.tsx` (`/gerencial`), `src/components/layout/NavItems.ts` (item **Visão gerencial**, `admin: true`, ícone `monitoring`, depois de Relatórios), `NavItems.test.ts`, `Sidebar.test.tsx`

**Interfaces:**
- Consumes: `managementDashboard(token, from, to)` (T3), tokens de gráfico `--chart-1..5` (6.1), Recharts (`LineChart`, `BarChart`, `PieChart`, `ResponsiveContainer`).
- Produces: `ChartCard({ title, description, children, table })`: cartão com o gráfico (`role="img"` e `aria-label` com a descrição) e `<details>` **Ver os números** contendo a tabela equivalente. `ManagementPage`: seletor de período (**Hoje**, **Últimos 7 dias**, **Este mês**, datas livres; padrão Hoje); cartões de números (Chegadas, Saídas, Hospedados agora, Recebido no período, Saldos a receber, Créditos, Contas vencidas); gráficos **Ocupação por dia** (linha, %), **Recebimentos por dia** (barras, líquido em R$) e **Contas em aberto por categoria** (pizza); tabela de recebimentos por forma de pagamento (recebido, devolvido, líquido; débito e crédito separados). Quem não é administrador vê "Esta área é só para administradores." (a rota existe, o backend recusa).

- [ ] **Step 1: Write the failing tests** (Recharts substituído por componentes simples via `vi.mock("recharts")` para jsdom):
  - Período padrão é hoje e chama `managementDashboard(token, hoje, hoje)`; **Últimos 7 dias** e **Este mês** recalculam as datas e consultam de novo; data final antes da inicial mostra o erro "A data final não pode ser antes da inicial." sem chamar a API.
  - Cartões mostram os números em `R$` e em palavras; recebido do período é a soma dos líquidos.
  - Cada gráfico tem `role="img"` com descrição e, em **Ver os números**, a tabela com os mesmos valores (ocupação em %, recebimento líquido por dia incluindo dia negativo, categoria e valor).
  - Tabela por forma de pagamento mostra débito e crédito separados e a linha "Sem modalidade" só quando existe.
  - Período sem movimento mostra "Sem movimento no período." nos gráficos, em vez de um gráfico vazio.
  - Erro `Forbidden` do backend mostra a mensagem; usuário recepção não vê o item de menu, administrador vê; `NavItems.test.ts` (destino `/gerencial` existe nas rotas, recepção não o vê).
- [ ] **Step 2: Run** `npx vitest run src/pages/management src/components/layout` — Expected: FAIL.
- [ ] **Step 3: Implement** a página, o cartão de gráfico e o item de menu.
- [ ] **Step 4: Run** `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [ ] **Step 5: Commit** `feat: management dashboard with charts`

---

### Task 7: Seed, documentação e encerramento

**Files:**
- Modify: `src-tauri/src/seed.rs` (+ teste), `docs/desenvolvimento/seed.md`, `CHANGELOG.md`, `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/README.md`, `docs/desenvolvimento/design-system.md` (Recepção como tela inicial; uso do painel lateral), `local/plano-completo.md`, `local/spec.md` (Anexo A.7 com R1–R8), este plano
- Create: `docs/usuario/{recepcao,visao-gerencial}.md`, `docs/desenvolvimento/{recepcao,visao-gerencial}.md`

- [ ] **Step 1: Seed:** o cenário existente já preenche a Recepção; o teste afirma que `reception.overview` devolve chegadas, saídas (a hospedagem do 101) e ao menos um item em risco, e que `management.dashboard` (admin) devolve ocupação, recebimentos do dia e contas vencidas coerentes com o seed.
- [ ] **Step 2: Docs:** guia da Recepção (resumo, mapa, abas, painel lateral, ações rápidas, atalhos `Alt+C`/`Alt+S`), guia da visão gerencial (período, números, gráficos e tabelas, quem acessa), desenvolvimento (serviços de leitura, composição, regras de ocupação e de dia local, decisões R1–R8, limitações: denominador de ocupação pelos quartos ativos de hoje, lista de 50 itens), `CHANGELOG.md` (`Added`/`Changed`: a tela inicial deixou de ser o Dashboard simples), roteiro de treinamento (módulo da Recepção e da visão gerencial; o Módulo 0 passa a mostrar a Recepção), Anexo A.7 da spec.
- [ ] **Step 3: Verificação completa:** `cargo test --features dev-seed`, `cargo clippy --all-targets --features dev-seed -- -D warnings`, `npx vitest run`, `npx tsc --noEmit`, `npm run build`; registrar contagens no Status.
- [ ] **Step 4:** `graphify update .`; anotar no Status.
- [ ] **Step 5:** Revisão final da branch por revisor independente (modelo mais capaz), com o Review Focus; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [ ] **Step 6: Commit** `docs: close marco 6.3`

---

## Status

**Não iniciado.** Plano escrito em 02/11/2026.

## Marcos seguintes

- **6.4 — Relatórios e documentos:** 10 relatórios com filtros e conciliação, CSV, PDFs e documentos internos versionados; item **Relatórios** do menu.
- **Marco 7:** backup, recuperação, atualização e segurança operacional (emite os alertas `backup_missing`, `disk_low` e `update_failed`).

## Self-Review

- **Cobertura do roteiro/spec:** §12.2 tela principal da recepção (mapa de 7 dias, situação, chegadas, saídas, bloqueios e conflitos destacados, ações rápidas) → T1/T4/T5; §12.1 dashboard por permissão (ocupação, ocupados/disponíveis, chegadas/saídas, hospedagens em andamento, recebimentos, saldos, contas a pagar/vencidas, alertas; período padrão hoje; barras, linhas e setores) → T2/T6, com alertas e ocupados/disponíveis também na Recepção; brief §3.1 (faixa de resumo, painel lateral que preserva a agenda, abas complementares) → T4/T5; "Dashboard com gráficos antes das ações" evitado (R5). Bloqueios aparecem na grade pelo estado "Em manutenção ou bloqueado" do quarto (já existente) e conflitos em "Em risco".
- **Consistência de tipos:** `ReceptionOverview`/`Group` (T1) e `ManagementDashboard` (T2) refletidos em T3; `RoomMapGrid`, `SidePanel`, `useOpenFromQuery` (T4) usados por T5; destinos de atalho (T5) conferidos nos testes de T4/T5.
- **Migração:** nenhuma.
- **Proporção:** corpos de função ficam com o implementador; os testes trazem valores exatos (50 itens, 60 min, 366 dias, 02:30Z/23:30 local, R$ 80,00).
