# Financeiro (Marco 5)

Recebimentos divididos, devoluções, bloqueio de duplicidade, contas a pagar e categorias de despesa. O fechamento de caixa tem documento próprio: [fechamento-de-caixa.md](fechamento-de-caixa.md). Plano: `docs/superpowers/plans/2026-10-30-marco-5-financeiro.md`; spec §9, §10.2 e Anexo A.5.

## Modelo de dados (migração `0011`)

| Tabela | Papel |
|---|---|
| `payments` | Recebimento: total (`amount_cents`), `kind` (`deposit` = sinal da reserva, `stay` = conta), `stay_id` nulo enquanto é sinal, campos de liberação de duplicidade. **Sem** modalidade. |
| `payment_components` | Parte do recebimento por modalidade; `UNIQUE(payment_id, method)`; a soma é o total. |
| `refunds` / `refund_components` | Devolução ligada a `payment_id` (e a `reservation_id`/`stay_id`), com motivo, usuário e instante. |
| `expense_categories` | Categorias de despesa (`name_key` único sem acento/caixa); desativadas, nunca apagadas. |
| `payables` / `payable_payments` | Conta a pagar e seus pagamentos (modalidade, instante, usuário). Situação **calculada**. |
| `cash_closings` / `cash_closing_lines` | Fechamento diário; ver o outro documento. |
| `reservations.cancellation_refund_cents` | Devolução calculada no cancelamento (política ou exceção). |

A migração reconstrói `payments` e `refunds` por cópia temporária (sem `RENAME`, que reescreveria chaves estrangeiras). Dados anteriores migram com modalidade `unspecified` (nunca oferecida na interface; o sinal antigo `prior_deposit` também vira `unspecified`).

Valores em centavos inteiros; instantes em UTC; "dia" sempre é a data **local** (`ctx.today()` / `utc_bounds`).

## Serviços e contratos

- `PaymentService::record(token, PaymentTarget, PaymentInput) -> PaymentReceipt`
  - `PaymentTarget::Stay { stay_id }` (hospedagem em andamento) ou `Reservation { reservation_id, expected_version }` (pendente/confirmada, `Conflict` se a versão mudou, sinal não passa do total combinado).
  - Valida os componentes (`domain::payment::validate_components`): lista não vazia, valores > 0, sem modalidade repetida, `unspecified` recusada, total ≤ R$ 1.000.000,00.
  - **Duplicidade (M5-6):** mesma reserva, mesmo total, mesmo conjunto de modalidades e mesma data local ⇒ `AppError::DuplicatePayment` (código `duplicate_payment`, `details` com o recebimento parecido). Com `duplicate_release` exige `ReleaseDuplicatePayment` e justificativa; grava `duplicate_of_payment_id`, justificativa e responsável.
  - Na mesma transação: número `PAG-` (sequência `payment`), componentes, `deposit_paid_cents`/status da reserva (sinal) ou `bump` da hospedagem, histórico da reserva e auditoria (`record_payment` em `stays`; `record_deposit` em `reservations`).
- `StayService::check_in` não copia mais o sinal: `payment_repo::link_to_stay` preenche `stay_id` nos recebimentos da reserva. `reservations.deposit_paid_cents` continua como soma materializada.
- `RefundService::record(token, RefundInput)`: motivo obrigatório (≤ 200), componentes válidos, total ≤ `payment − já devolvido`; número `DEV-`; auditoria `record_refund` (módulo `finance`, entidade `payment`). **Sem permissão extra (M5-7).** Funciona também em hospedagem encerrada e em reserva cancelada.
- Cancelamento (`ReservationService::cancel`) grava `cancellation_refund_cents`; `Reservation.refund_due_cents` = `max(0, cancellation_refund_cents − devolvido)` só para reservas canceladas. O cancelamento **não** cria devolução nem mexe em recebimentos (M5-8).
- `PayableService`: `create` (opcional `paid_now` = pagamento total na mesma transação), `list` (filtros e situação calculada em SQL), `get`, `pay` (parcial/total, `Conflict` por versão, nunca acima do saldo), `summary` (competência × data de pagamento) e categorias (`ManageCatalog`). Todas as operações de conta exigem `ManagePayables`.

Saldo da hospedagem permanece `lançamentos − recebido + devolvido` (`domain::account::balance`), somando `payments.amount_cents` e `refunds.amount_cents` por `stay_id`.

## Permissões

`ReleaseDuplicatePayment` e `ManagePayables` (ambas concedíveis); categorias exigem `ManageCatalog`. Veja [permissoes.md](permissoes.md).

## Decisões (M5-x) e valores provisórios

M5-1 migração `0011`; M5-2 e M5-3 sem tipo `Money` novo e despesas extras já no saldo; M5-4 sinal vira recebimento real vinculado no check-in; M5-5 legado `unspecified`; **M5-6 duplicidade bloqueia sem fila de retidos**; **M5-7 devolução sem permissão extra**; **M5-8 cancelamento não devolve sozinho**; **M5-9 permissão `ManagePayables`**; M5-10 a M5-11 fechamento; M5-12 visões por competência e pagamento; M5-13 categorias iniciais. Os em negrito e o M5-10/M5-11 estão no Anexo A.5 da spec, **aguardando aprovação**.

## Casos de erro

`Validation` (componentes, motivo, saldo, estado da reserva/hospedagem), `Conflict` (versão), `Forbidden` (permissão), `DuplicatePayment`. Nenhum erro deixa gravação parcial: a transação é desfeita e o número de sequência volta.

## Limitações conhecidas

- Não há fila de recebimentos retidos para revisão: o duplicado simplesmente não é gravado até alguém autorizado liberar.
- Recebimentos e devoluções não têm edição nem exclusão; correção é por devolução e novo recebimento.
- Pagamentos a fornecedores não entram no fechamento de caixa (M5-10).

## Testes

`cargo test` cobre `payment_service`, `refund_service`, `payable_service`, `closing_service`, os domínios `payment`, `payable` e `closing`, a migração `0011` (inclusive a preservação de dados) e as permissões. No front: `lib/payment`, `ComponentsEditor`, `PaymentDialog`, `RefundDialog`, `StayPage`, `ReservationPage`, `PayablesPage`, `ClosingPage`, `Dashboard`, `Sidebar` e `api`. O seed (`--features dev-seed`) tem teste do cenário financeiro.
