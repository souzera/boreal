# Limpeza e estado do quarto (Marco 4)

Plano: `docs/superpowers/plans/2026-10-27-marco-4-limpeza-manutencao.md`. Migração `0010`.

## Princípio

O estado do quarto é **calculado**, nunca gravado. `domain/room_state.rs::room_state(facts, interval, now)` recebe fatos (hóspede em casa, tarefa de limpeza mais recente, bloqueio efetivo hoje) e devolve `RoomState`. Precedência: ocupado → bloqueado → aguardando limpeza → em limpeza → limpo aguardando intervalo → disponível. O tempo só avança o último passo (intervalo depois de limpeza **concluída**); uma limpeza aberta nunca é encerrada pelo relógio.

## Dados

- `cleaning_tasks`: uma por hospedagem (`stay_id UNIQUE`), criada na **mesma transação** do check-out (`StayService::checkout`) com `ready_since` = hora efetiva da saída. Estados `awaiting → in_progress → done`; `version` em toda mudança; início e conclusão guardam usuário e hora.
- `rooms.cleaning_interval_minutes` (nulo = padrão de `settings.cleaning_interval_minutes`). Editável por `ManageCatalog` via `CatalogService::set_room_cleaning_interval` (0–1440), auditado.

## Serviços

- `CleaningService`: `queue`, `start`, `complete` (exige `in_progress`), `set_priority` (justificativa obrigatória; `None` volta à sugerida). Auditoria `module = "cleaning"`.
- Prioridade sugerida: `domain/cleaning.rs::suggest_priority(próxima_chegada, agora)` — urgente em até 4 h (`URGENT_WITHIN_HOURS`, provisório, Anexo A.4) ou já atrasada; alta no mesmo dia; normal depois; baixa sem reserva. Recalculada a cada leitura, o que cobre saída atrasada.
- `room_rules`: `cleaning_minutes` (intervalo efetivo), `ensure_room_clean` (portão do check-in/walk-in), `ensure_no_blocks`, `blocks_message`.
- `RoomStatusService` / `statuses()`: estado de cada quarto ativo; o mapa (`RoomRow.state/state_label/free_at`) reutiliza.

## Portão "quarto limpo" (fecha M3-10 / D-19)

Check-in e walk-in recusam o quarto com limpeza aberta ("aguardando limpeza" / "em limpeza") ou com `concluída_em + intervalo > agora`. Quarto sem histórico de limpeza está livre. A checagem fica dentro da transação.

## Decisões

M4-1 a M4-11 estão no plano. Rulings de execução: intervalo por quarto via comando próprio (não altera `UpdateRoom`); consultas por quarto em vez de lote (uma pousada tem dezenas de quartos); quarto desativado fica sem estado no mapa.

## Testes

Pura: matriz de estados, prioridade (4 h, virada de dia). Serviço: ciclo completo com relógio injetado, relógio avançando não libera quarto sujo, duplo clique (`Conflict`), tarefa única por hospedagem, check-out com falha não deixa tarefa, auditoria. Interface: fila, diálogo de prioridade, mapa com seis estados e legenda.

## Limitações

Alertas persistentes (limpeza urgente parada, saída atrasada) ficam no Marco 6. Enxoval fora do escopo.
