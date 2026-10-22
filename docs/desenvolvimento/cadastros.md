# Cadastros (Marco 1)

## Entidades e migrações

| Migração | Tabelas | Observações |
|---|---|---|
| 0002 | `sequences`, `user_permissions` | códigos visíveis; permissões extras por usuário |
| 0003 | `pousada`, chaves novas em `settings` | `pousada` é linha única (`id = 1`) com `version` |
| 0004 | `room_categories`, `rooms` | nome/código únicos sem diferenciar caixa; FK `ON DELETE RESTRICT` |
| 0005 | `guests` | `name_key`, `phone`, `cpf` normalizados; índices **não únicos** |

## Regras comuns

- Toda edição recebe `expected_version`; o `UPDATE ... WHERE version = ?` que não altera linha vira `AppError::Conflict`.
- Desativar (`active = 0`) não apaga nada. Exclusão física (`delete_room`, `delete_category`) só funciona sem dependências: a violação de chave estrangeira vira mensagem orientando a desativar. Não há botão de exclusão na interface.
- Listagens paginadas devolvem `Page<T>`; `page_size` é limitado a 1–100 e a ordenação usa lista fixa de colunas (nunca texto do cliente em SQL). Buscas escapam `%`, `_` e `\` (`like_pattern`).
- Leitura: qualquer sessão ativa e sem troca de senha pendente. Escrita do catálogo: `ManageCatalog` (só administrador). Hóspedes: criar/editar por qualquer sessão; desativar exige `ManageCatalog`.
- Auditoria na mesma transação, com `before`/`after`. Hóspedes: só nome e **CPF mascarado** (`***.***.***-25`); e-mail, endereço e telefone não vão à auditoria.

## Hóspedes: normalização e duplicidade

- `domain::text::fold`: minúsculas, sem acentos, espaços colapsados (chave `name_key`).
- `domain::phone::normalize`: só dígitos, 10–11 após remover `55` inicial quando o número tem 12–13 dígitos.
- `domain::cpf::normalize`: dígitos verificadores corretos, rejeita dígito repetido, aceita máscara. O frontend repete a validação (`lib/cpf.ts`) só para experiência; o backend decide.
- `GuestService::create(input, confirm_duplicates)`: com coincidência de nome, telefone ou CPF e sem confirmação **não grava nada** e devolve `Duplicates { matches }` (com os motivos). Com `confirm_duplicates = true`, cria outro cadastro. O código `HOS-000042` sai da sequência `guest` na mesma transação (rollback devolve o número).

## Configurações da pousada

`AppSettings` tem faixas validadas no backend (tentativas 3–20, bloqueio 1–1440 min, ociosidade 1–480 min, horários `HH:MM`, tolerâncias 0–600). Os padrões (`5/5/10`, `14:00`/`12:00`, `30`, `60`, `60`) são **provisórios** (pendência §25.5). `settings_version` controla edição concorrente do conjunto; `update` aplica `idle_lock_minutes` imediatamente em `SessionManager`.

Logotipo: PNG ou JPEG, assinatura de arquivo conferida com o tipo informado, ≤ 512 KB, guardado como BLOB; nunca aparece em `Debug` nem na auditoria.

## Testes

Serviços testados com SQLite em memória e relógios injetados (`services::test_support::Harness`): permissões, conflito de versão, auditoria, regras de duplicidade, exclusão com histórico (tabela `room_refs_test` criada no teste). Interface: Vitest + Testing Library com `renderPage` (roteador de dados + `QueryClientProvider`).

## Limitações conhecidas

- Histórico de reservas/pagamentos do hóspede ainda não existe (Marcos 2 em diante).
- A busca global só consulta hóspedes.
- Busca por nome usa `LIKE` sobre `name_key` (sem índice de texto); suficiente para o porte da pousada.
