# Documentação de desenvolvimento — Boreal PMS

Regras de negócio, decisões técnicas, permissões, auditoria e efeitos no banco de dados.

## Índice

- [Cadastros (Marco 1)](cadastros.md) — entidades, migrações, regras de edição, duplicidade de hóspedes, configurações.
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
