# Seed de desenvolvimento

Popula um banco **vazio** com dados fictícios para desenvolver e demonstrar o sistema. Não existe no build de produção: o código fica atrás da feature Cargo `dev-seed` (`src-tauri/src/seed.rs`, exemplo `src-tauri/examples/seed.rs`).

## Como rodar

```bash
BOREAL_SEED_ADMIN_PASSWORD='uma-senha-de-dev' \
  cargo run --manifest-path src-tauri/Cargo.toml --example seed --features dev-seed -- --db <caminho/boreal.db>
```

- `--db` é obrigatório; o seed nunca escolhe o banco sozinho. O aplicativo guarda o dele em `%APPDATA%\br.com.boreal.pms\boreal.db` (Windows).
- A chave de criptografia vem do cofre de credenciais do sistema (a mesma do aplicativo). **Use o caminho de um banco de desenvolvimento**; não rode contra um banco com dados reais.
- `BOREAL_SEED_ADMIN_PASSWORD`: senha do administrador `admin` (criado se ainda não houver usuários; se já existir, é a senha atual). `BOREAL_SEED_RECEPCAO_PASSWORD` (opcional) cria o usuário `recepcao` com senha temporária (troca obrigatória no primeiro acesso). Nenhuma senha fica no repositório.
- O seed **recusa** banco que já tenha quartos.

## O que ele cria

Passa pelos serviços reais (regras, auditoria e códigos sequenciais ficam consistentes). Datas são relativas a hoje.

| Item | Conteúdo |
|---|---|
| Quartos e tarifas | Categorias Standard (R$ 180,00) e Luxo (R$ 280,00); quartos 101–104 e 201–204 |
| Itens | Água mineral, Refrigerante, Lavanderia, Café da manhã extra |
| Hóspedes | 10 nomes e telefones inventados (sem CPF) |
| 101 | Hospedado, com consumos e saldo a receber |
| 102 | Saiu hoje, **aguardando limpeza**; reserva confirmada chegando amanhã (prioridade sobe) |
| 103 | Saiu hoje, **em limpeza** |
| 104 | Limpeza concluída, **aguardando o intervalo** |
| 201 | Bloqueio manual futuro (pintura) |
| 202 | Reserva confirmada afetada por uma ordem de manutenção (bloqueio preventivo + conflito a resolver) |
| 203 | Ordem de manutenção em andamento hoje (quarto **bloqueado**) |
| 204 | Reserva pendente e reserva confirmada com sinal |

## Testes

`cargo test --features dev-seed --lib seed`: cenário completo (estado de cada quarto), filas preenchidas, recusa de banco não vazio, banco com administrador já criado e senha errada sem alterar nada. O exemplo em si usa o cofre real e não é executado nos testes.

## Limitações

Só roda uma vez por banco. O financeiro do Marco 5 (devoluções, contas a pagar, fechamento) será acrescentado ao seed quando existir.
