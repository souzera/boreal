# Documentação de desenvolvimento — Boreal PMS

Regras de negócio, decisões técnicas, permissões, auditoria e efeitos no banco de dados.

## Decisões do Marco 0

- **Camadas:** `commands` (finos) → `services` (regras e transações) → `repositories` (SQL parametrizado) → `models`.
- **Banco:** SQLite com SQLCipher. A chave de 256 bits é gerada na primeira execução e guardada no cofre do sistema operacional. Se o banco existir e a chave não, o aplicativo recusa abrir (nunca recria).
- **Sessões:** em memória, com token aleatório; bloqueio por inatividade usa relógio monotônico; a troca de senha temporária é imposta no backend.
- **Auditoria:** tabela `audit_log` imutável (triggers), gravada na mesma transação da operação; nunca contém senhas.
- **Migrações:** versionadas, uma transação por migração; banco mais novo que o aplicativo é recusado.
- **Ambiente de build (Windows):** Strawberry Perl antes do Perl do Git no PATH, necessário para compilar o OpenSSL do SQLCipher.
