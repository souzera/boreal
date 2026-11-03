# Alertas (Marco 6.2)

Alertas persistentes, acionáveis e com histórico (spec §20.2). Plano: `docs/superpowers/plans/2026-10-31-marco-6-2-alertas-e-busca.md`; decisões A1 a A7 e Anexo A.6 da spec.

## Modelo (migração `0012`)

- `alerts(kind, subject_key, severity, title, detail, link, status 'active'|'resolved'|'dismissed', still_present, first_seen_at, last_seen_at, resolved_at, resolved_by, resolution_note, version)`.
  - Índice único parcial `(kind, subject_key) WHERE status = 'active'`: **um alerta ativo por causa**.
  - Índice único parcial `(kind, subject_key) WHERE status = 'dismissed' AND still_present = 1`: **um dispensado por causa enquanto ela continua**.
- `alert_events(alert_id, at, user_id, event 'raised'|'updated'|'resolved'|'dismissed', note)`.
- Gatilhos `BEFORE DELETE` nas duas tabelas (`RAISE(ABORT, 'histórico de alertas é imutável')`): **nada é apagado**, nem por engano.

## Motor (`AlertService`)

- `sync(findings, scope)` (numa transação): para cada achado cria o alerta (evento `raised`), atualiza o texto se mudou — **sem** mexer em `version` e **sem** evento quando só o detalhe muda (os minutos de atraso mudam a cada rodada e não podem invalidar uma dispensa em andamento nem encher o histórico); título, link ou prioridade novos geram o evento `updated`; senão só renova `last_seen_at`. Um achado repetido na mesma rodada é ignorado; um dispensado com a causa presente não é recriado; para os tipos do `scope` sem achado, os **ativos** viram `resolved` (evento `resolved`) e os **dispensados** ganham `still_present = 0` (se a causa voltar, nasce um alerta novo). Tipos fora do `scope` (backup, disco, atualização) nunca são tocados.
- `refresh(token)`: sob a mesma trava da conexão, carrega as configurações, roda `alert_detectors::collect_all` e sincroniza com os tipos gerenciados por detectores (duas rodadas nunca se misturam). `arrival_today` continua depois da meia-noite até acabar a tolerância de no-show, para o alerta não sumir no intervalo.
- `list`, `summary` (contagens por prioridade e os 5 mais urgentes), `history` e `dismiss`. `dismiss`: justificativa de 1 a 200 caracteres, recusa alerta que já deixou de ser ativo ("Este alerta já foi resolvido.", inclusive no duplo clique), `Conflict` por versão, recusa alerta **crítico** (`domain::alert::can_dismiss`), audita em `audit_log` (módulo `alerts`, operação `dismiss_alert`).
- **Permissão por tipo:** `AlertKind::requires_permission` (hoje só `payable_overdue` ⇒ `ManagePayables`). Quem não tem a permissão não vê o tipo na lista, no resumo nem no histórico, e a contagem do menu o ignora.

## Detectores (`services/alert_detectors.rs`)

Somente leitura; reutilizam repositórios e regras já existentes.

| Detector | Tipos | Base |
|---|---|---|
| `collect_stay_findings` | `arrival_today`, `arrival_overdue`, `deposit_overdue`, `departure_today`, `departure_late`, `departure_risk` | `reservation_repo::review_candidates`, `stay_repo::list` e `StayService::assess_departure` (tolerâncias e risco). `arrival_overdue` substitui `arrival_today` depois da tolerância de no-show; `departure_late` substitui `departure_today` depois da tolerância de saída. |
| `collect_room_findings` | `maintenance_conflict`, `cleaning_urgent` | conflitos abertos de manutenção; fila de limpeza com a prioridade efetiva (ajuste manual ou sugerida) urgente. |
| `collect_finance_findings` | `payable_overdue`, `checkout_balance`, `closing_pending` | contas vencidas com saldo; hospedagens em andamento com saída hoje ou antes e saldo ≠ 0; `closing_repo::pending_dates` (dias anteriores a hoje). |

"Hoje" é sempre a data **local** (`ctx.today()`). `subject_key`: `reservation:<id>`, `stay:<id>`, `conflict:<id>`, `cleaning:<id>`, `payable:<id>`, `closing:<data>`.

## Interface

`/alertas` (abas Ativos/Histórico, filtros por tipo e prioridade, dispensa com justificativa); o hook `useAlertSummary` reavalia e consulta a cada 60 s enquanto a sessão está ativa e alimenta o selo do menu e a tela Recepção. Prioridade sempre com ícone e rótulo.

## Limitações conhecidas

- Atraso de até 60 s entre o fato e o alerta (decisão A1: sem ganchos em cada operação).
- `backup_missing`, `disk_low` e `update_failed` existem como tipos críticos, mas sem emissor: o Marco 7 os alimenta chamando `sync` com `scope` vazio.
- A busca por texto das contas usa `LIKE` sem normalizar acentos.

## Testes

`alert_service` (motor, dispensa, histórico, permissão), `alert_detectors` (cada tipo, virada de dia e fuso, idempotência) e os domínios `alert`. No front: `AlertsPage`, `Sidebar`, `Dashboard`, `lib/alerts`.
