# Hóspedes

**Quem pode usar:** administradores e recepção (somente administradores desativam um cadastro).

## Para que serve

Cadastrar e consultar o **responsável pela hospedagem**. O histórico de reservas e pagamentos de cada hóspede será exibido aqui quando esses módulos forem entregues.

## Cadastrar um hóspede

1. Abra **Hóspedes** e clique em **Novo hóspede**.
2. Preencha **nome completo** e **telefone** (obrigatórios). CPF, e-mail e endereço são opcionais.
3. Clique em **Salvar hóspede**.

O CPF e o telefone ganham a formatação sozinhos enquanto você digita. Se o CPF estiver errado, o campo avisa antes de salvar.

**Resultado:** o hóspede recebe um código (por exemplo `HOS-000042`) e você volta à lista.

## Quando o hóspede já pode estar cadastrado

Ao salvar, o sistema compara **nome** (ignorando acentos e maiúsculas), **telefone** e **CPF** com os cadastros existentes. Se achar algo parecido, mostra uma lista com o motivo ("mesmo telefone", "mesmo CPF", "nome parecido") e você escolhe:

- **Usar este cadastro** — abre o cadastro que já existe e nada novo é criado.
- **Criar outro cadastro** — é outra pessoa; o cadastro novo é criado.
- **Voltar e revisar** — fecha o aviso; o que você digitou continua na tela.

## Buscar

Na lista ou no campo de busca do topo, digite parte do nome (sem precisar de acento), telefone, CPF ou o código.

## Editar e desativar

Clique na linha do hóspede para editar. Administradores veem **Desativar hóspede**: o cadastro deixa de aparecer para novas reservas, mas o histórico é mantido.

## Se algo der errado

| Mensagem | O que fazer |
|---|---|
| "Telefone inválido. Informe o DDD e o número." | Digite o telefone com DDD (10 ou 11 dígitos). |
| "CPF inválido. Confira os números." | Revise o CPF ou deixe o campo em branco. |
| "O registro foi alterado por outra operação." | Outra pessoa editou ao mesmo tempo. Volte à lista, abra o cadastro de novo e refaça a alteração. |

Se você sair da tela com alterações não salvas, o sistema pergunta: **Salvar**, **Descartar** ou **Continuar editando**.
