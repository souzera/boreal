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
- [Financeiro (Marco 5)](financeiro.md) — recebimentos divididos, duplicidade, devoluções, contas a pagar e categorias.
- [Fechamento de caixa (Marco 5)](fechamento-de-caixa.md) — esperado por modalidade, conferência e regras.
- [Design system (Marco 6.1)](design-system.md) — tokens, contraste, ícones, menu e atalhos.
- [Backup, restauração e recuperação (Marco 7)](backup-e-recuperacao.md) — formato, validação, retenção, restauração, abertura segura, alertas e logs.
- [Segurança operacional (Marco 7)](seguranca-operacional.md) — segredos, CSP, permissões e dependências.
- [Relatórios, CSV e PDF (Marco 6.4)](relatorios.md) — motor único, permissões, datas de referência, CSV, PDF e volume.
- [Documentos internos (Marco 6.4)](documentos.md) — instantâneo versionado, emissão e reimpressão.
- [Recepção (Marco 6.3)](recepcao.md) — visão composta do dia, painel lateral, `?abrir=` e atalhos.
- [Visão gerencial (Marco 6.3)](visao-gerencial.md) — agregações do período, permissão e gráficos.
- [Alertas (Marco 6.2)](alertas.md) — modelo, motor, detectores e regras de dispensa.
- [Busca global (Marco 6.2)](busca.md) — grupos, permissões, escape e volume.
- [Seed de desenvolvimento](seed.md) — dados fictícios para desenvolver e demonstrar (feature `dev-seed`).
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
