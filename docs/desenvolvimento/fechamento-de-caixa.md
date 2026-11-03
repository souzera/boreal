# Fechamento de caixa (Marco 5)

Spec §11. Um fechamento por data local, sem abertura formal de caixa.

## Cálculo do esperado

Para a data local `D` (limites UTC por `domain::dates::utc_bounds` com o deslocamento do fuso do computador):

- `recebido[método] = Σ payment_components` dos recebimentos cujo `received_at` está em `D`;
- `devolvido[método] = Σ refund_components` das devoluções cujo `refunded_at` está em `D`;
- `esperado[método] = recebido − devolvido`, para `cash`, `pix`, `debit`, `credit`, `transfer` e, **só quando há movimento**, `unspecified` (dados legados).
- `supplier_payments_cents = Σ payable_payments` do dia: **informativo, não entra no esperado** (M5-10; se a pousada paga fornecedor com o dinheiro do caixa, basta subtrair por modalidade).

## API

- `ClosingService::preview(token, "YYYY-MM-DD") -> ClosingPreview` (esperado atual, fechamento gravado se houver, `after_closing_cents`).
- `ClosingService::close(token, CloseInput { date, counted[] }) -> ClosingView`.
- `ClosingService::pending_dates(token) -> Vec<String>`: dias **anteriores a hoje** com recebimento ou devolução e sem fechamento (hoje nunca é "pendente").

## Regras

- Qualquer usuário autenticado fecha; data ≤ hoje; o esperado é **recalculado no servidor**, o cliente só envia os conferidos.
- Exige o conferido das cinco modalidades (e de `unspecified` quando há esperado); conferido ≥ 0; modalidade repetida ou desconhecida é recusada.
- Diferença ≠ 0 exige justificativa **na linha**, mas nunca impede concluir (`domain::closing::build_closing_lines`).
- `cash_closings.closing_date` é `UNIQUE`: o segundo fechamento do mesmo dia (inclusive duplo clique) é recusado ("Esse dia já foi fechado.").
- Linhas gravadas: esperado, conferido, diferença e justificativa; auditoria `close_cash` (módulo `finance`) na mesma transação.
- **Nada em `payments`/`refunds` é alterado.** Operações posteriores ao fechamento continuam permitidas; `after_closing_cents` = esperado atual − esperado gravado, informativo. Não há transferência retroativa entre períodos.

## Testes

`closing_service`: esperado por modalidade com débito e crédito separados, diferença justificada, validações sem gravação, fechamento duplicado, data futura, fechamento tardio, virada de dia local (23:30 × 00:10 em UTC−03:00), pendência que não bloqueia, movimentos depois do fechamento, fornecedores à parte e linha `unspecified`.
