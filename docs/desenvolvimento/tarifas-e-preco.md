# Tarifas e preço (Marco 2)

## Tabelas (migrações `0006`, `0007`)

- `rate_rules`: regra por categoria — `weekday`, `season`, `holiday` ou `date_override` — com valor em centavos, validado para nunca ser zero.
- `holidays`: calendário de feriados; a regra `holiday` vale para as datas ali cadastradas.
- Política comercial em `settings` (percentual e prazo do sinal) e `cancellation_tiers` (faixas de devolução). **Nenhum número comercial é constante no código**: os padrões do Anexo A da spec entram só na migração.

Migrações já publicadas nunca são editadas; mudanças entram em migração nova.

## Prioridade e preço (`domain/pricing.rs`)

`price_stay(rules, holidays, check_in, check_out) -> Quote` é função pura. Para cada noite de `check_in` a `check_out - 1`:

1. exceção de data; 2. feriado; 3. temporada; 4. dia da semana.

A noite de saída não é cobrada. Noites sem regra vão para `missing_dates` e **bloqueiam** a reserva (o serviço devolve as datas). O serviço de tarifas recusa temporadas ativas sobrepostas na mesma categoria.

## Dinheiro e arredondamento (`domain/money.rs`)

Valores são `i64` em centavos, com aritmética `checked_*`. Percentuais usam pontos-base (`3000` = 30%) e `percent_of(cents, bp)` arredonda **meio para cima, uma única vez, sobre o total** (nunca por noite). Exemplo da suíte: R$ 1.003,35 a 30% → R$ 301,01.

## Preço congelado

Ao criar a reserva, cada noite é gravada em `reservation_nights` (data, valor, regra aplicada). Mudar a tabela depois **não altera** reservas existentes; só a alteração de datas/quarto reprecifica as noites **novas** (veja [Reservas e disponibilidade](reservas-e-disponibilidade.md)).
