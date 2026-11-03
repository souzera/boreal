# Busca global (Marco 6.2)

Spec §20.1. Serviço de leitura: `SearchService::search(token, text) -> SearchResults`.

## Regras

- Texto aparado, **mínimo de 2 caracteres** (`Validation`), `%`, `_` e `\` escapados (`like_pattern` + `ESCAPE '\'`): são letras comuns.
- Comparação sem acento e sem caixa pelo `name_key` (hóspede) e por `LIKE` do SQLite (ASCII, para códigos).
- Telefone e CPF casam por **dígitos**, só com 4 ou mais (números curtos, como o do quarto, não geram ruído). O CPF volta **mascarado**.
- O texto igual ao **código do quarto** casa as reservas e hospedagens daquele quarto.
- Quatro grupos, até **8 itens** cada, com o total real. Ordem: código exato, depois nome exato (hóspedes), depois mais recente.
- **Movimentos:** recebimentos (`PAG-`) e devoluções (`DEV-`) por código ou nome do hóspede; contas a pagar (`CAP-`, fornecedor, descrição) **somente com `ManagePayables`**. O grupo e o total não revelam contas a pagar a quem não tem a permissão.
- Reserva e hospedagem compartilham o código (spec §3.3): a mesma busca devolve as duas, em grupos diferentes.

## Decisões

- **Sem FTS5** (A6): `LIKE` com os índices únicos de código e o `name_key` bastam no volume de uma pousada, e não dependemos do suporte a FTS5 do SQLCipher. Teste de volume: `search_over_five_years_of_data_stays_fast` (≈ 15 mil reservas com hospedagens e recebimentos; cada busca abaixo de 500 ms no build de teste).
- Se o volume real crescer muito, o primeiro passo é um índice em `reservations(room_id, check_in_date)` e `guests(phone)`, e só depois FTS5.

## Interface

`/busca?q=` mostra os grupos com ícone e rótulo de situação, link para cada registro (`/hospedes/:id`, `/reservas/:id`, `/hospedagens/:id`, ou o `link` do movimento), a contagem ("Mostrando 8 de 12") e orientação quando não há resultado. O campo do topo envia qualquer texto não vazio e a página de resultados explica a regra dos 2 caracteres; `Ctrl+K` foca o campo.

## Testes

`search_service` (nome, telefone, CPF, códigos em qualquer caixa, quarto, curingas literais, mínimo de 2, limite de 8, permissão de contas a pagar, sessão obrigatória, volume de 5 anos) e `Search.test.tsx`.
