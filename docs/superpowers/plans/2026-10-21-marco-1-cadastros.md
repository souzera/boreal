# Marco 1 — Cadastros e administração Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entregar os cadastros que o resto do sistema consome (dados da pousada, configurações, usuários, categorias, quartos, hóspedes), a base de componentes de tela, e quitar a dívida de segurança do Marco 0.

**Architecture:** Mesmas camadas do Marco 0 (`commands` → `services` → `repositories`). Cada entidade tem migração própria, serviço com permissão verificada no backend, auditoria na mesma transação, controle otimista por `version` e tela React sobre `DataTable`/formulários RHF+Zod. Permissões extras por usuário são carregadas na sessão como máscara de bits.

**Tech Stack:** o do Marco 0 + `base64` (logotipo), `unicode-normalization` (comparação de nomes).

**Spec:** `local/spec.md` (§3, §13.1–13.2, §16.2, §17, §18, §19). Roteiro: `local/plano-completo.md` §5.

**Branch:** `marco-1-cadastros`, criada a partir de `marco-0-fundacao` (a integração do Marco 0 em `main` é decisão pendente e independente).

## Status — Marco 1 implementado e revisado (2026-10-21)

Branch `marco-1-cadastros` (a partir de `marco-0-fundacao`); 15 tarefas implementadas com TDD.

- **Testes na última execução:** 108 unitários + 4 de integração (Rust), 128 (Vitest); `clippy -D warnings` e `vite build` sem erros.
- **Pendente de verificação manual:** `npm run tauri dev` (primeira execução → login → bloqueio por inatividade → telas de Usuários, Configurações, Quartos, Hóspedes, busca; console sem violações de CSP). A sessão de implementação não tinha interface gráfica.
- **Revisão final da branch:** 1 Crítico e 6 Importantes, todos corrigidos com teste vermelho → verde (ver abaixo); 11 menores adiados.

### Desvios do plano

- Tarefa 5: sem `sequence_service.rs`; `sequence_repo::{next, format_code}` bastam.
- Tarefa 6: `Permission::from_str` virou `Permission::parse` (aviso do clippy); comandos `grantable_permissions` e `set_extra_permissions`.
- Tarefa 7: o app passou a usar roteador de dados (`createHashRouter` + `RouterProvider`), exigido pelo aviso de alterações não salvas (`useBlocker`). `src/test/render.tsx` (`renderPage`) é o auxiliar de testes.
- Tarefa 8: `settings_version` controla edição concorrente das configurações; `get_session_policy` está aberto a qualquer sessão.
- Tarefa 9: `get_app_info` devolve `last_migration_at`; ociosidade da interface vem do backend (`useSessionPolicy`).
- Tarefa 12: sem botão de exclusão nas telas de catálogo (o backend tem `delete_*`, testado); o caminho do usuário é desativar.
- Tarefa 14: a tela de hóspedes usa `leaveTo` para navegar só depois de desligar o aviso de alterações não salvas.
- Tarefa 15: busca global só consulta hóspedes por enquanto.
- Migrações: `0002` a `0005` (o roteiro previa uma só); as dos próximos marcos começam em `0006`.
- Ambiente: `npm install --legacy-peer-deps`; Strawberry Perl antes do Perl do Git no PATH.

### Correções da revisão final

- **C1** diálogos em portal ficavam visíveis sobre a tela de bloqueio → `LockContext`: `DialogContent` não é exibido com a sessão bloqueada/encerrada.
- **I1** fim de sessão pelo backend desmontava a tela e perdia edições → novo estado `expired` com reentrada na própria tela; **Sair** passou a ser navegação para `/login` (respeita alterações não salvas).
- **I2** diálogos de formulário fechavam sem checar alterações → `useGuardedClose`.
- **I3** máscara de telefone com +55; **I4** clique duplo em "Criar outro cadastro"; **I5** tempo de inatividade da interface não atualizava (agora no store); **I6** cache de consultas esvaziado ao sair/trocar de pessoa.

### Pendências adiadas

- Verificação manual do app (acima).
- Fechar a janela do aplicativo com formulário alterado ainda não é interceptado (`onCloseRequested` do Tauri, spec §21): planejado para o Marco 8.
- Menores da revisão: edição de hóspede sem checagem de duplicidade; rótulo "Confirmar desativação" na reativação; "nome parecido" × nome igual; CPF colado com mais de 11 dígitos é truncado; busca por dígitos também casa telefone/CPF; `NOCASE` só ASCII; erro de logotipo renomeado; sem "Recarregar" após conflito na pousada; aviso de logout imediato no diálogo de permissões; semântica de linhas clicáveis no `DataTable`.
- Mutex envenenável (`lock().unwrap()`): tratar no Marco 2 antes dos fluxos longos.
- `IDLE_MINUTES` padrão (10) usado na interface até o backend responder.

---

## Global Constraints

- Regras críticas, permissões e validações são aplicadas e revalidadas no backend; o frontend só melhora a experiência (spec §2.2, §13.1).
- Operações com várias gravações usam transação; auditoria na **mesma** transação (spec §2.3, §13.4).
- Edição de registro compara `version`; versão antiga ⇒ `AppError::Conflict`, sem sobrescrita silenciosa (spec §2.3).
- Cadastros com histórico nunca são apagados fisicamente: `active = 0`. Exclusão física só sem dependências, com `ManageCatalog` e confirmação (spec §3.1, §3.3).
- Hóspede: obrigatórios nome completo e telefone; opcionais CPF, e-mail, endereço; alerta de duplicidade por nome, telefone ou CPF, com conferência e opção de criar outro cadastro (spec §3.2).
- IDs internos numéricos + código sequencial visível, gerado no backend, sem reuso de números confirmados (spec §3.3).
- Datas de calendário `YYYY-MM-DD`; instantes UTC RFC 3339; valores em centavos; exibição `DD/MM/AAAA`, `HH:MM`, `R$ 150,00` (spec §18, §19).
- Campos obrigatórios com `*` e legenda; validação ao salvar com campos inválidos indicados; confirmação ao sair com alterações não salvas (spec §18).
- Relógios injetados (`UtcClock`, `Clock`); proibido `Utc::now()`/`Instant::now()` fora de `SystemUtc`/`SystemClock`.
- Logs, erros e auditoria nunca carregam senhas, tokens nem dados sensíveis desnecessários; CPF só mascarado na auditoria.
- Senha: mínimo de 6 caracteres, sem normalização (decisão do dono, Marco 0).
- Toda mudança relevante atualiza `CHANGELOG.md` (`Unreleased`), `docs/usuario/`, `docs/desenvolvimento/` e este plano (regra do `CLAUDE.md`).
- Ambiente: Strawberry Perl antes do Perl do Git no PATH para qualquer `cargo`; `npm install --legacy-peer-deps`.

## Review Focus

1. **CPF inválido passa:** `111.111.111-11`, 10 dígitos, letras, máscara parcial; CPF válido é guardado só com dígitos e exibido com máscara.
2. **Duplicidade perdida por forma:** `"José  da Silva"` × `"jose da silva"`, telefone `(11) 99999-0000` × `11999990000`; a conferência deve achar ambos.
3. **Edição concorrente:** duas edições do mesmo quarto/hóspede/usuário/categoria com a mesma `version`; a segunda recebe `Conflict` e nada é perdido.
4. **Configuração inválida trava o sistema:** ociosidade `0`, horário `25:00`, tentativas `0`; devem ser recusadas (ninguém pode se trancar para fora).
5. **Permissão extra:** revogada, deve valer na hora (sessões do usuário encerradas); `ManageUsers`/`ManageSettings`/`ManageCatalog`/`RestoreBackup`/`ViewFinancialReports` nunca são concedíveis; recepção sem permissão não consegue chamar o comando mesmo conhecendo o nome.
6. **Logotipo hostil:** arquivo enorme, não-imagem ou renomeado; deve ser recusado sem derrubar o app.

---

## File Structure

```
src-tauri/src/
  migrations/sql/0002_sequences_permissions.sql   # sequences, user_permissions
  migrations/sql/0003_pousada_settings.sql        # pousada, defaults de configurações
  migrations/sql/0004_catalog.sql                 # room_categories, rooms
  migrations/sql/0005_guests.sql                  # guests
  repositories/sequence_repo.rs, permission_repo.rs, pousada_repo.rs, catalog_repo.rs, guest_repo.rs, user_repo.rs (+list)
  services/sequence_service.rs, settings_service.rs, pousada_service.rs, catalog_service.rs, guest_service.rs
  models/settings.rs, pousada.rs, catalog.rs, guest.rs, page.rs
  domain/cpf.rs, phone.rs, text.rs                # validação/normalização puras
  commands/{settings,pousada,users,catalog,guests}.rs
src/
  components/data-table/DataTable.tsx, ConfirmDialog.tsx, PageHeader.tsx, UnsavedChangesGuard.tsx
  components/layout/LockGate.tsx
  hooks/useUnsavedChanges.ts
  lib/format.ts (+ cpf, telefone), lib/api.ts (+ comandos)
  pages/settings/*, pages/users/*, pages/catalog/*, pages/guests/*, pages/Search.tsx
CHANGELOG.md, docs/usuario/*.md, docs/desenvolvimento/*.md
```

---

### Task 1: Documentação-base e endurecimento do Tauri

**Files:**
- Create: `CHANGELOG.md`, `docs/usuario/README.md`, `docs/desenvolvimento/README.md`
- Modify: `src-tauri/tauri.conf.json` (CSP), `src-tauri/Cargo.toml` (metadados; remover `tauri-plugin-opener`), `src-tauri/capabilities/default.json` (remover `opener:default`), `src-tauri/src/lib.rs` (remover `.plugin(tauri_plugin_opener::init())`), `package.json` (remover `@tauri-apps/plugin-opener`)
- Modify: `CLAUDE.md` (apontar a spec em `local/spec.md`)

**Interfaces:**
- Produces: `CHANGELOG.md` com seção `## [Unreleased]` e subtítulos `Added/Changed/Fixed/Removed/Security`; pastas de documentação existentes para as tarefas seguintes.

- [x] **Step 1:** Criar `CHANGELOG.md` (Keep a Changelog, seção `Unreleased` com o resumo do Marco 0 em `Added`/`Security`), `docs/usuario/README.md` (índice) e `docs/desenvolvimento/README.md` (índice + decisões do Marco 0: chave no cofre, camadas, sessões, auditoria).
- [x] **Step 2:** `tauri.conf.json` → `app.security.csp` = `"default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"` e `devCsp` acrescentando `ws://localhost:1420 http://localhost:1420`. Remover o plugin `opener` (Rust, capability e dependência npm). `Cargo.toml`: `description = "Sistema de gestão de pousada (PMS)"`, `authors = ["Boreal"]`.
- [x] **Step 3: Run** `cargo build --manifest-path src-tauri/Cargo.toml` e `npm run build`. Expected: sucesso, sem referências a `opener` (`git grep -n opener -- src src-tauri` vazio).
- [ ] **Step 4: Verificação manual** (usuário ou sessão com tela): `npm run tauri dev`; primeira execução cria admin → login → dashboard; ficar ocioso trava; senha desbloqueia; console sem violações de CSP. Registrar o resultado na seção Status do plano.
- [x] **Step 5: Commit** `chore: docs scaffolding, csp, remove unused opener plugin`

---

### Task 2: Relógio injetado em auditoria e repositórios

**Files:**
- Modify: `src-tauri/src/audit/mod.rs`, `src-tauri/src/repositories/user_repo.rs`, `src-tauri/src/services/{auth_service,user_service}.rs`
- Test: testes existentes + novos em `audit/mod.rs` e `services/auth_service.rs`

**Interfaces:**
- Produces: `AuditEntry` ganha o campo `at: DateTime<Utc>`; `audit::record(tx, entry)` grava `entry.at`. Funções de `user_repo` que gravam data recebem `at: DateTime<Utc>` como último parâmetro (`insert`, `set_active`, `update_password`, `record_failed_attempt`, `reset_attempts`). Serviços passam `self.ctx.utc.now_utc()`.

- [x] **Step 1: Teste que falha** `audit_uses_injected_time`: com `FakeUtc` em 2026-10-21T12:00:00Z, após `login` a linha de `audit_log` tem `at = '2026-10-21T12:00:00Z'`; e `users.updated_at` da troca de senha idem.
- [x] **Step 2: Run** `cargo test audit_uses_injected_time` → FAIL (hoje grava a hora real).
- [x] **Step 3: Implementar** a mudança de assinatura acima; atualizar todos os chamadores.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` → PASS (43+ testes).
- [x] **Step 5: Commit** `refactor: inject clock into audit and user repository`

---

### Task 3: Endurecimento do backend de autenticação

**Files:**
- Modify: `src-tauri/src/services/{auth_service,user_service}.rs`, `src-tauri/src/error.rs`, `src-tauri/src/auth/session.rs`
- Test: `services/auth_service.rs`, `services/user_service.rs`, `error.rs`

**Interfaces:**
- Produces:
  - `UserService::reset_password(token, user_id, temp_password, expected_version: i64)` — versão vinda do cliente; antiga ⇒ `Conflict`.
  - `SessionManager::destroy_user_except(user_id: i64, keep_token: &str)`.
  - `AppError::Database(String)` com `Display` fixo: "Não foi possível concluir a operação. Tente novamente." (detalhe só em `Debug`).
  - `Debug` manual de `NewUserInput` e `LoginResult` que redigem `initial_password`/`token`.

- [x] **Step 1: Testes que falham**:
  - `unlock_after_lock_expiry_does_not_relock_on_one_typo`: 5 erros no login travam; após 6 min uma senha errada em `unlock_session` ⇒ `InvalidCredentials` (não `AccountLocked`).
  - `change_own_password_counts_wrong_current_and_locks`: 5 senhas atuais erradas bloqueiam a conta.
  - `change_own_password_rejects_same_password` e `change_own_password_ends_other_sessions` (a sessão atual continua; outra do mesmo usuário vira `Unauthorized`).
  - `reset_password_rejects_stale_version` (`Conflict`) e `reset_password_ends_user_sessions`.
  - `debug_output_never_contains_secrets`: `format!("{:?}")` de `NewUserInput` e `LoginResult` não contém a senha/token.
  - `database_error_message_is_plain`: `AppError::Database("UNIQUE constraint failed: x".into()).to_string()` não contém `UNIQUE`.
- [x] **Step 2: Run** `cargo test auth_service user_service error` → FAIL.
- [x] **Step 3: Implementar**: `unlock_session` zera tentativas quando `locked_until` expirou (como o `login`); `change_own_password` usa `register_failure` na senha atual errada, recusa `new == current` e chama `destroy_user_except`; `reset_password` usa a versão informada e encerra sessões do usuário; forçar `DUMMY_HASH` no `AuthService::new`; `Debug` manuais.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` e `cargo clippy ... -D warnings` → verdes.
- [x] **Step 5: Commit** `fix: harden auth flows (unlock, password change, reset version, debug redaction)`

---

### Task 4: Tela de bloqueio uniforme e logout real (frontend)

**Files:**
- Create: `src/components/layout/LockGate.tsx`
- Modify: `src/components/layout/AppShell.tsx`, `src/pages/LockScreen.tsx`, `src/routes.tsx`, `src/pages/ChangePassword.tsx`
- Test: `src/components/layout/LockGate.test.tsx`, ajustar `LockScreen`/`ChangePassword` tests

**Interfaces:**
- Produces: `LockGate({ children })` — renderiza `children` ocultos/inertes + `LockScreen` quando `state === "locked"`; usado por `AppShell` e pela rota `/change-password`. "Sair" da tela de bloqueio chama `api.logout(token)` e depois `clear()`.

- [x] **Step 1: Testes que falham**: `LockGate` bloqueado oculta filhos e mostra campo de senha, desbloqueado mostra filhos (estado preservado); `ChangePassword` dentro de `LockGate` fica oculta quando bloqueado; `LockScreen` "Sair" chama `logout` com o token (mock de `api`).
- [x] **Step 2: Run** `npx vitest run src/components/layout` → FAIL.
- [x] **Step 3: Implementar** `LockGate` (extraído do `AppShell`) e usá-lo nos dois lugares; `LockScreen.onLogout = async () => { try { await logout(token) } finally { clear() } }`.
- [x] **Step 4: Run** `npx vitest run` → PASS.
- [x] **Step 5: Commit** `fix: uniform lock gate and real logout from lock screen`

---

### Task 5: Sequências (códigos visíveis)

**Files:**
- Create: `src-tauri/src/migrations/sql/0002_sequences_permissions.sql`, `src-tauri/src/repositories/sequence_repo.rs`, `src-tauri/src/services/sequence_service.rs`
- Modify: `src-tauri/src/migrations/mod.rs` (registrar 0002), `repositories/mod.rs`, `services/mod.rs`
- Test: `sequence_repo.rs`

**Interfaces:**
- Produces: tabela `sequences(name TEXT PRIMARY KEY, next_value INTEGER NOT NULL)` com linhas iniciais `guest`, `reservation`, `stay`, `payment`, `refund` em 1; `sequence_repo::next(tx: &Transaction, name: &str) -> Result<i64, AppError>` (lê e incrementa na mesma transação; nome desconhecido ⇒ `Database`); `format_code(prefix: &str, n: i64) -> String` ⇒ `"HOS-000042"`.
- A migração 0002 também cria `user_permissions` (usada na Task 6).

- [x] **Step 1: Testes que falham**: `next_returns_consecutive_numbers`; `rolled_back_transaction_does_not_consume_number`; `committed_numbers_are_never_reused` (commit, nova transação, número seguinte); `unknown_sequence_fails`; `format_code_pads_six_digits` (`format_code("HOS", 42) == "HOS-000042"`).
- [x] **Step 2: Run** `cargo test sequence` → FAIL.
- [x] **Step 3: Implementar** `UPDATE sequences SET next_value = next_value + 1 WHERE name = ?1 RETURNING next_value - 1`.
- [x] **Step 4: Run** → PASS; `migrations` tests ainda verdes.
- [x] **Step 5: Commit** `feat: sequences for visible codes`

---

### Task 6: Permissões extras por usuário

**Files:**
- Create: `src-tauri/src/repositories/permission_repo.rs`
- Modify: `src-tauri/src/auth/permissions.rs`, `src-tauri/src/auth/session.rs`, `src-tauri/src/services/{auth_service,user_service}.rs`, `src-tauri/src/commands/auth.rs` (+ `commands/users.rs`), `src-tauri/src/lib.rs`
- Test: `permissions.rs`, `session.rs`, `user_service.rs`

**Interfaces:**
- Produces:
  - `Permission` ganha `ManageCatalog`; `Permission::as_str(self) -> &'static str`, `Permission::from_str(&str) -> Option<Permission>`, `Permission::grantable() -> &'static [Permission]` = `ApplyDiscount, ExceedCapacity, ReverseEntry, CancellationException, ChangeReservation, ReleaseRoom`.
  - `PermissionSet(u32)` (`Copy`): `empty()`, `with(Permission)`, `contains(Permission) -> bool`.
  - `SessionInfo { user_id, role, extra: PermissionSet }`; `SessionManager::create_with(user_id, role, extra, now)` (`create` = extras vazias); `require` aceita `role_has(role, p) || extra.contains(p)`.
  - `permission_repo::list(conn, user_id) -> Vec<Permission>`; `replace(tx, user_id, &[Permission], granted_by, at)`.
  - `UserService::set_extra_permissions(token, user_id, perms: &[Permission])` — exige `ManageUsers`; recusa permissões fora de `grantable()` (`Validation`) e usuário admin; audita antes/depois; encerra as sessões do usuário.
  - O login carrega as extras na sessão.

- [x] **Step 1: Testes que falham**: `grantable_excludes_admin_only_permissions`; `permission_set_roundtrip`; `extra_permission_allows_recepcao_operation` (recepção com `ApplyDiscount` passa em `require`, sem ela `Forbidden`); `revoking_extra_permission_ends_sessions`; `cannot_grant_manage_users`; `set_extra_permissions_requires_manage_users`; `set_extra_permissions_is_audited`.
- [x] **Step 2: Run** `cargo test permission` → FAIL.
- [x] **Step 3: Implementar** (máscara de bits pela posição em `Permission::ALL`).
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` → PASS.
- [x] **Step 5: Commit** `feat: per-user extra permissions loaded into the session`

---

### Task 7: Componentes base de interface

**Files:**
- Create: `src/components/data-table/DataTable.tsx`, `src/components/ConfirmDialog.tsx`, `src/components/PageHeader.tsx`, `src/hooks/useUnsavedChanges.ts`, `src/components/UnsavedChangesGuard.tsx`
- Modify: `src/lib/format.ts` (+ testes), `src/components/layout/Sidebar.tsx` (itens finais)
- Test: `DataTable.test.tsx`, `useUnsavedChanges.test.tsx`, `ConfirmDialog.test.tsx`, `format.test.ts`

**Interfaces:**
- Produces:
  - `DataTable<T>({ columns, rows, total, page, pageSize, onPageChange, sort, onSortChange, filters?, emptyMessage, onRowClick? })` — ordenação/paginação controladas pelo pai (a consulta é do backend); `ColumnDef<T> = { key: string; header: string; sortable?: boolean; render: (row: T) => ReactNode }`.
  - `ConfirmDialog({ open, title, description, confirmLabel, onConfirm, onCancel, destructive? })`.
  - `useUnsavedChanges(isDirty: boolean)` — bloqueia navegação interna (React Router `useBlocker`) e `beforeunload`; `UnsavedChangesGuard` mostra o diálogo **Salvar / Descartar / Continuar editando**.
  - `formatCpf(digits: string): string` (`"12345678909"` → `"123.456.789-09"`), `onlyDigits(s)`, `formatPhone(digits: string): string` (`"11999990000"` → `"(11) 99999-0000"`, 10 dígitos → `(11) 9999-0000`).

- [x] **Step 1: Testes que falham**: DataTable renderiza cabeçalhos/linhas, mostra `emptyMessage`, clicar no cabeçalho ordenável chama `onSortChange`, paginação chama `onPageChange`; `useUnsavedChanges` com `isDirty=true` mostra o diálogo ao navegar e "Continuar editando" mantém a página; `ConfirmDialog` chama os callbacks; formatadores com os exemplos acima (e entradas parciais sem lançar).
- [x] **Step 2: Run** `npx vitest run` → FAIL.
- [x] **Step 3: Implementar** (shadcn `table`, `dialog`, `select` via `npx shadcn@latest add table dialog select checkbox textarea badge`).
- [x] **Step 4: Run** `npx vitest run` e `npm run build` → PASS.
- [x] **Step 5: Commit** `feat: data table, unsaved-changes guard, confirm dialog and formatters`

---

### Task 8: Configurações e dados da pousada (backend)

**Files:**
- Create: `src-tauri/src/migrations/sql/0003_pousada_settings.sql`, `models/settings.rs`, `models/pousada.rs`, `repositories/pousada_repo.rs`, `services/settings_service.rs`, `services/pousada_service.rs`, `commands/settings.rs`, `commands/pousada.rs`
- Modify: `repositories/settings_repo.rs` (+`set`), `services/mod.rs`, `lib.rs`, `Cargo.toml` (`base64`)
- Test: `settings_service.rs`, `pousada_service.rs`

**Interfaces:**
- Produces:
  - `AppSettings { max_failed_attempts: u32, lockout_minutes: u32, idle_lock_minutes: u32, default_check_in_time: String, default_check_out_time: String, late_checkout_tolerance_minutes: u32, no_show_tolerance_minutes: u32, cleaning_interval_minutes: u32 }`. Padrões **provisórios** (pendência §25.5): `5, 5, 10, "14:00", "12:00", 30, 60, 60`.
  - `SettingsService::get(token) -> AppSettings` (qualquer sessão), `update(token, AppSettings, expected_version: i64) -> ()` exige `ManageSettings`; valida faixas (tentativas 3–20; bloqueio 1–1440; ociosidade 1–480; horários `HH:MM` válidos; tolerâncias 0–600; limpeza 0–600), audita antes/depois, chama `sessions.set_idle_lock`. `get_session_policy(token) -> { idle_lock_minutes }`.
  - `PousadaService::get(token) -> Pousada { name, address, phone, has_logo, version }`, `get_logo(token) -> Option<{ mime, base64 }>`, `update(token, UpdatePousada { name, address, phone, logo: LogoChange }, expected_version)` exige `ManageSettings`. `LogoChange = Keep | Remove | Set { mime, base64 }`; aceita só `image/png` e `image/jpeg` com assinatura de arquivo válida e ≤ 512 KB decodificado.
  - Tabela `pousada` de linha única (`id = 1 CHECK`), `version`.

- [x] **Step 1: Testes que falham**: `update_settings_applies_idle_lock_immediately` (sessão passa a travar no novo limite); `rejects_invalid_values` (ociosidade 0, tentativas 0, horário `25:00`, `9:7`); `settings_update_requires_manage_settings`; `settings_update_audited_with_before_after`; `stale_settings_version_conflicts`; `get_session_policy_available_to_recepcao`; pousada: `rejects_non_image_logo` (bytes de texto com mime `image/png`), `rejects_oversized_logo`, `accepts_png_and_roundtrips`, `remove_logo`, `recepcao_can_read_but_not_update`.
- [x] **Step 2: Run** `cargo test settings pousada` → FAIL.
- [x] **Step 3: Implementar**; validar assinatura PNG (`89 50 4E 47`) / JPEG (`FF D8 FF`); `Debug` sem o conteúdo do logo.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` → PASS.
- [x] **Step 5: Commit** `feat: settings and pousada data (backend)`

---

### Task 9: Configurações e pousada (interface) e ociosidade dinâmica

**Files:**
- Create: `src/pages/settings/SettingsPage.tsx`, `src/pages/settings/PousadaForm.tsx`, `src/pages/settings/SystemForm.tsx`, `src/pages/settings/AboutCard.tsx`
- Modify: `src/lib/api.ts`, `src/routes.tsx`, `src/components/layout/AppShell.tsx` (remover `IDLE_MINUTES`; usar `session policy`), `src/components/layout/Sidebar.tsx`
- Test: `SettingsPage.test.tsx`, `AppShell.test.tsx`

**Interfaces:**
- Consumes: comandos da Task 8; `getAppInfo()`.
- Produces: rota `/configuracoes` (só admin visível; backend é a autoridade); `AboutCard` mostra versão do app, versão do esquema e data da última migração aplicada (`get_app_info` passa a devolver `last_migration_at`); `AppShell` lê `idle_lock_minutes` de `get_session_policy` ao iniciar a sessão.

- [x] **Step 1: Testes que falham**: formulário valida (`*` e legenda) e mostra o erro de ociosidade 0; salvar chama `updateSettings` e mostra confirmação "Configurações salvas"; recepção não vê o item de menu; `AppShell` usa o minuto vindo do backend (mock) para travar; sair com formulário sujo mostra o diálogo de alterações não salvas.
- [x] **Step 2: Run** `npx vitest run src/pages/settings src/components/layout` → FAIL.
- [x] **Step 3: Implementar** (e `get_app_info` com `last_migration_at` lido de `schema_migrations`; teste Rust correspondente `app_info_includes_last_migration`).
- [x] **Step 4: Run** `npx vitest run`, `cargo test` → PASS.
- [x] **Step 5: Commit** `feat: settings and pousada screens, dynamic idle lock`

---

### Task 10: Usuários (backend de listagem/edição e interface)

**Files:**
- Modify: `repositories/user_repo.rs` (+`list`, `set_role`), `services/user_service.rs`, `commands/users.rs` (+registro em `lib.rs`), `src/lib/api.ts`
- Create: `src/pages/users/UsersPage.tsx`, `src/pages/users/UserForm.tsx`, `src/pages/users/PermissionsDialog.tsx`
- Test: `user_service.rs`, `UsersPage.test.tsx`

**Interfaces:**
- Produces:
  - `UserService::list(token, UserQuery { text: Option<String>, active: Option<bool>, page: u32, page_size: u32 }) -> Page<UserRow>` (`ManageUsers`); `UserRow` inclui `extra_permissions`.
  - `UserService::set_role(token, user_id, role, expected_version)`: não permite remover o **último administrador ativo** nem rebaixar a si mesmo.
  - `UserService::set_active` não permite desativar o último administrador ativo.
  - Comandos Tauri: `list_users`, `set_user_role`, `set_user_active`, `set_extra_permissions` (e os já existentes `create_user`, `reset_password` com `expected_version`).
  - `Page<T> { items: Vec<T>, total: u64, page: u32, page_size: u32 }` em `models/page.rs` (reutilizado nas próximas tarefas).

- [x] **Step 1: Testes que falham** (Rust): `list_requires_manage_users`; `list_filters_and_paginates`; `cannot_remove_last_active_admin` (rebaixar e desativar); `set_role_stale_version_conflicts`; `role_change_is_audited`. (Vitest): lista mostra usuários com paginação; criar usuário exibe senha temporária exigida; diálogo de permissões só oferece as `grantable()`; desativar pede confirmação.
- [x] **Step 2: Run** `cargo test user_service` e `npx vitest run src/pages/users` → FAIL.
- [x] **Step 3: Implementar** consulta paginada com `LIMIT/OFFSET` e `COUNT(*)`; ordenação restrita a colunas permitidas (lista fixa, nunca interpolar entrada do usuário).
- [x] **Step 4: Run** suítes completas → PASS.
- [x] **Step 5: Commit** `feat: user management screens and safeguards`

---

### Task 11: Catálogo — categorias e quartos (backend)

**Files:**
- Create: `migrations/sql/0004_catalog.sql`, `models/catalog.rs`, `repositories/catalog_repo.rs`, `services/catalog_service.rs`, `commands/catalog.rs`
- Modify: `migrations/mod.rs`, `lib.rs`, `services/mod.rs`
- Test: `catalog_service.rs`

**Interfaces:**
- Produces (todas as escritas exigem `ManageCatalog`; leituras, qualquer sessão):
  - `Category { id, name, description, active, version }`, `Room { id, code, category_id, category_name, max_capacity, description, notes, active, version }`.
  - `CatalogService::create_category(token, NewCategory{name, description}) -> i64`, `update_category(token, id, UpdateCategory, expected_version)`, `set_category_active(token, id, active)`, `delete_category(token, id)`, `list_categories(token, active_only: bool) -> Vec<Category>`.
  - Idem para quartos: `create_room(token, NewRoom{code, category_id, max_capacity, description, notes})`, `update_room(...)`, `set_room_active`, `delete_room`, `list_rooms(token, RoomQuery{text, category_id, active, page, page_size}) -> Page<Room>`.
  - Regras: `code`/`name` únicos sem diferenciar caixa (`Validation("Já existe um quarto com este código.")`); `max_capacity` ≥ 1; quarto exige categoria **ativa**; não desativar categoria com quartos ativos sem aviso (retorna `Validation` listando a contagem); `delete_*` só sem referências (FK `RESTRICT` mapeada para `Validation("... tem histórico; desative em vez de apagar")`).

- [x] **Step 1: Testes que falham**: duplicidade de nome/código em caixas diferentes; capacidade 0 recusada; recepção recebe `Forbidden` em toda escrita e consegue listar; `update_room_stale_version_conflicts`; `deactivate_category_with_active_rooms_is_refused`; `delete_room_with_dependency_fails_and_keeps_row` (tabela de teste `room_refs_test` com FK para `rooms`); `delete_unused_room_succeeds_and_is_audited`; auditoria de criar/editar/desativar com antes/depois.
- [x] **Step 2: Run** `cargo test catalog` → FAIL.
- [x] **Step 3: Implementar** (`PRAGMA foreign_keys=ON` já ativo).
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` e `clippy` → verdes.
- [x] **Step 5: Commit** `feat: room categories and rooms (backend)`

---

### Task 12: Catálogo — interface

**Files:**
- Create: `src/pages/catalog/CategoriesPage.tsx`, `RoomsPage.tsx`, `RoomForm.tsx`, `CategoryForm.tsx`
- Modify: `src/lib/api.ts`, `src/routes.tsx`, `Sidebar.tsx`
- Test: `RoomsPage.test.tsx`, `CategoriesPage.test.tsx`

**Interfaces:**
- Consumes: Task 7 (`DataTable`, `ConfirmDialog`, `useUnsavedChanges`), Task 11.
- Produces: rotas `/quartos` e `/quartos/categorias`; botões de escrita só visíveis com permissão (o backend segue recusando).

- [x] **Step 1: Testes que falham**: lista de quartos com filtro por categoria e busca; criar quarto mostra erro de código duplicado vindo do backend sem perder o que foi digitado; capacidade 0 mostra erro de campo; desativar pede confirmação e exibe o resultado ("Quarto 101 desativado"); recepção não vê "Novo quarto".
- [x] **Step 2: Run** `npx vitest run src/pages/catalog` → FAIL.
- [x] **Step 3: Implementar** com RHF+Zod; erros do backend (`validation`, `conflict`) tratados sem apagar o formulário; `Conflict` oferece "Recarregar".
- [x] **Step 4: Run** `npx vitest run` e `npm run build` → PASS.
- [x] **Step 5: Commit** `feat: rooms and categories screens`

---

### Task 13: Hóspedes (backend)

**Files:**
- Create: `migrations/sql/0005_guests.sql`, `domain/{mod,cpf,phone,text}.rs`, `models/guest.rs`, `repositories/guest_repo.rs`, `services/guest_service.rs`, `commands/guests.rs`
- Modify: `Cargo.toml` (`unicode-normalization`), `lib.rs`, `services/mod.rs`
- Test: `domain/cpf.rs`, `domain/phone.rs`, `domain/text.rs`, `guest_service.rs`

**Interfaces:**
- Produces:
  - `domain::cpf::normalize(input: &str) -> Option<String>` (11 dígitos válidos pelos dígitos verificadores, rejeita sequências repetidas) e `mask(cpf: &str) -> String` (`***.***.***-09`).
  - `domain::phone::normalize(input: &str) -> Option<String>` (somente dígitos, 10 a 13; remove `+55` inicial quando houver 12–13 dígitos).
  - `domain::text::fold(input: &str) -> String` (minúsculas, sem acentos, espaços colapsados) — chave de comparação de nomes.
  - `Guest { id, code, full_name, phone, cpf: Option<String>, email, address, active, version }`; código `HOS-000001` via sequência `guest`.
  - `GuestService::find_possible_duplicates(token, &GuestInput) -> Vec<DuplicateMatch { guest: Guest, reasons: Vec<DuplicateReason> }>` com `DuplicateReason = Name | Phone | Cpf`.
  - `GuestService::create(token, GuestInput, confirm_duplicates: bool) -> CreateGuestOutcome` = `Created { id, code } | Duplicates(Vec<DuplicateMatch>)`; sem `confirm_duplicates` e com achados, **nada é gravado**.
  - `update(token, id, GuestInput, expected_version)`, `set_active(token, id, active)` (`ManageCatalog`), `get(token, id)`, `list(token, GuestQuery { text, active, page, page_size }) -> Page<Guest>` (busca por nome sem acento, telefone, CPF, código).
  - Auditoria guarda CPF **mascarado** e não grava e-mail/endereço completos (apenas os nomes dos campos alterados).

- [x] **Step 1: Testes que falham**: CPF: válido `529.982.247-25` aceito (guarda `52998224725`), `111.111.111-11` e `52998224724` recusados; telefone: `(11) 99999-0000` e `+55 11 99999-0000` ambos viram `11999990000`; `123` recusado; `fold("  José  da  SILVA ") == "jose da silva"`; serviço: nome e telefone obrigatórios; duplicidade por nome acentuado, por telefone formatado diferente e por CPF; criação sem confirmar com duplicados não grava; confirmando, cria outro cadastro; `update` com versão antiga ⇒ `Conflict`; recepção cria e edita, mas não desativa; busca acha "jose" ao digitar "José"; códigos consecutivos sem reuso; auditoria não contém o CPF completo (`audit_masks_cpf`).
- [x] **Step 2: Run** `cargo test domain guest` → FAIL.
- [x] **Step 3: Implementar**; duplicidade por nome compara `fold` completo; consulta usa colunas normalizadas persistidas (`name_key`, `phone_digits`, `cpf`) com índices.
- [x] **Step 4: Run** `cargo test --manifest-path src-tauri/Cargo.toml` e `clippy` → verdes.
- [x] **Step 5: Commit** `feat: guests with duplicate detection (backend)`

---

### Task 14: Hóspedes (interface)

**Files:**
- Create: `src/pages/guests/GuestsPage.tsx`, `GuestForm.tsx`, `DuplicatesDialog.tsx`
- Modify: `src/lib/api.ts`, `src/routes.tsx`, `Sidebar.tsx`, `src/lib/format.ts`
- Test: `GuestsPage.test.tsx`, `GuestForm.test.tsx`

**Interfaces:**
- Consumes: Tasks 7 e 13.
- Produces: rotas `/hospedes`, `/hospedes/novo`, `/hospedes/:id`; `DuplicatesDialog` lista os achados com motivos ("mesmo telefone", "mesmo CPF", "nome parecido"), ações **Usar este cadastro** e **Criar outro cadastro**; máscara de CPF/telefone ao digitar.

- [x] **Step 1: Testes que falham**: nome e telefone obrigatórios com `*`/legenda e erros por campo; CPF inválido mostra erro; ao salvar com duplicados o diálogo aparece e **o formulário mantém os dados**; "Criar outro cadastro" reenvia com `confirm_duplicates=true`; busca por texto; sair com formulário sujo pede Salvar/Descartar/Continuar; recepção não vê "Desativar".
- [x] **Step 2: Run** `npx vitest run src/pages/guests` → FAIL.
- [x] **Step 3: Implementar.**
- [x] **Step 4: Run** `npx vitest run`, `npm run build` → PASS.
- [x] **Step 5: Commit** `feat: guests screens with duplicate review`

---

### Task 15: Fechamento do marco — shell final, busca e documentação

**Files:**
- Create: `src/pages/Search.tsx`, `docs/usuario/{usuarios,configuracoes,quartos,hospedes}.md`, `docs/desenvolvimento/{cadastros,permissoes,sequencias,seguranca-sessoes}.md`
- Modify: `Sidebar.tsx` (itens finais com ícones+rótulo), `AppShell.tsx` (campo de busca global), `README.md`, `CHANGELOG.md`, este plano e `local/plano-completo.md`

**Interfaces:**
- Produces: campo de busca no cabeçalho que navega para `/busca?q=`; `Search.tsx` consulta hóspedes (e, depois, reservas/hospedagens) respeitando permissões e mostra resultados vazios com orientação; documentação do marco.

- [x] **Step 1: Teste que falha** `Search.test.tsx`: digitar e enviar leva a `/busca?q=...`; resultado de hóspede mostra nome, código e link; consulta vazia mostra instrução.
- [x] **Step 2: Run** `npx vitest run src/pages/Search.test.tsx` → FAIL.
- [x] **Step 3: Implementar** a busca e a documentação (linguagem simples em `docs/usuario/`; regras, permissões, auditoria e efeitos em dados em `docs/desenvolvimento/`); `CHANGELOG.md` `Added/Changed/Security`.
- [x] **Step 4: Run** suítes completas: `cargo test`, `cargo clippy -D warnings`, `npm test`, `npm run build`; `graphify update .`.
- [x] **Step 5: Atualizar** a seção Status deste plano (passos, testes, desvios, pendências) e o roteiro; **revisão final da branch** por revisor independente; corrigir Críticos/Importantes; commit `docs: close marco 1`.

---

## Self-Review

- **Cobertura da spec (Marco 1):** §3.1 quartos/categorias → T11/T12; §3.2 hóspedes e duplicidade → T13/T14; §3.3 códigos → T5/T13; §13.1 permissões e §13.2 senhas/usuários → T3/T6/T10; §16.2 versão/esquema → T9; §17 dados da pousada → T8/T9; §18 componentes, asteriscos, alterações não salvas → T7/T12/T14; §19 formatos → T7. Dívida do Marco 0 (roteiro §2) → T1–T4, T8/T9. Ficam para marcos posteriores: reservas, hospedagens e histórico do hóspede.
- **Consistência de tipos:** `Page<T>` (T10) reutilizado em T11/T13; `expected_version` aparece em todas as edições; `AuditEntry.at` (T2) é usado por todas as tarefas seguintes; `PermissionSet`/`SessionInfo.extra` (T6) é consumido por `require` em T8–T13; `ManageCatalog` (T6) usada em T11/T13.
- **Migrações:** a numeração do roteiro (`0002` único) foi dividida em `0002`–`0005`; as seguintes do roteiro avançam de acordo (revisar numeração ao planejar o Marco 2).
- **Proporção:** corpos de função ficaram a cargo do implementador; testes trazem os valores exatos da spec.
