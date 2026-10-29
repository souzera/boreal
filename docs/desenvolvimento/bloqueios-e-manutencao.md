# Bloqueios e manutenção (Marco 4)

Plano: `docs/superpowers/plans/2026-10-27-marco-4-limpeza-manutencao.md`. Migração `0010`.

## Bloqueios (`room_blocks`)

- Período em noites: `start_date` inclusivo, `end_date` exclusivo; sem `end_date` = até liberação manual. Sem intervalo de limpeza nas bordas.
- `kind`: `manual | preventive | maintenance`. `status`: `preventive | active | awaiting_release | released`.
- Criação manual (`BlockService::create`, `ReleaseRoom`): motivo 1–500, início ≥ hoje, fim > início, quarto ativo. **Recusada** se há reserva ativa (pendente/confirmada/hospedada) tocando o período ou outro bloqueio não liberado (uma hospedagem em curso ocupa pelo menos a noite de hoje, mesmo atrasada). Para quarto com reservas, abre-se ordem de manutenção.
- **O bloqueio de uma ordem de manutenção não expira pela data prevista** (`maintenance_order_id` ignora `end_date` nas consultas): só a liberação o encerra. `resolve_conflict_manually` exige `ReleaseRoom`.
- Liberação (`release`, `ReleaseRoom`, justificativa): `Conflict` se a versão mudou ou já estava liberado; bloqueio de manutenção só libera com a ordem **concluída**.

## Integração com a disponibilidade

`room_rules::ensure_no_blocks` / `block_repo::overlapping(conn, room, start, end, effective_only)`:

- Reserva nova, alteração (vira `problem` na prévia), consulta de disponibilidade (`AvailabilityResult.blocks`) e walk-in: contam **todos** os bloqueios, inclusive preventivos.
- Check-in de reserva existente: só bloqueios **efetivos** (`active`/`awaiting_release`); o preventivo existe justamente porque há reservas afetadas e só impede novas (spec §8).
- Reserva que termina no dia em que o bloqueio começa passa.
- `check_availability` devolve também `not_ready` (limpeza pendente ou intervalo) para chegadas de hoje; a tela de hospedagem sem reserva o mostra e impede a confirmação.

## Ordens de manutenção

- `domain/maintenance.rs::transition`: `open → in_progress → awaiting_resolution → completed`; nada pula nem volta. Código `MAN-000001` (sequência `maintenance`).
- `MaintenanceService::open`: cria a ordem, um `maintenance_conflicts` por reserva afetada e o bloqueio (`preventive` se há conflito, senão `active`) numa transação.
- `apply_unavailability`: reavalia; conflito cuja reserva já não toca o período vira `resolved`; sem abertos, o bloqueio vira `active`. Idempotente. `resolve_conflict_manually` exige nota.
- `start` recusa com conflitos abertos; `complete` move o bloqueio para `awaiting_release` — **não libera** (spec §8).
- Permissões (M4-5): ordens abertas por qualquer usuário autenticado; bloqueios e liberação com `ReleaseRoom`. O backend valida; a interface só esconde botões.
- Auditoria `module = "maintenance"` / `"blocks"` na mesma transação.

## Testes

Backend: validações sem gravação parcial, reserva sobre bloqueio (manual, preventivo, aberto, fim = início), alteração e consulta, check-in/walk-in, liberação (permissão, justificativa, duplo clique), ordem com conflito (abrir, cancelar por fora, aplicar), conclusão não libera, máquina de estados, auditoria. Interface: listas, diálogos com validação e valores preservados, página da ordem.

## Limitações

Ordem não pode ser cancelada (a spec não prevê o estado). Prazos de alerta e relatórios: Marco 6.
