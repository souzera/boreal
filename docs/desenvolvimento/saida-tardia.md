# Saída tardia e check-out

## Regra (provisória, D-15)

`domain::late_checkout::assess(saída_prevista, agora, tolerância, última_diária, fee_bp)`:

- `minutos_de_atraso = max(0, agora − saída_prevista)` (data de calendário + horário previsto, no fuso local da pousada).
- `aplica = fee_bp > 0 && minutos_de_atraso > tolerância`. Exatamente na tolerância (30 min) **não** cobra; com 31 min cobra.
- `cobrança = percent_of(última_diária, fee_bp)` (meio centavo sobe), **uma única vez**: atrasar dias não gera uma cobrança por dia, e **nunca** gera uma diária.
- Tolerância = a da hospedagem (`stays.late_tolerance_minutes`) ou, se nula, `late_checkout_tolerance_minutes`.
- Última diária = a última `night` não estornada (`stay_repo::last_night_cents`).

Os valores (30 min, 50%) vivem em `settings` e são editáveis; nada é constante no código.

## Check-out (`StayService::checkout`)

Uma transação:

1. Compara `expected_version` (`Conflict` se a conta ou a hospedagem mudou) e a transição `in_house → checked_out` (`domain::stay::transition`).
2. Decisão da saída tardia (`LateFeeChoice`): se aplica, exige `Charge` (grava um `late_fee`) ou `Waive { justification }` (exige `WaiveLateFee` + justificativa e **não** grava lançamento; preserva atraso e valor calculado); `NotApplicable` quando aplica, ou `Charge`/`Waive` quando não aplica, são recusados.
3. Saldo recalculado depois da cobrança. Se ≠ 0 exige `balance_exception` com `CheckoutWithBalance` e justificativa; grava saldo, justificativa e autorizador. Mensagens distintas para saldo devedor e crédito do hóspede.
4. Hospedagem → `checked_out`; reserva → `completed`; `reservation_changes` (`check_out`) e auditoria na mesma transação. Falha em qualquer passo desfaz tudo (inclusive o `late_fee`).

`preview_checkout` calcula os mesmos números sem gravar: atraso, cobrança prevista, saldo atual (`balance_cents`), saldo com a cobrança (`balance_with_late_fee_cents`) e risco.

## Risco para a próxima reserva

`departure_risk(saída_prevista, agora, limpeza, início_da_próxima)`: só existe se `agora > saída_prevista` e `agora + limpeza > início_da_próxima`. A próxima reserva é a bloqueante do mesmo quarto com início ≥ saída prevista (60 dias à frente). É **só informação** (lista de hospedagens e demonstrativo): nada é alterado ou cancelado. Alertas persistentes ficam para o Marco 6.

## Testes

`domain::late_checkout` (fronteiras de 30/31 min, 45 h, virada de ano, arredondamento, risco) e `StayService` (cobrar, dispensar, tolerância por hospedagem, cobrança única, diária nunca criada, reversão em falha, risco sem tocar na reserva).
