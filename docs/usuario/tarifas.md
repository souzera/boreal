# Tarifas

**Quem pode usar:** todos consultam; **administradores** cadastram e alteram.

## Para que serve

A tabela de tarifas define quanto custa cada diária de cada categoria de quarto. O sistema usa essa tabela para calcular o preço de toda reserva. **Sem tarifa para uma noite, a reserva não pode ser criada**: o sistema nunca assume "R$ 0,00".

## Tipos de regra

Quando mais de uma regra vale para a mesma noite, vence a de cima:

1. **Exceção de data** — um preço para um dia específico (por exemplo, um evento na cidade).
2. **Feriado** — vale para as datas cadastradas no calendário de feriados.
3. **Temporada** — vale num período (por exemplo, de 20/12 a 10/01).
4. **Dia da semana** — o preço comum de cada dia (segunda a domingo).

A **data de saída não gera diária**: uma estadia de 25/10 a 28/10 tem 3 noites (25, 26 e 27).

## Cadastrar uma regra

1. Abra **Tarifas** e escolha a categoria.
2. Use **Definir** (dia da semana e feriado), **Nova temporada** ou **Nova exceção de data**, preencha o valor em reais (por exemplo `150,00`) e as datas.
3. Salve.

Use **Simular preço** para conferir quanto custaria uma estadia antes de reservar.

**Resultado:** a regra aparece na lista. Reservas que já existem **não mudam**: o preço combinado de cada noite fica congelado no momento da reserva.

## Calendário de feriados

Em **Tarifas**, no **Calendário de feriados**, use **Adicionar feriado** para cadastrar cada data (inclusive feriados municipais). Uma regra do tipo **Feriado** vale para todas elas.

## Se algo der errado

| Mensagem | O que fazer |
|---|---|
| "Não há tarifa para: …" (ao reservar) | Cadastre uma regra que cubra as datas listadas. |
| "Esta temporada se sobrepõe a outra temporada ativa…" | Ajuste as datas; duas temporadas ativas da mesma categoria não podem valer no mesmo dia. |
