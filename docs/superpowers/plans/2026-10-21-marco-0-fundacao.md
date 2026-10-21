# Marco 0 — Fundação do PMS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entregar o esqueleto executável do PMS: app Tauri com banco SQLite criptografado, migrações, auditoria transacional, autenticação com permissões no backend e shell de interface (login, bloqueio, menu lateral, dashboard vazio).

**Architecture:** Backend Rust em camadas (`commands` → `services` → `repositories`, com `models`, `auth`, `audit`, `migrations`, `db`). O frontend React só fala com o backend por comandos Tauri; toda regra e checagem de permissão vive em Rust. Banco acessado via `rusqlite` + SQLCipher; a chave fica no cofre de credenciais do SO.

**Tech Stack:** Tauri 2, Rust, rusqlite (`bundled-sqlcipher`), keyring, argon2, React + TypeScript, Vite, Tailwind CSS, shadcn/ui, React Router, TanStack Query, Zustand, React Hook Form, Zod, date-fns, Vitest, GitHub Actions.

**Spec:** `local/spec.md` (seções 2, 3.3, 13, 14, 16.1, 18, 19, 23)

## Status — Marco 0 concluído (2026-10-21)

Branch `marco-0-fundacao`; 11 tarefas implementadas com TDD e revisão final da branch inteira.

- **Testes na última execução:** 43 unitários + 3 de integração (Rust), 21 (Vitest); `clippy -D warnings` e `vite build` sem erros.
- **Pendente de verificação manual:** `npm run tauri dev` (abrir janela, primeira execução → login → bloqueio por inatividade). Rodar antes de abrir o Marco 1.
- **Integração:** aguardando decisão (merge local, PR ou manter).

### Desvios do plano (decididos durante a execução)

- `npm install --legacy-peer-deps` (peer opcional de `@hookform/resolvers`); o CI usa o mesmo.
- Strawberry Perl é pré-requisito para compilar o SQLCipher/OpenSSL no Windows (`C:\Strawberry\perl\bin` antes do Perl do Git no PATH).
- `SessionManager::create(user_id, role, now)` recebe o perfil; novos erros: `Locked`, `InvalidCredentials`, `AccountLocked`, `KeyInvalid`, `SchemaTooNew`, `PasswordChangeRequired`.
- Tarefa 8: descartado o `last_seen_utc` (voltar o relógio só alonga o bloqueio); adicionados `settings_repo`, `Ctx`, `user_repo::{find_with_hash, set_active}`; senha errada no desbloqueio conta para o bloqueio da conta.
- Dependência extra `chrono`; `argon2` compilado com `opt-level = 3` em dev (testes ~20x mais rápidos).
- CI e templates do GitHub (`.github/`) foram criados antes da Tarefa 11.

### Correções da revisão final (todas com teste vermelho → verde)

Chave inválida com mensagem clara; troca de senha temporária imposta no backend (`touch` estrito vs `touch_allow_pending`); desativar usuário encerra sessões e impede desbloqueio; tela de bloqueio como cobertura (página continua montada) + heartbeat de atividade a cada 30 s; banco mais novo que o app é recusado.

### Pendências adiadas (menores)

Desbloqueio logo após expirar o bloqueio re-bloqueia no primeiro erro; hash fictício preguiçoso; bloqueio revela usernames; `change_own_password` não conta erros nem encerra outras sessões; "Sair" na tela de bloqueio não encerra a sessão no backend; `/change-password` fora do `AppShell`; `IDLE_MINUTES` fixo no frontend; timestamps de auditoria sem relógio injetado; checagem de versão em `reset_password` sem efeito; `Debug` em structs com segredos; erros crus do banco na UI; mutex envenenável; `csp: null` e plugin `opener` sem uso; metadados do `Cargo.toml`; `get_app_info` sem consumidor na UI; Zod `min(8)` vs caracteres do backend. Sem `integrity_check` pós-migração e sem backup antes de migrar (previstos para os marcos de backup/recuperação).

### Documentação ainda não criada

`CHANGELOG.md`, `docs/usuario/` e `docs/desenvolvimento/` (exigidos pelo `CLAUDE.md`) ainda não existem.

---

## Global Constraints

- Aplicação desktop **Rust + Tauri**; UI **React + TypeScript**, **Tailwind CSS + shadcn/ui** (§2.1).
- Frontend sem acesso direto ao banco; comunicação só por comandos Tauri, sem API HTTP local (§2.2).
- Camadas: `commands`, `services`, `repositories`, `models`, `auth`, `audit`, `migrations` (§2.2).
- Operações com múltiplas gravações usam transação; auditoria na mesma transação da operação (§2.3, §13.4).
- Controle de acesso aplicado no backend, não só ocultando botões (§13.1).
- Perfis iniciais: administrador e recepção (§13.1).
- Senha inicial definida pelo admin e trocada no primeiro acesso; só o admin redefine senha de outro usuário; senha temporária exige troca no próximo acesso (§13.2).
- Bloqueio temporário da conta após limite configurável de tentativas; evento auditado (§13.2).
- Bloqueio por inatividade configurável; desbloqueio exige nova autenticação; dados ocultos durante o bloqueio (§13.3).
- Auditoria: usuário, data/hora, operação, módulo, registro afetado, valores anterior/posterior; não editável nem apagável por usuários comuns; logs técnicos nunca guardam senhas (§13.4).
- Banco criptografado em repouso; chave no cofre do SO; **nunca** abrir o banco sem criptografia se a chave faltar (§14).
- Migrações versionadas aplicadas na inicialização, sem deixar o banco inconsistente (§16.1).
- Datas de calendário `YYYY-MM-DD`; instantes em UTC; valores monetários em centavos inteiros (§19).
- Formatos de UI: datas `DD/MM/AAAA`, horas `HH:MM`, valores `R$ 150,00` (§18).
- Menu lateral fixo e permanente; dashboard inicial após autenticação; campos obrigatórios com asterisco (§18).
- Repositório sem banco real, backups ou credenciais; branch principal `main` (§23).

## Review Focus

1. Senha com espaços, acentos ou emoji (ex.: `"  sénha 🔑 "`): deve ser aceita sem normalização que quebre o login; senha vazia ou só espaços é rejeitada.
2. Login de usuário inexistente vs. senha errada: mesma mensagem e tempo semelhante (sem enumeração de usuários); inexistente não gera bloqueio.
3. Relógio do sistema retrocedido: conta bloqueada não destrava por voltar o relógio; instantes gravados não mudam.
4. Chave ausente/corrompida no cofre com banco já existente: app para com erro claro, sem criar banco novo vazio por cima.
5. Falha no meio de uma migração ou crash entre operação e auditoria: nada parcial persiste.

---

## File Structure

```
package.json, vite.config.ts, tsconfig.json, tailwind config, components.json
src/
  main.tsx, App.tsx, routes.tsx
  lib/format.ts            # money + date formatters
  lib/api.ts               # typed wrappers over Tauri invoke
  stores/session.ts        # Zustand session/lock state
  components/layout/AppShell.tsx, Sidebar.tsx
  components/ui/*          # shadcn
  pages/Login.tsx, ChangePassword.tsx, LockScreen.tsx, Dashboard.tsx, FirstRun.tsx
src-tauri/
  Cargo.toml
  src/main.rs, lib.rs
  src/db/mod.rs            # open_encrypted, key retrieval
  src/db/keystore.rs       # KeyStore trait + OS keyring impl + in-memory fake
  src/migrations/mod.rs    # runner
  src/migrations/sql/0001_foundation.sql
  src/audit/mod.rs
  src/auth/password.rs, session.rs, permissions.rs
  src/models/user.rs
  src/repositories/user_repo.rs
  src/services/auth_service.rs, user_service.rs
  src/commands/auth.rs, app.rs
  src/error.rs             # AppError (serializable, user-safe messages)
.github/workflows/ci.yml
README.md
```

---

### Task 1: Scaffold Tauri + React + tooling

**Files:**
- Create: projeto Tauri 2 (template React + TS) na raiz; `tailwind`/`shadcn` configurados; `vitest.config.ts`; `src/lib/smoke.test.ts`
- Modify: `.gitignore` (garantir `target/`, `node_modules/`, `*.db`, `*.sqlite*`, `*.bak`, `local/`)

**Interfaces:**
- Produces: scripts npm `dev`, `build`, `test` (vitest), `tauri`; `cargo test` rodando em `src-tauri`.

- [x] **Step 1: Scaffold** com `npm create tauri-app@latest` (template React-TS) na raiz, identificador `br.com.boreal.pms`, nome do app `Boreal PMS`.
- [x] **Step 2: Instalar dependências** do stack (Tailwind, shadcn init com tema neutro, react-router-dom, @tanstack/react-query, zustand, react-hook-form, zod, @hookform/resolvers, date-fns, recharts, vitest, @testing-library/react, jsdom).
- [x] **Step 3: Teste de fumaça** `src/lib/smoke.test.ts`: `expect(1 + 1).toBe(2)`. Run: `npm test` → PASS. Run: `cargo test --manifest-path src-tauri/Cargo.toml` → PASS (0 testes ok).
- [ ] **Step 4: Verificar** `npm run tauri dev` abre a janela (fechar em seguida). _Pendente: verificação manual, sessão sem interface._
- [x] **Step 5: Commit** `chore: scaffold tauri react app`

---

### Task 2: Formatadores pt-BR (dinheiro e datas)

**Files:**
- Create: `src/lib/format.ts`
- Test: `src/lib/format.test.ts`

**Interfaces:**
- Produces:
  - `formatMoney(cents: number): string`
  - `parseMoney(input: string): number | null` (retorna centavos inteiros ou `null` se inválido)
  - `formatDate(iso: string): string` (`YYYY-MM-DD` → `DD/MM/AAAA`, sem conversão de fuso)
  - `formatTime(utcIso: string, timeZone?: string): string` (`HH:MM` local)

- [x] **Step 1: Testes que falham** (`format.test.ts`):
  - `formatMoney(15000)` → `"R$ 150,00"`; `formatMoney(120000)` → `"R$ 1.200,00"`; `formatMoney(1)` → `"R$ 0,01"`; `formatMoney(-2550)` → `"-R$ 25,50"`.
  - `parseMoney("150,75")` → `15075`; `parseMoney("R$ 1.200,00")` → `120000`; `parseMoney("0,01")` → `1`; `parseMoney("abc")` → `null`; `parseMoney("1,999")` → `null` (mais de 2 casas).
  - `formatDate("2026-03-01")` → `"01/03/2026"` com `TZ` simulado `America/Sao_Paulo` e `UTC` (mesmo resultado).
  - `formatTime("2026-10-21T15:30:00Z", "America/Sao_Paulo")` → `"12:30"`.
- [x] **Step 2: Run** `npx vitest run src/lib/format.test.ts` → FAIL (módulo inexistente).
- [x] **Step 3: Implementar** as quatro funções em `src/lib/format.ts`; `formatDate` faz split da string ISO (nunca `new Date(iso)`); `parseMoney` trabalha só com inteiros (sem `parseFloat`).
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: pt-BR money and date formatters`

---

### Task 3: Banco criptografado e cofre de chave

**Files:**
- Create: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/keystore.rs`, `src-tauri/src/error.rs`
- Modify: `src-tauri/Cargo.toml` (`rusqlite` com `bundled-sqlcipher`, `keyring`, `rand`, `thiserror`, `serde`)
- Test: módulo `#[cfg(test)]` em `db/mod.rs`

**Interfaces:**
- Produces:
  - `trait KeyStore { fn get(&self) -> Result<Option<Vec<u8>>, AppError>; fn set(&self, key: &[u8]) -> Result<(), AppError>; }`
  - `OsKeyStore` (keyring, serviço `br.com.boreal.pms`), `MemoryKeyStore` (testes)
  - `open_database(path: &Path, keys: &dyn KeyStore) -> Result<Connection, AppError>`: se o arquivo **não existe**, gera chave de 32 bytes aleatória, grava no `KeyStore` e cria o banco; se **existe** e a chave está ausente → `AppError::KeyUnavailable` (nunca cria/abre sem criptografia); ativa `PRAGMA foreign_keys=ON`, `journal_mode=WAL`.
  - `AppError` serializável com variantes `KeyUnavailable`, `Database`, `Validation(String)`, `Unauthorized`, `Forbidden`, `Conflict`, e mensagens em linguagem simples.

- [x] **Step 1: Testes que falham** em `db/mod.rs`:
  - `creates_encrypted_db_and_persists_key`: abre em tempdir com `MemoryKeyStore`; o arquivo criado **não** começa com `SQLite format 3\0`; reabrir com o mesmo store lê os dados gravados.
  - `refuses_existing_db_without_key`: cria banco, usa `MemoryKeyStore` vazio no segundo open → `Err(AppError::KeyUnavailable)` e o arquivo permanece byte a byte igual.
  - `wrong_key_fails`: troca a chave no store → `Err(AppError::Database(_))`.
- [x] **Step 2: Run** `cargo test db::` → FAIL.
- [x] **Step 3: Implementar** conforme as interfaces; chave aplicada com `PRAGMA key` em formato hex raw (`x'...'`) e validada com `SELECT count(*) FROM sqlite_master`.
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: encrypted sqlite with os keyring key store`

---

### Task 4: Runner de migrações + migração 0001

**Files:**
- Create: `src-tauri/src/migrations/mod.rs`, `src-tauri/src/migrations/sql/0001_foundation.sql`
- Test: `#[cfg(test)]` em `migrations/mod.rs`

**Interfaces:**
- Consumes: `Connection` de `open_database`.
- Produces:
  - `run_migrations(conn: &mut Connection) -> Result<u32, AppError>` (retorna versão final); tabela `schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT)`.
  - Cada migração roda em **uma transação**; falha reverte a migração e interrompe.
  - Esquema 0001: `users(id, username UNIQUE COLLATE NOCASE, display_name, password_hash, role CHECK IN ('admin','recepcao'), active, must_change_password, failed_attempts, locked_until, version, created_at, updated_at)`, `settings(key PRIMARY KEY, value)`, `audit_log(id, user_id NULL, at TEXT, module, operation, entity, entity_id, before_json, after_json, justification)` com **triggers** `BEFORE UPDATE`/`BEFORE DELETE` que fazem `RAISE(ABORT, 'audit_log é imutável')`.
  - Configurações padrão em `settings` (valores iniciais **marcados como provisórios** até a pendência §25.5): `max_failed_attempts=5`, `lockout_minutes=5`, `idle_lock_minutes=10`.

- [x] **Step 1: Testes que falham:**
  - `applies_all_and_is_idempotent`: rodar duas vezes retorna a mesma versão e não duplica linhas em `schema_migrations`.
  - `failed_migration_rolls_back`: injetar uma migração de teste com SQL inválido na 2ª instrução → nenhuma tabela dela existe e a versão permanece anterior.
  - `audit_log_is_immutable`: inserir linha, `UPDATE` e `DELETE` resultam em erro contendo `imutável`.
  - `username_unique_case_insensitive`: inserir `Ana` e `ana` → segundo falha.
- [x] **Step 2: Run** `cargo test migrations::` → FAIL.
- [x] **Step 3: Implementar** runner (migrações embutidas via `include_str!`, lista ordenada por versão) e o SQL 0001.
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: versioned migrations and foundation schema`

---

### Task 5: Serviço de auditoria transacional

**Files:**
- Create: `src-tauri/src/audit/mod.rs`
- Test: `#[cfg(test)]` em `audit/mod.rs`

**Interfaces:**
- Consumes: `rusqlite::Transaction`.
- Produces:
  - `struct AuditEntry { user_id: Option<i64>, module: &'static str, operation: &'static str, entity: &'static str, entity_id: Option<i64>, before: Option<serde_json::Value>, after: Option<serde_json::Value>, justification: Option<String> }`
  - `fn record(tx: &Transaction, entry: &AuditEntry) -> Result<(), AppError>` (grava `at` em UTC RFC3339).
  - `fn scrub(value: serde_json::Value) -> serde_json::Value`: remove chaves `password`, `password_hash`, `new_password` antes de gravar.

- [x] **Step 1: Testes que falham:**
  - `record_commits_with_transaction` e `record_rolled_back_with_transaction` (rollback da `tx` ⇒ 0 linhas em `audit_log`).
  - `scrub_removes_password_keys`: `{"password_hash":"x","name":"a"}` → `{"name":"a"}`, inclusive em objetos aninhados.
- [x] **Step 2: Run** `cargo test audit::` → FAIL.
- [x] **Step 3: Implementar** `record` (aplica `scrub` a before/after) e `scrub` recursivo.
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: transactional audit log service`

---

### Task 6: Senhas e repositório de usuários

**Files:**
- Create: `src-tauri/src/auth/password.rs`, `src-tauri/src/models/user.rs`, `src-tauri/src/repositories/user_repo.rs`
- Test: `#[cfg(test)]` nos mesmos arquivos

**Interfaces:**
- Produces:
  - `hash_password(plain: &str) -> Result<String, AppError>` (Argon2id); `verify_password(plain: &str, hash: &str) -> bool`; `validate_password_policy(plain: &str) -> Result<(), AppError>` (mínimo 8 caracteres Unicode; rejeita vazia/só espaços; **não** faz trim nem normalização).
  - `enum Role { Admin, Recepcao }`; `struct User { id: i64, username: String, display_name: String, role: Role, active: bool, must_change_password: bool, failed_attempts: u32, locked_until: Option<String>, version: i64 }`
  - `UserRepo`: `insert(tx, &NewUser) -> Result<i64>`, `find_by_username(conn, &str) -> Result<Option<(User, String /*hash*/)>>`, `find(conn, i64) -> Result<Option<User>>`, `update_password(tx, id, hash, must_change: bool, expected_version: i64) -> Result<()>` (falha com `Conflict` se a versão mudou), `record_failed_attempt(tx, id, locked_until: Option<&str>)`, `reset_attempts(tx, id)`, `count(conn) -> Result<i64>`.

- [x] **Step 1: Testes que falham:**
  - `hash_verify_roundtrip` com `"  sénha 🔑 "` (espaços preservados); verificar sem espaços ⇒ `false`.
  - `policy_rejects_short_empty_blank`: `"abc"`, `""`, `"        "` ⇒ erro; `"12345678"` ⇒ ok.
  - `update_password_conflicts_on_stale_version`.
  - `find_by_username_is_case_insensitive`.
- [x] **Step 2: Run** `cargo test auth::password user_repo` → FAIL.
- [x] **Step 3: Implementar** (argon2 com parâmetros default da crate; repositório sem regra de negócio).
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: password hashing and user repository`

---

### Task 7: Permissões e sessões

**Files:**
- Create: `src-tauri/src/auth/permissions.rs`, `src-tauri/src/auth/session.rs`
- Test: `#[cfg(test)]` nos mesmos arquivos

**Interfaces:**
- Produces:
  - `enum Permission { ManageUsers, ViewFinancialReports, ApplyDiscount, ExceedCapacity, ReverseEntry, CancellationException, ChangeReservation, ReleaseRoom, RestoreBackup, ManageSettings, … }` (todas as sensíveis citadas em §13.1) e `fn role_has(role: Role, p: Permission) -> bool` (admin: todas; recepção: nenhuma das sensíveis por padrão).
  - `struct SessionManager` (em memória, `Mutex`): `create(user_id: i64, now: Instant) -> SessionToken(String, 32 bytes aleatórios hex)`, `touch(token, now) -> Result<SessionInfo, AppError>` (`Unauthorized` se inexistente; `Locked` se ocioso além de `idle_lock`), `unlock(token, now)` (só após reautenticação feita pelo serviço), `destroy(token)`.
  - `fn require(info: &SessionInfo, p: Permission) -> Result<(), AppError>` (`Forbidden`).
  - Tempo injetado via `trait Clock { fn now(&self) -> Instant }` (idle usa relógio monotônico, não o relógio de parede).

- [x] **Step 1: Testes que falham:**
  - `recepcao_lacks_sensitive_permissions` (iterar todas as sensíveis), `admin_has_all`.
  - `idle_session_becomes_locked` com `FakeClock` avançando além do limite; `touch` retorna `Locked`; após `unlock` volta a funcionar.
  - `unknown_token_unauthorized`; `destroy_invalidates`.
  - `require_forbidden_for_recepcao`.
- [x] **Step 2: Run** `cargo test auth::` → FAIL.
- [x] **Step 3: Implementar.**
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: permissions and idle-locking sessions`

---

### Task 8: Serviço de autenticação e usuários

**Files:**
- Create: `src-tauri/src/services/auth_service.rs`, `src-tauri/src/services/user_service.rs`
- Test: `#[cfg(test)]` nos mesmos arquivos (banco em memória via `MemoryKeyStore` + tempdir)

**Interfaces:**
- Consumes: Tasks 3–7.
- Produces (`AuthService`):
  - `bootstrap_admin(&self, username: &str, display_name: &str, password: &str) -> Result<()>` — só funciona se `UserRepo::count == 0`; cria admin com `must_change_password = false` (primeira instalação); auditado.
  - `needs_bootstrap(&self) -> Result<bool>`
  - `login(&self, username: &str, password: &str) -> Result<LoginResult { token: String, user: User, must_change_password: bool }>`: erro genérico `AppError::InvalidCredentials` para usuário inexistente, senha errada ou conta inativa; conta bloqueada ⇒ `AppError::AccountLocked { until }`; ao atingir `max_failed_attempts` define `locked_until = now_utc + lockout_minutes` e audita `login_lockout`; sucesso zera tentativas e audita `login`; usuário inexistente executa um `verify_password` contra hash fictício para igualar tempo e **não** registra tentativa.
  - `change_own_password(token, current: &str, new: &str)` — exige senha atual, valida política, zera `must_change_password`.
  - `unlock_session(token, password: &str)` — reautentica e chama `SessionManager::unlock`.
  - `logout(token)`.
- Produces (`UserService`, exige `ManageUsers`): `create_user(token, NewUserInput{username, display_name, role, initial_password})` (cria com `must_change_password = true`), `reset_password(token, user_id, temp_password)` (`must_change_password = true`), `set_active(token, user_id, active)`. Tudo auditado na mesma transação, nunca gravando senha.
- Bloqueio de conta usa instante UTC gravado; relógio retrocedido não destrava (comparação `now_utc < locked_until` com `now_utc = max(relógio atual, último instante registrado em settings.last_seen_utc)`; atualizar `last_seen_utc` a cada login/ação auditada).

- [x] **Step 1: Testes que falham:**
  - `bootstrap_only_once`: segundo `bootstrap_admin` ⇒ erro.
  - `login_success_audited`; `login_wrong_password_and_unknown_user_return_same_error`.
  - `lockout_after_max_attempts`: 5 falhas ⇒ 6ª tentativa com senha correta ⇒ `AccountLocked`; auditoria contém `login_lockout`.
  - `lockout_survives_clock_rewind`: com `FakeUtc` voltando 1h após o bloqueio, continua `AccountLocked`.
  - `unknown_user_does_not_create_lock_state` (nenhuma linha afetada).
  - `must_change_flag_flow`: admin cria usuário ⇒ login retorna `must_change_password = true` ⇒ `change_own_password` ⇒ próximo login `false`.
  - `recepcao_cannot_create_or_reset_users` ⇒ `Forbidden`.
  - `audit_never_contains_password`: varrer `audit_log` por `before_json/after_json` de todas as operações e afirmar ausência da senha em claro e de `password_hash`.
  - `unlock_session_requires_correct_password`.
  - `user_creation_rolls_back_with_audit_failure`: forçar falha na gravação de auditoria ⇒ usuário não existe.
- [x] **Step 2: Run** `cargo test services::` → FAIL.
- [x] **Step 3: Implementar** serviços; cada operação de escrita abre uma `Transaction`, executa repositório + `audit::record` e só então `commit`.
- [x] **Step 4: Run** → PASS.
- [x] **Step 5: Commit** `feat: auth and user services with lockout and audit`

---

### Task 9: Comandos Tauri e inicialização do app

**Files:**
- Create: `src-tauri/src/commands/auth.rs`, `src-tauri/src/commands/app.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`
- Test: `src-tauri/tests/startup.rs` (integração)

**Interfaces:**
- Consumes: Task 8.
- Produces comandos Tauri (todos retornam `Result<T, AppError>` serializável): `needs_bootstrap() -> bool`, `bootstrap_admin(input)`, `login(username, password) -> LoginResult`, `logout(token)`, `change_own_password(token, current, new)`, `unlock_session(token, password)`, `session_status(token) -> {state: "active"|"locked", user}`, `create_user(token, input)`, `reset_password(token, user_id, temp_password)`, `get_app_info() -> {app_version, schema_version}`.
- Inicialização: caminho do banco em `app_data_dir()/boreal.db`; `OsKeyStore`; `run_migrations`; falha de chave ⇒ janela exibe tela de erro "Chave do banco indisponível" (comando `startup_status() -> "ok" | {error: "..."}`), nunca cria banco novo.

- [x] **Step 1: Testes de integração que falham** (`tests/startup.rs`): `startup_creates_db_and_runs_migrations`; `startup_with_missing_key_reports_error_and_leaves_file_untouched`; `full_flow`: bootstrap → login admin → create_user recepção → login recepção → recepção chamando `create_user` ⇒ `Forbidden`.
- [x] **Step 2: Run** `cargo test --test startup` → FAIL.
- [x] **Step 3: Implementar** comandos finos (apenas desserializar, chamar serviço, serializar) e o `setup` do Tauri com estado gerenciado (`AppState { auth: AuthService, users: UserService }`).
- [x] **Step 4: Run** → PASS. _`npm run tauri dev` sem erro: pendente (verificação manual)._
- [x] **Step 5: Commit** `feat: tauri commands and startup sequence`

---

### Task 10: Shell do frontend (login, troca de senha, bloqueio, layout)

**Files:**
- Create: `src/lib/api.ts`, `src/stores/session.ts`, `src/routes.tsx`, `src/components/layout/AppShell.tsx`, `src/components/layout/Sidebar.tsx`, `src/pages/{FirstRun,Login,ChangePassword,LockScreen,Dashboard,StartupError}.tsx`
- Modify: `src/main.tsx`, `src/App.tsx`
- Test: `src/stores/session.test.ts`, `src/pages/Login.test.tsx`, `src/components/layout/AppShell.test.tsx`

**Interfaces:**
- Consumes: comandos da Task 9 via `invoke`; `formatDate` (Task 2).
- Produces:
  - `api.ts`: funções tipadas `needsBootstrap()`, `login()`, `logout()`, `changeOwnPassword()`, `unlockSession()`, `sessionStatus()`, `getAppInfo()`; erros do backend normalizados em `ApiError { code: string; message: string }`.
  - `useSessionStore` (Zustand): `{ token, user, state: "anonymous"|"active"|"locked", mustChangePassword }`, ações `setSession`, `lock`, `clear`.
  - Rotas: `/first-run`, `/login`, `/change-password`, `/` (Dashboard dentro de `AppShell`); guarda redireciona conforme `state`/`mustChangePassword`.
  - `AppShell`: menu lateral fixo (Dashboard ativo; demais itens futuros desabilitados), cabeçalho com usuário e botão sair; `LockScreen` cobre a tela inteira e **não renderiza** children enquanto `state === "locked"`.
  - Inatividade: hook `useIdleLock(minutes)` que, após o tempo sem eventos, chama `lock()`; o backend continua sendo a autoridade (`sessionStatus` consultado a cada comando; `Locked` do backend força `lock()`).

- [x] **Step 1: Testes que falham:**
  - `session.test.ts`: `lock()` muda `state` para `locked` preservando `user`; `clear()` volta a `anonymous`.
  - `Login.test.tsx`: submissão com campos vazios mostra erros e marca obrigatórios com `*` e legenda "* campo obrigatório"; erro do backend `InvalidCredentials` exibe "Usuário ou senha inválidos"; sucesso com `mustChangePassword` navega a `/change-password`.
  - `AppShell.test.tsx`: com `state="locked"` o texto do Dashboard **não** está no DOM e o campo de senha da `LockScreen` está; com `active` o menu lateral está presente.
- [x] **Step 2: Run** `npx vitest run` → FAIL.
- [x] **Step 3: Implementar** páginas (RHF + Zod; mensagens em pt-BR), store, rotas e `AppShell`; `StartupError` exibida quando `startup_status` retorna erro.
- [ ] **Step 4: Run** `npx vitest run` → PASS; verificação manual com `npm run tauri dev`: primeira execução cria admin, login leva ao dashboard vazio, ficar ocioso trava e a senha desbloqueia. _Pendente: verificação manual (coberta por testes de componente e integração)._
- [x] **Step 5: Commit** `feat: frontend shell with login, lock screen and layout`

---

### Task 11: CI, README e checagem do repositório

**Files:**
- Create: `.github/workflows/ci.yml`, `README.md`
- Modify: `.gitignore` (se faltar algo)

**Interfaces:**
- Produces: workflow em `push`/`pull_request` para `main` com jobs: `frontend` (`npm ci`, `npm test`, `npm run build`) e `backend` (`cargo test`, `cargo clippy -- -D warnings`) em `windows-latest`.

- [x] **Step 1: Escrever** o workflow e o README (objetivo, tecnologias, pré-requisitos Rust/Node/WebView2/toolchain para SQLCipher, instalação, execução, testes, estrutura, segurança: chave no cofre do SO e aviso de perda de chave irrecuperável).
- [x] **Step 2: Verificar** localmente `npm test`, `npm run build`, `cargo test`, `cargo clippy -- -D warnings` ⇒ tudo passa.
- [x] **Step 3: Verificar** que `git ls-files` não contém `*.db`, `*.sqlite*`, `*.bak` nem credenciais.
- [x] **Step 4: Commit** `ci: github actions, readme`

---

## Self-Review

- **Cobertura da spec (escopo do Marco 0):** §2 → T1/T3/T9; §2.3 transações → T5/T8; §3.3 (IDs) fica para o marco de cadastros; §13.1–13.4 → T5–T10; §14 chave no cofre/sem fallback → T3/T9; §16.1 migrações → T4; §18 menu/asterisco → T10; §19 formatos → T2; §23 CI/README → T11. Backup, restauração e alertas ficam para marcos posteriores (declarado fora do Marco 0).
- **Consistência de tipos:** `AppError`, `KeyStore`, `Role`, `Permission`, `SessionManager`, `AuthService` usados nas tarefas seguintes com as mesmas assinaturas definidas acima.
- **Pendências da §25 refletidas:** valores de bloqueio/inatividade são provisórios e configuráveis (`settings`); lib de criptografia e ORM decididos (SQLCipher via rusqlite).
- **Proporção:** plano compacto; corpos de funções deixados ao implementador.

## Marcos seguintes (fora deste plano)

1. Cadastros: quartos/categorias, hóspedes, identificadores sequenciais.
2. Tarifas e reservas com disponibilidade.
3. Check-in/hospedagem/check-out, limpeza e manutenção.
4. Financeiro: recebimentos, devoluções, despesas, contas a pagar, fechamento.
5. Dashboard, relatórios, PDFs/CSV, documentos.
6. Backup/restauração, alertas, atualizações, E2E.
