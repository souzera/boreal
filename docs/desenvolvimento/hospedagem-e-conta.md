# Hospedagem e conta do hóspede (Marco 3)

## Modelo (migração `0009`)

- `reservations.origin` (`booking` | `walk_in`).
- `stays`: uma por reserva (`reservation_id UNIQUE`), `code` = código da reserva, `status` (`in_house` | `checked_out`), horários efetivos em UTC, `early_check_in`, `exceeded_capacity`, `late_tolerance_minutes` (NULL = padrão), campos do check-out (atraso, cobrança calculada e decisão, dispensa, saldo em aberto com justificativa e autorizador) e `version`.
- `stay_occupants`: titular (`is_holder`) e acompanhantes (nome, documento, `guest_id` opcional).
- `charges`: livro da conta. `amount_cents <> 0`; `kind` ∈ `night`, `consumption`, `extra`, `discount`, `adjustment`, `late_fee`, `reversal`. `reverses_charge_id UNIQUE` impede dois estornos do mesmo lançamento; índice parcial `(stay_id, night_date) WHERE kind = 'night'` impede diária duplicada.
- `payments` e `refunds`: **esquema mínimo antecipado**. `payments.method` inclui `prior_deposit` (sinal da reserva transposto no check-in). O Marco 5 move a forma de pagamento para `payment_components`.
- `catalog_items`: itens e serviços (sem estoque); nunca apagados, só desativados; `name_key` garante nome único sem acento/caixa.
- `settings.late_checkout_fee_bp`: provisório (D-15).

**Saldo** (`domain::account::balance`): `Σ charges − Σ payments + Σ refunds`, aritmética checada. Estorno é uma linha com valor negativo, então a soma já está correta.

## Decisões

| # | Decisão | Custo se estiver errada |
|---|---|---|
| M3-1 | Walk-in cria também uma linha de `reservations` (`origin = walk_in`, `checked_in`, código `AVU-`, prefixo próprio para não colidir com o código de hóspede `HOS-`). Disponibilidade, mapa e conflitos seguem lendo só `reservations`. | O walk-in aparece na lista de reservas. |
| M3-2 | Migração `0009` (o roteiro dizia `0004`). | — |
| M3-3 | Recebimento provisório com uma forma de pagamento. | O Marco 5 o substitui. |
| M3-4 | Sinal pago e desconto da reserva viram `payments`/`charges` no check-in. | — |
| M3-5 | Check-in aceita reserva pendente ou confirmada, inclusive em dia posterior ao previsto. | Pendente sem sinal pode hospedar. |
| M3-6 | O "autorizador" do check-out com saldo é o próprio usuário com `CheckoutWithBalance`. | Dupla aprovação seria um fluxo novo. |
| M3-7 | Saída tardia: `late_checkout_fee_bp` da última diária não estornada, uma vez, quando `minutos de atraso > tolerância`. | Valor provisório. |
| M3-8 | Tolerância por hospedagem exige `ChangeReservation`. | — |
| M3-9 | Saldo ≠ 0 (devedor ou crédito) exige `CheckoutWithBalance`. | Devolução do crédito só existe no Marco 5. |
| M3-10 | Check-in confere o intervalo de limpeza **previsto** (`find_conflicts`). **Atualizado no Marco 4:** também recusa quarto com limpeza aberta ou intervalo pós-limpeza não cumprido ([Limpeza](limpeza-e-estado-do-quarto.md)). | Fechado. |

## Serviços (`StayService`)

Toda operação: sessão → regras → **uma transação** com gravação + `reservation_changes` (quando muda a reserva) + `audit_log` (`module = "stays"`).

| Operação | Permissão | Efeitos |
|---|---|---|
| `check_in` | `EarlyCheckIn` (se antes do horário), `ExceedCapacity` (se acima da capacidade) | Cria hospedagem, ocupantes, diárias congeladas, desconto e sinal; reserva → `checked_in`. Revalida conflitos desde `min(início previsto, agora)`. O titular precisa constar entre os ocupantes. |
| `walk_in` | `ExceedCapacity` (se acima da capacidade) | Cria reserva `AVU-` + hospedagem, preço pela tabela de tarifas. |
| `add_charge` | — | Consumo/despesa extra (avulso ou do catálogo). |
| `add_adjustment` | `ApplyDiscount` | Ajuste com justificativa. |
| `reverse_charge` | `ReverseEntry` | Estorno (+ relançamento opcional) na mesma transação. |
| `record_payment` | — | Provisório. |
| `checkout` | `WaiveLateFee`, `CheckoutWithBalance` (conforme o caso) | Ver [Saída tardia](saida-tardia.md). Hospedagem → `checked_out`, reserva → `completed`. |
| `set_late_tolerance` | `ChangeReservation` | 0–600 minutos ou NULL. |

Lançamentos, estornos e pagamentos incrementam `stays.version`; assim, uma prévia de check-out feita antes de uma mudança na conta é recusada (`Conflict`) e a tela recarrega o demonstrativo. `reverse_charge` recusa estornar `reversal`, lançamento já estornado e hospedagem encerrada.

Limites: lançamento ou recebimento de R$ 0,01 a R$ 1.000.000,00; preço de item até R$ 100.000,00.

## Disponibilidade e hospedagem em andamento

`reservation_repo::blocking(conn, room, from, to, now)` devolve sempre as reservas `checked_in` do quarto, qualquer que seja a data; se a saída prevista já passou, a ocupação termina em `now` (o hóspede ainda não saiu). Assim check-in, hospedagem sem reserva, nova reserva e alteração enxergam o quarto ocupado até o check-out. O check-in é recusado quando `agora ≥ saída prevista` da própria reserva (altere as datas antes).

## Erros e limitações conhecidas

- Não há devolução (`refunds` só entra no saldo); crédito do hóspede exige a exceção no check-out.
- O sinal pago antes do check-in aparece como recebimento `prior_deposit`; `record_deposit` continua só para reservas abertas.
- Ao concluir o check-out antes da data prevista, a reserva fica `completed` com as datas originais; o quarto passa a ser livre para a disponibilidade, e o check-out cria a tarefa de limpeza (Marco 4): o quarto só recebe novo check-in depois da limpeza concluída e do intervalo.
- Ocupante sem nome ou lista sem o titular é recusada.

## Testes

`cargo test` (serviço: check-in, walk-in, conta, check-out, auditoria, transações), `domain::{stay, account, late_checkout}` (funções puras), migração `0009` (restrições do livro), `npm test` (lista, tela, diálogos de check-in, walk-in, lançamento, estorno, pagamento e check-out).
