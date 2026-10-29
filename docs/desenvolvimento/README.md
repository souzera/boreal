# Documentação de desenvolvimento — Boreal PMS

Regras de negócio, decisões técnicas, permissões, auditoria e efeitos no banco de dados.

## Índice

- [Cadastros (Marco 1)](cadastros.md) — entidades, migrações, regras de edição, duplicidade de hóspedes, configurações.
- [Tarifas e preço (Marco 2)](tarifas-e-preco.md) — prioridade das regras, arredondamento, preço congelado.
- [Reservas e disponibilidade (Marco 2)](reservas-e-disponibilidade.md) — estados, conflitos, prévia com impressão digital, cancelamento, mapa.
- [Hospedagem e conta do hóspede (Marco 3)](hospedagem-e-conta.md) — livro de lançamentos, check-in, walk-in, estorno, check-out e decisões M3-1 a M3-10.
- [Saída tardia e check-out (Marco 3)](saida-tardia.md) — regra provisória D-15, decisão da recepção e risco para a próxima reserva.
- [Limpeza e estado do quarto (Marco 4)](limpeza-e-estado-do-quarto.md) — estado calculado, fila, prioridade e portão "quarto limpo".
- [Bloqueios e manutenção (Marco 4)](bloqueios-e-manutencao.md) — bloqueios na disponibilidade, ordens e conflitos.
- [Permissões](permissoes.md) — perfis, permissões extras por usuário e como acrescentar uma nova.
- [Sequências](sequencias.md) — códigos visíveis sem reuso.
- [Segurança de sessões e autenticação](seguranca-sessoes.md) — bloqueios, sessões, auditoria e erros.

## Decisões do Marco 0

- **Camadas:** `commands` (finos) → `services` (regras e transações) → `repositories` (SQL parametrizado) → `models`.
- **Banco:** SQLite com SQLCipher. A chave de 256 bits é gerada na primeira execução e guardada no cofre do sistema operacional. Se o banco existir e a chave não, o aplicativo recusa abrir (nunca recria).
- **Migrações:** versionadas, uma transação por migração; banco mais novo que o aplicativo é recusado.
- **Ambiente de build (Windows):** Strawberry Perl antes do Perl do Git no PATH, necessário para compilar o OpenSSL do SQLCipher.

## Como testar

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm test
npm run build
```
