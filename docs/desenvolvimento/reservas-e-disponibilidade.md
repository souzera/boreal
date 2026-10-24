# Reservas e disponibilidade (Marco 2)

## Tabelas (migração `0008`)

`reservations` (valores, sinal, prazo, `version` para concorrência otimista), `reservation_nights` (preço congelado por noite) e `reservation_changes` (histórico: antes/depois em JSON, preço antes/depois, justificativa, usuário, data).

## Estados e transições

`pending → confirmed` (sinal atingiu o esperado, via `record_deposit`, ou confirmação manual) · `pending|confirmed → cancelled` · `pending|confirmed → no_show` (manual, só depois da chegada). `pending|confirmed → checked_in` (check-in) e `checked_in → completed` (check-out) acontecem no Marco 3 — veja [Hospedagem e conta](hospedagem-e-conta.md). Hospedagens sem reserva nascem `checked_in` (`origin = walk_in`). Pendente, confirmada e hospedada **bloqueiam** o quarto; o mapa também mostra as concluídas.

Sinal vencido **não muda o estado**: é só um indicador calculado (`deposit_overdue`) e entra na lista "para revisar" (`list_needing_review`).

## Disponibilidade (`domain/availability.rs`)

`find_conflicts` compara **intervalos de data e hora** (entrada às `expected_check_in_time`, saída às `expected_check_out_time`) com o **intervalo de limpeza** configurado entre uma saída e a próxima entrada. A checagem roda **dentro da transação** que grava a reserva (e a alteração), sob o mesmo `Mutex` da conexão: duas criações concorrentes no mesmo quarto — só uma vence. O backend revalida tudo; a tela só antecipa a resposta (`check_room_availability`).

## Alteração com prévia (`plan_change`)

`preview_change` não escreve nada e devolve preço anterior, novo, diferença, noites (mantidas × novas), conflitos, datas sem tarifa e problemas. Noites mantidas conservam o valor combinado; as novas usam a tabela vigente.

`apply_change` exige `ChangeReservation`, `expected_version` e justificativa, **refaz o plano na transação** e compara o `fingerprint` da prévia com o do plano refeito (hash `DefaultHasher` do resultado). Se divergir, devolve `AppError::StalePreview` (`stale_preview`) e não grava; a tela mostra a mensagem e refaz a prévia. Versão diferente → `Conflict`.

## Cancelamento (`domain/cancellation.rs`)

`compute(tiers, days_before, deposit_paid, no_show)` devolve devolução e retenção sobre o **sinal pago**: faixas padrão 7 dias → 100%, 3 dias → 50%, 0 → 0%. No-show retém o sinal. Exceção (`CancellationException` + justificativa) fixa o valor retido entre 0 e o sinal pago e é auditada, preservando o valor calculado. O prazo do sinal é `min(criação + 3 dias, chegada − 1 dia)`, nunca antes da criação.

## Desconto

`apply_discount` (`ApplyDiscount`) calcula sobre o valor de tabela, em reais ou pontos-base, **substitui** o desconto anterior, recalcula o sinal esperado e registra antes/depois.

## Interino

`record_deposit` grava `deposit_paid_cents` e confirma ao atingir o esperado. É provisório até o módulo de recebimentos (Marco 5), que deve substituí-lo.

## Mapa de quartos (`room_map_service`)

`room_map(token, from, days)` — somente leitura, de 1 a 31 dias. Devolve os quartos (inclusive inativos, sinalizados) e uma célula por reserva por dia, de `check_in` a `check_out` inclusive, com `is_check_in`/`is_check_out`. No dia de troca aparecem duas células (saída e chegada).

## Auditoria e permissões

Toda escrita grava `audit_log` na mesma transação. `my_permissions` informa à interface o que oferecer; a autorização real é sempre do backend (veja [Permissões](permissoes.md)).
