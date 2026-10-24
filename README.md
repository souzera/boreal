# Boreal PMS

Sistema desktop de gestão para uma única pousada (reservas, hospedagens, limpeza, financeiro e relatórios). Funciona offline, em um computador na recepção, com os dados guardados localmente em um banco SQLite criptografado.

Estado atual: **Marco 2 (tarifas e reservas)** — além da fundação (banco criptografado, auditoria, autenticação com permissões) e dos cadastros, o sistema calcula preços por tarifa, cria, altera e cancela reservas com disponibilidade validada e mostra o mapa de quartos. Hospedagem, financeiro e relatórios vêm nos próximos marcos.

## Tecnologias

- **Backend:** Rust + Tauri 2, `rusqlite` com SQLCipher, Argon2id para senhas.
- **Interface:** React + TypeScript, Vite, Tailwind CSS, shadcn/ui, React Router, TanStack Query, Zustand, React Hook Form + Zod, date-fns, Recharts.
- **Testes:** `cargo test` (Rust) e Vitest + Testing Library (interface).

## Pré-requisitos (Windows)

- [Node.js](https://nodejs.org) 22 ou superior
- [Rust](https://rustup.rs) estável, com as ferramentas de build do Visual Studio (C++)
- WebView2 (já incluído no Windows 11 e nas versões recentes do Windows 10)
- **Strawberry Perl** (`winget install StrawberryPerl.StrawberryPerl`): necessário para compilar o OpenSSL usado pelo SQLCipher. Deixe `C:\Strawberry\perl\bin` antes do Perl do Git no `PATH`.

## Instalação e execução

```bash
npm install --legacy-peer-deps
npm run tauri dev
```

`--legacy-peer-deps` evita um conflito de peer dependency opcional do `@hookform/resolvers`.

Na primeira execução o sistema pede a criação do usuário administrador.

## Testes

```bash
npm test                                              # interface
cargo test --manifest-path src-tauri/Cargo.toml       # backend
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Estrutura

```
src/                 interface React (pages, components, stores, lib)
src-tauri/src/
  commands/          comandos Tauri (camada fina, sem regra de negócio)
  services/          regras de negócio e transações
  repositories/      acesso ao SQLite
  models/            tipos de domínio
  auth/              senhas, permissões e sessões
  audit/             auditoria centralizada
  migrations/        migrações versionadas (sql/)
  db/                abertura do banco criptografado e cofre da chave
docs/superpowers/plans/   planos de implementação
```

## Segurança

- O frontend não acessa o banco: tudo passa por comandos Tauri, e **permissões e validações são aplicadas no backend**.
- O banco é criptografado (SQLCipher). A chave de 256 bits é gerada na primeira execução e guardada no cofre de credenciais do sistema operacional, nunca em arquivos.
- Se o banco existir e a chave não estiver disponível, o aplicativo **não abre nem recria** o banco e mostra um erro.
- **Atenção:** a perda definitiva da chave torna os dados irrecuperáveis. O procedimento de recuperação e troca de computador ainda está pendente de definição.
- A auditoria é imutável (triggers no banco) e nunca registra senhas.
- Não versione bancos de dados, backups, credenciais ou `.env` (o `.gitignore` já cobre os casos comuns).
