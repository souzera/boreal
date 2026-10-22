# Sequências e códigos visíveis

Registros mostrados ao usuário têm um **código sequencial** além do id interno: `HOS-000042` (hóspedes). Reservas e hospedagens compartilharão o mesmo código ao longo do ciclo; recebimentos e devoluções terão sequências independentes (spec §3.3).

## Tabela `sequences`

`(name TEXT PRIMARY KEY, next_value INTEGER)`, criada na migração 0002 com `guest`, `reservation`, `stay`, `payment`, `refund` em 1.

## API

- `sequence_repo::next(tx, name) -> i64`: `UPDATE ... SET next_value = next_value + 1 RETURNING next_value - 1`, dentro da transação do chamador.
- `sequence_repo::format_code(prefix, n)`: `HOS-000042` (seis dígitos; acima disso cresce).

## Garantias

- **Sem reuso de números confirmados**: o contador só avança junto com o `commit`.
- Transação desfeita **devolve** o número (nenhuma lacuna criada por falhas): testado em `rolled_back_transaction_does_not_consume_number`.
- Como o SQLite serializa escritas e o serviço mantém uma única conexão, dois cadastros simultâneos nunca recebem o mesmo número.
- Nome de sequência desconhecido é erro (não cria sequências por engano).
