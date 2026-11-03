# Marco 7 — Backup, recuperação, atualização e segurança operacional Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Proteger os dados da pousada ao longo do tempo: chave de recuperação, backup criptografado e validado, restauração segura (inclusive em outro computador), verificação de integridade na abertura, migrações que só rodam com backup válido, alertas administrativos e uma tela de "Saúde do sistema".

**Architecture:** O banco já é SQLCipher com a chave no cofre do Windows. O backup é uma **cópia criptografada com a mesma chave** (API de backup do SQLite sobre a conexão aberta), escrita num arquivo parcial, **validada** (abre com a chave, `integrity_check`, `foreign_key_check`, versão do esquema) e só então renomeada; a retenção apaga o que sobrar depois de um backup novo válido. A chave de recuperação é essa mesma chave em texto agrupado e com dígito de conferência, mostrada só ao administrador com a senha. Um `BackupService` (com a chave em memória, zerada ao sair) faz backup, restauração e checagens; a inicialização do aplicativo ganha o passo "backup antes de migrar", o relatório de integridade e o modo de recuperação. Os alertas saem de detectores que leem o histórico de execuções (`backup_runs`) e o espaço em disco (sonda injetável nos testes).

**Tech Stack:** Rust/rusqlite (feature `backup`)/SQLCipher, `fs4` (espaço livre), `zeroize`; React/TS/Vitest; plugin Tauri `dialog` (já instalado).

**Spec:** `local/spec.md` §14 (segurança), §15 (backups e recuperação), §16 (atualizações e migrações), §20.2 (alertas), §24 (critérios 8, 9, 10, 14); `local/plano-completo.md` §11, tarefas 7.1 a 7.9. Último marco antes de Qualidade/empacotamento (Marco 8).

**Branch:** `marco-7-backup`, a partir de `marco-6-4-relatorios`. **Migração `0014`** (`backup_runs`, `system_events`).

## Decisões deste plano (chamadas minhas; revisar na tela pronta)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| K1 | **Chave de recuperação** = a chave do banco (32 bytes) em 64 caracteres hexadecimais, em 8 grupos de 8, mais um grupo de conferência de 4 (CRC-16). Só o administrador a vê, **digitando a senha de novo**; cada exibição é auditada (sem a chave). A tela deixa imprimir e pede para guardar fora do computador. Perdeu a chave **e** o cofre do Windows ⇒ dados irrecuperáveis (aviso na tela). | Spec §14: a recuperação da chave é requisito crítico. Custo: quem tem a chave impressa tem acesso total; por isso a senha na exibição. |
| K2 | **Backup = cópia criptografada com a mesma chave do banco** (a chave de recuperação abre qualquer backup). Nome `boreal-AAAA-MM-DD-HHMMSS.bak` (hora local). Nada em texto claro. | Um só segredo para guardar. Custo: trocar a chave do banco exigiria reexportar os backups antigos (fora do escopo). |
| K3 | **Quando roda:** (a) **Fazer backup agora** (manual); (b) **ao concluir o fechamento de caixa** (é o "encerrar o expediente" que o sistema conhece), em segundo plano e sem nunca falhar o fechamento; (c) ao abrir o aplicativo, se o último backup válido tem mais de 24 h. Sem gancho ao fechar a janela. | Cobre "diário ao encerrar o expediente" sem depender de horário fixo. Custo: pousada que não fecha o caixa e não abre o app por dias fica sem backup novo (o alerta avisa). |
| K4 | **Retenção:** o backup **mais novo de cada um dos 7 dias** mais recentes com backup; o resto é apagado **só depois** de um backup novo validado, e **só** arquivos com o padrão do nome (nunca outros arquivos da pasta). | Spec §15.1. |
| K5 | **Validação** depois de gravar: abre com a chave, `PRAGMA integrity_check = ok`, `foreign_key_check` vazio, mesma versão do esquema do banco de origem e mesma contagem da trilha de auditoria. Arquivo parcial (`.partial`) é apagado em qualquer falha. Antes de gravar: pasta existe (ou é criada), aceita escrita (arquivo de teste) e há espaço livre ≥ 2× o tamanho do banco + 64 MB. | Spec §15.1. |
| K6 | **Alertas** (críticos, não dispensáveis, só para o administrador — `ManageSettings`): **Sem backup válido** (nenhum nas últimas 24 h; o texto diz o motivo da última falha), **Pouco espaço em disco** (livre < maior entre 1 GB e 3× o banco) e **Falha na atualização**. Somem sozinhos quando a causa some. | Spec §20.2 e §15. A recepção não consegue resolver nenhum dos três. |
| K7 | **Restaurar** (só `RestoreBackup`, administrador): escolhe um backup da pasta (ou outro arquivo), vê a **prévia** (data do backup, versão do esquema, contagens, se é mais antigo ou mais novo que o banco atual), digita `RESTAURAR` e a senha; o sistema faz **antes** uma cópia de segurança do estado atual (falhou ⇒ não restaura), copia o backup **para dentro da conexão aberta** (API de backup), reaplica migrações se o backup for de esquema antigo, confere a integridade e **encerra todas as sessões** (volta ao login). A auditoria é gravada **depois**, no banco restaurado, com o nome do administrador no texto (o usuário pode nem existir no backup). Backup inválido ou de esquema mais novo que o aplicativo é recusado. | Spec §15.2. Custo: a restauração exige que o aplicativo continue aberto; falha no meio da cópia é coberta pela cópia de segurança e pela validação final (roda de novo a restauração da cópia). |
| K8 | **Troca de computador:** na tela de primeiro acesso, o link **Já tenho um backup** pede o arquivo e a **chave de recuperação**, confere (abre o backup com a chave), grava a chave no cofre do novo computador, copia o arquivo para o lugar do banco e abre o aplicativo. Só funciona se ainda **não existe** banco ali. | Spec §16.2 ("backup e restauração manual"). |
| K9 | **Modo de recuperação** (banco com problema de integridade ao abrir): a tela de erro de inicialização mostra o problema e oferece restaurar um backup da pasta padrão, **sem login** (o banco não permite), exigindo a **chave de recuperação** e a digitação de `RESTAURAR`; guarda antes uma cópia do banco com problema (`boreal-com-problema-…db`). | Sem esta saída, banco corrompido = aplicativo inutilizável. Custo: quem tem a chave impressa pode restaurar sem usuário. |
| K10 | **Disco cheio:** o SQLite já desfaz a transação inteira; o sistema só **traduz** `SQLITE_FULL` para "Sem espaço em disco. Libere espaço e tente de novo. Nada foi salvo." (código `disk_full`) e o alerta de pouco espaço avisa antes. Teste com limite de páginas. | "Impedida sem gravação parcial" já é garantido pelas transações. |
| K11 | **Migração segura:** com migrações pendentes **e** banco com dados, o aplicativo faz um **backup antes de migrar** (pasta configurada ou `…/backups`); se falhar, **não migra** e a tela de erro explica que os dados estão intactos. Cada migração continua numa transação própria. | Spec §16.1. |
| K12 | **Atualização que falha:** a falha de migração é gravada em `startup-status.json` ao lado do banco; na próxima abertura bem-sucedida o sistema registra o evento `update_failed` em `system_events` e o alerta **Falha na atualização** fica até uma migração concluir depois dele (`update_ok`). Banco **mais novo** que o aplicativo já é recusado (`SchemaTooNew`) com texto claro. | O banco não está disponível quando a migração falha, por isso o arquivo auxiliar. |
| K13 | **Logs técnicos** em `…/logs/boreal-AAAA-MM-DD.log` (separados da auditoria), uma linha por evento (`hora nível evento mensagem`), **sem** senhas, chaves, caminhos de backup completos nem dados de hóspedes; 14 dias de retenção. | Spec §15.3. |
| K14 | **Saúde do sistema** (`/saude`, só administrador): backups recentes, espaço em disco, versões, resultado da última verificação de integridade, chave de recuperação, restaurar e a pasta de logs. | Roteiro 7.8. |
| K15 | **Endurecimento:** CSP com `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'self'`; permissões do Tauri revistas e listadas; `npm audit` e `cargo audit` rodados e registrados (sem corrigir dependência de terceiros sem decisão); chave em memória zerada ao sair. | Roteiro 7.9. |

## Global Constraints

- Falha de backup **nunca** bloqueia a recepção nem o fechamento de caixa; vira alerta persistente.
- Nenhum backup, chave, senha ou caminho completo em log, auditoria, mensagem de erro, fixture ou documentação. Nenhum banco ou backup real no repositório.
- O backup só substitui/remove arquivos **depois** de um novo backup válido; só toca arquivos com o padrão do nome.
- Permissões no backend: restaurar e chave de recuperação exigem administrador (`RestoreBackup`/`ManageSettings`) **e a senha**; backup manual exige `ManageSettings`.
- Operações destrutivas (restaurar) só com confirmação digitada; a cópia de segurança do estado atual é obrigatória.
- Auditoria de negócio na mesma transação da operação; a da restauração é gravada no banco restaurado.
- Datas `DD/MM/AAAA`, horas `HH:MM` locais; instantes em UTC no banco.
- Ambiente: Strawberry Perl antes do Perl do Git no PATH; `npm install --legacy-peer-deps`; um `boreal-pms.exe` aberto impede `cargo test` sem `--lib`.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano, o Anexo A.9 da spec e o grafo.

## Review Focus

1. **Backup só vale se abre e é íntegro:** arquivo parcial sempre apagado em falha; backup antigo nunca removido antes de um novo válido; arquivos fora do padrão nunca tocados; destino inexistente/sem espaço vira falha registrada (T2).
2. **Segredos:** a chave de recuperação só aparece com senha e a um administrador; nunca em auditoria, log ou erro; backup sem a chave não abre; chave errada na restauração não altera nada (T1, T5, T7).
3. **Restauração segura:** cópia de segurança obrigatória; backup inválido ou de esquema novo recusado **sem tocar no banco**; sessões encerradas; auditoria no banco restaurado; dados idênticos aos do backup (T5).
4. **Migração:** pendente + dados ⇒ backup antes; sem backup, nada migra; migração que falha no meio deixa o banco anterior intacto e gera o alerta depois (T6).
5. **Integridade e disco:** banco com problema abre em modo de recuperação (não em tela em branco); `SQLITE_FULL` não deixa gravação parcial e vira mensagem clara; crash no meio de transação (simulado) não corrompe (T4).

---

## File Structure

```
src-tauri/
  migrations/sql/0014_backup_health.sql          # backup_runs, system_events
  src/domain/recovery_key.rs                     # formato, CRC, parse
  src/domain/backup.rs                           # nome, data, retenção (funções puras)
  src/domain/secret.rs                           # SecretKey (zera ao soltar, Debug mascarado)
  src/models/backup.rs, health.rs
  src/repositories/backup_repo.rs                # backup_runs, system_events
  src/services/backup_service.rs                 # criar, validar, reter, listar, restaurar
  src/services/recovery_service.rs               # chave de recuperação, nova máquina, modo de recuperação
  src/services/health_service.rs                 # verificação de integridade, disco, relatório de inicialização
  src/services/alert_detectors.rs                # + backup_missing, disk_low, update_failed
  src/logging.rs                                 # logs técnicos rotativos
  src/commands/backup.rs, recovery.rs, health.rs
src/
  lib/api.ts, lib/backup.ts
  pages/health/HealthPage.tsx, BackupCard.tsx, RestoreDialog.tsx, RecoveryKeyCard.tsx, DiskCard.tsx (+ testes)
  pages/RecoverFromBackup.tsx, pages/StartupError.tsx, pages/FirstRun.tsx    # nova máquina e modo de recuperação
  pages/settings/SystemForm.tsx                  # pasta de backup
docs/usuario/{backup-e-restauracao,chave-de-recuperacao,saude-do-sistema}.md
docs/desenvolvimento/{backup-e-recuperacao,seguranca-operacional}.md
```

---

### Task 1: Chave de recuperação e segredo em memória (7.1)

**Files:**
- Create: `domain/secret.rs`, `domain/recovery_key.rs`, `services/recovery_service.rs`, `commands/recovery.rs`
- Modify: `Cargo.toml` (`zeroize = "1"`), `domain/mod.rs`, `services/mod.rs` (`Ctx` ganha `key: Arc<SecretKey>`), `app.rs` (lê a chave uma vez em `initialize`), `services/test_support.rs` (chave fixa de teste, `recovery`), `commands/mod.rs`, `lib.rs`, `src/lib/api.ts`
- Test: `domain/recovery_key.rs`, `services/recovery_service.rs`

**Interfaces:**
- Produces:
  - `struct SecretKey(Vec<u8>)` (`new`, `as_bytes`, `hex`; `Drop` zera; `Debug` imprime `SecretKey(***)`);
  - `recovery_key::format(key: &[u8]) -> String` (`XXXXXXXX-…` 8 grupos + `-CCCC`, maiúsculas), `recovery_key::parse(text: &str) -> Result<Vec<u8>, AppError>` (ignora espaços, hífens e caixa; confere o CRC; tamanho 32 bytes; erros em português: "A chave de recuperação está incompleta." / "Há um erro de digitação na chave de recuperação.");
  - `RecoveryService::reveal(&self, token, password) -> Result<String, AppError>` (`ManageSettings` + senha do próprio administrador; audita `settings`/`reveal_recovery_key` sem a chave; erro de senha = `InvalidCredentials` sem contar como tentativa de login).
- Regras: a chave nunca aparece em `Debug`, auditoria ou erro.

- [x] **Step 1: Testes** (`recovery_key`): `format` de uma chave conhecida devolve 8 grupos de 8 + conferência de 4; `parse(format(k)) == k`; aceita minúsculas, espaços e sem hífens; troca de um caractere, grupo faltando e tamanho errado dão os dois erros certos; `SecretKey` zera o conteúdo ao soltar (via `zeroize` verificado numa cópia por ponteiro) e `format!("{key:?}")` não contém os bytes. (`recovery_service`): admin com a senha certa recebe a chave igual à da `Ctx`; senha errada ⇒ `InvalidCredentials` e nada é auditado como revelação; recepção ⇒ `Forbidden`; auditoria tem `reveal_recovery_key` e **não** contém a chave; sem sessão ⇒ erro.
- [x] **Step 2: Run** `cargo test --lib -- recovery` — Expected: FAIL.
- [x] **Step 3: Implement** `SecretKey`, o formato com CRC-16/CCITT, o serviço e o comando `reveal_recovery_key`; `initialize` lê a chave do `KeyStore` e a guarda em `Ctx`.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: recovery key and in-memory secret`

---

### Task 2: Motor de backup (7.2)

**Files:**
- Create: `migrations/sql/0014_backup_health.sql`, `domain/backup.rs`, `models/backup.rs`, `repositories/backup_repo.rs`, `services/backup_service.rs`
- Modify: `Cargo.toml` (`rusqlite` com `backup`; `fs4`), `migrations/mod.rs` (versão 14 e contagens dos testes), `models/settings.rs` (+ `backup_dir: String`), `repositories/settings_repo.rs`, `app.rs`, `services/test_support.rs` (`backup` e `DiskProbe` falso), `services/mod.rs` (`DiskProbe` no `Ctx`)
- Test: `domain/backup.rs`, `services/backup_service.rs`, `migrations`

**Interfaces:**
- Migração `0014`: `backup_runs(id, kind 'manual'|'auto'|'pre_migration'|'pre_restore', status 'ok'|'failed', started_at, finished_at, file_name, bytes, schema_version, message)` e `system_events(id, kind 'update_failed'|'update_ok'|'integrity_problem'|'restore', at, message)`, ambas **imutáveis** (gatilhos), índice por `finished_at`.
- Produces:
  - `domain::backup::{file_name(local: NaiveDateTime) -> String, parse_date(name: &str) -> Option<NaiveDate>, is_backup_name(name) -> bool, retain(names: &[String], days: usize) -> Vec<String> /* a apagar */}`;
  - `trait DiskProbe { fn free_bytes(&self, path: &Path) -> Result<u64, AppError>; }` (real com `fs4`; falso nos testes) em `Ctx`;
  - `BackupService::{run(token_or_system, kind) -> Result<BackupRunView, AppError>, run_system(kind) /* sem sessão: usado pelo fechamento e pela abertura */, list_files(token) -> Vec<BackupFileView>, recent_runs(token, limit) -> Vec<BackupRunView>, last_valid_at() -> Option<String>}`;
  - `BackupRunView { id, kind, status, finished_at, file_name, bytes, message }`, `BackupFileView { file_name, date, bytes }`.
- Regras (K2–K5): cópia criptografada com a chave de `Ctx` pela API de backup para `…/nome.bak.partial`; valida (K5); renomeia; grava `backup_runs`; aplica a retenção **depois** de validar; qualquer falha apaga o `.partial`, grava `failed` com mensagem amigável ("A pasta de backup não está acessível.", "Não há espaço suficiente na pasta de backup.") e devolve o erro; `backup_dir` vazio ⇒ `<pasta do banco>/backups`; `run` manual exige `ManageSettings`.

- [x] **Step 1: Testes** (`domain/backup`): nome local gera `boreal-2026-10-21-143005.bak`; `parse_date` lê o dia; nomes de fora do padrão são ignorados; `retain` mantém o mais novo de cada um dos 7 dias mais recentes e devolve o resto (3 arquivos no mesmo dia ⇒ 2 a apagar; 9 dias ⇒ os 2 mais antigos). (`backup_service`, banco em arquivo numa pasta temporária, `MemoryKeyStore`): `backup_creates_an_encrypted_valid_file_that_opens_only_with_the_key` (cabeçalho não é `SQLite format 3`, abre com a chave, `integrity_check = ok`, mesmos dados; sem a chave/chave errada não abre); `backup_restores_identical_data` (compara tabelas principais linha a linha); `failed_validation_removes_the_partial_file_and_records_the_failure`; `retention_removes_old_files_only_after_a_new_valid_backup` (falha no novo ⇒ nada apagado) e `retention_never_touches_files_outside_the_pattern`; `missing_folder_is_created_and_unwritable_folder_fails_with_a_friendly_message`; `not_enough_space_fails_before_writing` (sonda falsa com pouco espaço, nada criado); `recepcao_cannot_run_a_manual_backup` (`Forbidden`) mas `run_system` não exige sessão; `runs_are_recorded_and_immutable`; migração `0014` cria as tabelas e os gatilhos.
- [x] **Step 2: Run** `cargo test --lib -- backup` — Expected: FAIL.
- [x] **Step 3: Implement** migração, repositório, funções puras e o serviço; se a API de backup não funcionar com SQLCipher nesta versão, usar `ATTACH … KEY` + `sqlcipher_export` (registrar o desvio).
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: encrypted validated backups with retention`

---

### Task 3: Gatilhos de backup, alertas e disco (7.3 e parte de 7.5)

**Files:**
- Modify: `services/alert_detectors.rs` (`collect_system_findings`), `domain/alert.rs` (`managed_by_detectors` inclui os 3 tipos; `requires_permission(ManageSettings)`), `services/closing_service.rs`/`commands/closing.rs` (backup depois do fechamento), `lib.rs` (backup de recuperação ao abrir), `error.rs` (`disk_full`), `services/health_service.rs` (novo: `disk_status`)
- Create: `services/health_service.rs`, `models/health.rs`
- Test: `services/alert_detectors.rs`, `services/health_service.rs`, `error.rs`, `services/closing_service.rs`

**Interfaces:**
- Produces: `HealthService::disk_status(token) -> DiskStatus { free_bytes, db_bytes, threshold_bytes, low: bool }`; achados `backup_missing` (chave `backup`), `disk_low` (chave `disk`), `update_failed` (chave `update`); `AppError::from(rusqlite::Error)` traduz `SQLITE_FULL` para `Validation`-like com código `disk_full` e a frase do K10.
- Regras (K3, K6, K10): `backup_missing` quando não há `backup_runs` `ok` nas últimas 24 h; o detalhe traz a data do último válido ("Último backup válido: 21/10/2026 às 18:05") ou "Nenhum backup válido foi feito ainda." e, se a última tentativa falhou, o motivo; `disk_low` quando livre < max(1 GiB, 3× banco); `update_failed` quando o último `system_events` de tipo `update_failed` não tem `update_ok` depois; os três são críticos, só `ManageSettings` os vê e somem sozinhos; fechar o caixa dispara `run_system("auto")` em segundo plano e **nunca** muda o resultado do fechamento; ao abrir, `run_system("auto")` se o último válido tem mais de 24 h.

- [x] **Step 1: Testes:** `backup_missing_appears_without_a_recent_valid_run_and_resolves_after_one`; `a_failed_last_run_shows_its_reason_in_the_alert`; `old_valid_backup_over_24h_raises_the_alert`; `disk_low_uses_the_fake_probe_and_the_threshold` (limite 1 GiB, banco de 600 MB ⇒ 1,8 GB); `update_failed_stays_until_update_ok`; `the_three_alerts_are_admin_only_and_not_dismissable` (recepção não vê nem conta; `dismiss` recusa); `closing_succeeds_even_if_the_backup_fails` (pasta inválida ⇒ fechamento ok, alerta depois); `sqlite_full_is_translated_and_nothing_is_saved` (banco com `PRAGMA max_page_count`; inserir muitas linhas numa operação de serviço ⇒ erro `disk_full` e contagem das tabelas inalterada).
- [x] **Step 2: Run** `cargo test --lib -- alert_detectors health closing error` — Expected: FAIL.
- [x] **Step 3: Implement** os detectores (lendo `backup_repo` e `DiskProbe`), a permissão por tipo, o disparo pós-fechamento (thread separada, sem unir) e o de abertura.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: backup alerts, disk monitor and disk-full handling`

---

### Task 4: Integridade ao abrir, modo de recuperação e crash (7.5)

**Files:**
- Modify: `app.rs` (`App` passa a guardar `RwLock<Result<Arc<AppState>, AppError>>` e `state()` devolve `Result<Arc<AppState>, AppError>`; `initialize` roda a verificação e devolve o relatório), `commands/*.rs` (só se algum não compilar com o novo `state()`), `lib.rs`, `error.rs` (`IntegrityProblem`, código `integrity`), `services/health_service.rs` (`startup_report`), `tests/startup.rs`
- Test: `services/health_service.rs`, `app.rs`, `tests/startup.rs`

**Interfaces:**
- Produces: `health::check_database(conn) -> IntegrityReport { ok: bool, problems: Vec<String> }` (`PRAGMA integrity_check` + `foreign_key_check`, no máximo 20 problemas, mensagens sem dados); `initialize` grava `system_events` `integrity_problem` quando há problema e, com problema, **não** abre o aplicativo normal: devolve `Err(AppError::IntegrityProblem(resumo))`; `HealthService::last_startup(token) -> StartupReport { checked_at, integrity_ok, problems, app_version, schema_version }`.
- Regras (K9): a verificação roda a cada abertura (banco pequeno) e o resultado fica no relatório; **crash simulado** não corrompe.

- [x] **Step 1: Testes:** `a_healthy_database_passes_the_startup_check_and_is_reported`; `a_corrupted_file_is_reported_as_an_integrity_problem_and_does_not_open` (altera bytes de páginas internas de uma cópia); `foreign_key_violations_are_listed_without_row_data`; `crash_in_the_middle_of_a_transaction_leaves_the_database_intact` (abre transação, grava, `mem::forget` da conexão, copia `.db` e `-wal` para outra pasta, reabre a cópia: integridade ok e a gravação **não** está lá, a anterior está); `initialize_with_a_wrong_key_still_fails_with_key_invalid_and_does_not_touch_the_file`; `state_can_be_replaced_after_a_recovery` (o novo `App` troca o estado e os comandos passam a usar o novo).
- [x] **Step 2: Run** `cargo test --lib -- health app` — Expected: FAIL.
- [x] **Step 3: Implement** a verificação, o relatório, `IntegrityProblem` e a troca de `App` para `RwLock<…Arc…>`; ajustar os comandos que não compilarem.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` e, **sem** `boreal-pms.exe` aberto, `cargo test --test startup` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: startup integrity check and recovery mode state`

---

### Task 5: Restauração (7.4 e troca de computador)

**Files:**
- Modify: `services/backup_service.rs` (`preview_restore`, `restore`), `services/recovery_service.rs` (`restore_on_new_computer`, `recover_corrupted`), `commands/backup.rs`, `commands/recovery.rs`, `lib.rs`, `src/lib/api.ts`
- Test: `services/backup_service.rs`, `services/recovery_service.rs`

**Interfaces:**
- Produces:
  - `BackupService::preview_restore(token, path) -> RestorePreview { backup_at, file_name, schema_version, app_schema_version, newer_than_current: bool, counts: { reservations, stays, payments, guests }, current_counts }` (`RestoreBackup`; abre o arquivo com a chave, `integrity_check`);
  - `BackupService::restore(token, path, confirmation: &str, password: &str) -> Result<RestoreResult { safety_copy: String }, AppError>`;
  - `RecoveryService::restore_on_new_computer(backup_path, recovery_key_text, db_path, keys: &dyn KeyStore) -> Result<(), AppError>` (só se `db_path` não existe);
  - `RecoveryService::recover_corrupted(backup_path, recovery_key_text, confirmation, db_path, keys) -> Result<(), AppError>`; comandos `preview_restore`, `restore_backup`, `restore_on_new_computer`, `recover_corrupted` (os dois últimos sem sessão e que reinicializam o estado).
- Regras (K7–K9): confirmação exata `RESTAURAR` (senão `Validation`), senha do administrador conferida, backup inválido/de esquema mais novo recusado **antes** de qualquer escrita; cópia de segurança obrigatória (`pre_restore`); restaura pela API de backup para dentro da conexão; reaplica migrações; confere a integridade; encerra todas as sessões; registra `system_events` `restore` e a auditoria no banco restaurado (usuário nulo, nome no texto); chave errada ⇒ `KeyInvalid` sem alterar nada; nova máquina grava a chave no cofre **somente depois** de o backup abrir com ela.

- [x] **Step 1: Testes:** `restore_brings_back_exactly_the_backed_up_data` (cria dados, backup, altera, restaura, compara); `restore_makes_a_safety_copy_first_and_fails_without_it`; `invalid_or_newer_schema_backups_are_refused_without_touching_the_database`; `wrong_confirmation_or_wrong_password_changes_nothing`; `reception_cannot_preview_or_restore` (`Forbidden`); `restore_ends_all_sessions_and_audits_in_the_restored_database`; `restoring_an_older_schema_backup_migrates_it_forward`; `new_computer_restore_end_to_end` (banco A + backup → chave de recuperação → pasta vazia B com cofre vazio: nova máquina restaura, a chave vai ao cofre, o app abre e os dados batem; com chave errada nada é criado nem gravado no cofre); `new_computer_restore_refuses_when_a_database_already_exists`; `corrupted_database_can_be_recovered_with_key_and_typed_confirmation` (guarda a cópia `boreal-com-problema-…`).
- [x] **Step 2: Run** `cargo test --lib -- restore recovery` — Expected: FAIL.
- [x] **Step 3: Implement** prévia, restauração, nova máquina e recuperação, e os comandos.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: safe restore, new computer restore and corrupted database recovery`

---

### Task 6: Migração segura e atualização (7.6 e 7.7)

**Files:**
- Modify: `app.rs` (`initialize`: backup antes de migrar, `startup-status.json`, evento `update_failed`/`update_ok`), `migrations/mod.rs` (`pending(conn) -> u32`), `services/health_service.rs`, `services/settings_service.rs`/`models/settings.rs` (`AppInfo` ganha `last_valid_backup_at`), `src/pages/settings/AboutCard.tsx`, `src/pages/StartupError.tsx`
- Test: `app.rs`, `migrations/mod.rs`

**Interfaces:**
- Produces: `migrations::pending(conn) -> usize`; `initialize` com a ordem **abrir → verificar integridade → (pendentes e dados?) backup `pre_migration` → migrar → registrar `update_ok`**; falha de backup ⇒ `Err(AppError::MigrationBackupFailed)` com texto "Não foi possível fazer o backup antes de atualizar o banco. Seus dados estão intactos…" e **nenhuma** migração aplicada; falha de migração ⇒ `startup-status.json` (`{ "failed_at", "message" }`, sem dados) e erro claro; na abertura seguinte bem-sucedida, o evento `update_failed` entra em `system_events`.
- Regras (K11, K12): banco novo (sem dados) migra sem backup; `SchemaTooNew` mantém a recusa com texto "Este banco foi criado por uma versão mais nova do Boreal…"; Configurações → Sobre mostra versão do app, do esquema, última atualização e último backup válido.

- [x] **Step 1: Testes:** `pending_migrations_with_data_make_a_backup_first` (banco no esquema 12 com dados; `initialize` ⇒ arquivo `.bak` válido existe e o esquema vai a 14); `a_fresh_database_migrates_without_a_backup`; `if_the_pre_migration_backup_fails_nothing_is_migrated` (pasta de backup impossível ⇒ erro e `schema_migrations` intacto); `a_failing_migration_leaves_the_previous_database_intact_and_writes_the_status_file`; `next_successful_start_records_update_failed_and_the_alert_resolves_after_update_ok`; `a_newer_database_is_refused_with_a_plain_message_and_untouched`; `about_info_includes_the_last_valid_backup`.
- [x] **Step 2: Run** `cargo test --lib -- app migrations` — Expected: FAIL.
- [x] **Step 3: Implement** o fluxo de `initialize`, o arquivo auxiliar e os eventos; front: `AboutCard` e o texto da `StartupError`.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `npx vitest run src/pages/settings`, `npx tsc --noEmit` — Expected: PASS.
- [x] **Step 5: Commit** `feat: backup before migrating and update failure tracking`

---

### Task 7: Logs técnicos (7.8)

**Files:**
- Create: `src/logging.rs`
- Modify: `lib.rs` (inicia o log; troca os `eprintln!`), `services/backup_service.rs`, `services/recovery_service.rs`, `app.rs` (eventos), `commands/health.rs` (pasta de logs)
- Test: `logging.rs`

**Interfaces:**
- Produces: `logging::{init(dir: &Path), event(level: Level, name: &str, message: &str), rotate(dir, today, keep_days)}`; formato `2026-10-21T18:05:00-03:00 INFO backup.ok mensagem`; `logging::scrub(message) -> String` remove caminhos completos (mantém só o nome do arquivo), sequências de 64 caracteres hexadecimais e qualquer texto depois de `senha=`/`password=`.
- Regras (K13): arquivos diários, 14 dias; linha com no máximo 500 caracteres; falha ao escrever o log **nunca** propaga.

- [x] **Step 1: Testes:** `writes_one_line_per_event_in_the_daily_file`; `rotation_keeps_the_last_14_days_and_ignores_other_files`; `scrub_removes_full_paths_hex_keys_and_passwords`; `a_log_failure_never_breaks_the_caller` (pasta somente leitura simulada); `backup_and_restore_log_events_without_secrets` (executa um backup e uma restauração com chave conhecida e confere que nenhum arquivo de log contém a chave, a senha ou o caminho completo).
- [x] **Step 2: Run** `cargo test --lib -- logging` — Expected: FAIL.
- [x] **Step 3: Implement** o módulo e ligar os eventos (backup ok/falha, restauração, integridade, migração).
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` — Expected: PASS / limpo.
- [x] **Step 5: Commit** `feat: technical logs without secrets`

---

### Task 8: Saúde do sistema, backup e chave na interface (7.8 e telas de 7.1 a 7.5)

**Files:**
- Create: `commands/health.rs` (se ainda não), `src/lib/backup.ts`, `src/pages/health/{HealthPage,BackupCard,RestoreDialog,RecoveryKeyCard,DiskCard}.tsx` e testes, `src/pages/RecoverFromBackup.tsx`
- Modify: `src/lib/api.ts`, `src/routes.tsx` (`/saude`), `src/components/layout/NavItems.ts` (**Saúde do sistema**, `admin: true`, grupo Configurações, ícone `health_and_safety`), `src/pages/FirstRun.tsx` (link **Já tenho um backup**), `src/pages/StartupError.tsx` (modo de recuperação), `src/pages/settings/SystemForm.tsx` (pasta de backup com **Escolher pasta**), `capabilities/default.json` (`dialog:allow-open`), `NavItems.test.ts`, `Sidebar.test.tsx`

**Interfaces:**
- Produces (cliente): `listBackupFiles`, `recentBackupRuns`, `runBackup`, `diskStatus`, `lastStartup`, `revealRecoveryKey`, `previewRestore`, `restoreBackup`, `restoreOnNewComputer`, `recoverCorrupted` e os tipos `BackupRunView`, `BackupFileView`, `DiskStatus`, `StartupReport`, `RestorePreview`; `HealthPage` com: **Backup** (pasta atual, **Fazer backup agora**, lista dos últimos backups com data, tamanho e situação em texto, mensagem do último erro), **Restaurar** (escolher da lista ou **Escolher outro arquivo**; `RestoreDialog` mostra a **prévia** em texto, exige digitar `RESTAURAR` e a senha, botão **Restaurar este backup**; depois do sucesso volta ao login), **Chave de recuperação** (`RecoveryKeyCard`: **Mostrar chave** pede a senha, mostra os 9 grupos, **Imprimir** e aviso "guarde fora do computador; sem ela e sem este computador os dados não podem ser recuperados"), **Espaço em disco** (livre, limite, situação em texto e ícone), **Versões e verificação** (aplicativo, esquema, última verificação de integridade, pasta de logs).
- Regras: só o administrador vê o item e a tela (os comandos recusam a recepção); tudo com texto, não só cor; erros em linguagem simples; a senha nunca fica no estado depois do envio.

- [x] **Step 1: Testes:** `NavItems`/`Sidebar` (item só para o administrador; destino `/saude` existe); `HealthPage` (recepção vê o aviso e nada é consultado; administrador vê as cinco áreas); `BackupCard` (**Fazer backup agora** chama `runBackup` e mostra "Backup concluído: boreal-….bak"; falha mostra a mensagem do backend e a lista continua; estados vazio e carregando); `RestoreDialog` (prévia em texto; botão desabilitado até digitar `RESTAURAR` e a senha; cancelar não chama a API; sucesso chama `restoreBackup` e leva a `/login`; senha errada mostra o erro e **mantém** o diálogo; backup de esquema mais novo mostra a recusa); `RecoveryKeyCard` (pede a senha antes; mostra 9 grupos; senha errada não mostra nada; **Imprimir** chama `window.print`); `DiskCard` (limite e situação em texto, "Pouco espaço" com ícone); `FirstRun` (link **Já tenho um backup** abre `RecoverFromBackup`; chave inválida mostra o erro e nada é criado; sucesso recarrega); `StartupError` com problema de integridade oferece a recuperação; `SystemForm` (pasta de backup com **Escolher pasta**).
- [x] **Step 2: Run** `npx vitest run src/pages/health src/pages src/components/layout` — Expected: FAIL.
- [x] **Step 3: Implement** as telas e as chamadas; no Rust, completar `commands/health.rs`.
- [x] **Step 4: Run** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `cargo check`, `npx vitest run`, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: system health screen with backup, restore and recovery key`

---

### Task 9: Endurecimento (7.9)

**Files:**
- Modify: `src-tauri/tauri.conf.json` (CSP), `capabilities/default.json` (revisão), `Cargo.toml`/`package.json` (somente se uma atualização segura for trivial e testada), `docs/desenvolvimento/seguranca-operacional.md`
- Test: `src/lib/csp.test.ts` (novo), `tests/`

- [x] **Step 1: Testes:** `csp.test.ts` lê `tauri.conf.json` e afirma `default-src 'self'`, `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'self'`, ausência de `unsafe-eval` e de `*`; `capabilities.test.ts` afirma que só as permissões **listadas** existem (lista fechada no teste: `core:default`, as `core:window:allow-*` da barra de título, `dialog:allow-save`, `dialog:allow-open`) — acrescentar uma permissão exige mudar o teste de propósito; Rust: `secret_key_is_zeroed_on_drop` (se ainda não coberto) e `debug_output_never_shows_secrets` para `SecretKey`, `LogoChange` e as entradas de senha.
- [x] **Step 2: Run** `npx vitest run src/lib` — Expected: FAIL.
- [x] **Step 3: Implement** a CSP, a revisão das permissões e rodar `npm audit` e `cargo audit` (instalar a ferramenta se faltar; se não for possível, registrar como pendência); documentar cada achado em `seguranca-operacional.md` com decisão (corrigido, aceito com justificativa ou aguardando).
- [x] **Step 4: Run** `npx vitest run`, `npm run build`, `cargo check` (valida a CSP e as permissões) — Expected: PASS.
- [x] **Step 5: Commit** `chore: tighten csp and review permissions and dependencies`

---

### Task 10: Seed, documentação e encerramento

**Files:**
- Modify: `seed.rs` (+ teste), `docs/desenvolvimento/seed.md`, `CHANGELOG.md`, `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/README.md`, `local/plano-completo.md`, `local/spec.md` (Anexo A.9 com K1–K15), este plano
- Create: `docs/usuario/{backup-e-restauracao,chave-de-recuperacao,saude-do-sistema}.md`, `docs/desenvolvimento/{backup-e-recuperacao,seguranca-operacional}.md`

- [x] **Step 1: Seed:** o cenário de desenvolvimento cria uma pasta de backup de demonstração **fora do repositório** (a do banco de demonstração) e o teste afirma que `backup.run_system("manual")` funciona e que o alerta de backup some depois dele.
- [x] **Step 2: Docs:** guias de usuário (fazer e conferir backup, restaurar, **trocar de computador** passo a passo, guardar a chave de recuperação, o que fazer quando aparece cada alerta, banco com problema), de desenvolvimento (formato do backup, validação, retenção, restauração por dentro da conexão, ordem do `initialize`, arquivo auxiliar, logs, procedimento de **reversão de migração** com o backup `pre_migration`, limitações), `CHANGELOG.md`, roteiro (módulos de Backup e restauração e de Saúde do sistema, com ⚠ nos limites provisórios), Anexo A.9, `plano-completo.md`.
- [x] **Step 3: Verificação completa:** `cargo test --features dev-seed --lib` (e `--test startup` com o aplicativo fechado), `cargo clippy --all-targets --features dev-seed -- -D warnings`, `npx vitest run`, `npx tsc --noEmit`, `npm run build`; registrar contagens e tempos (backup e restauração de um banco com 5 anos de dados fictícios) no Status.
- [x] **Step 4:** `graphify update .`; anotar no Status.
- [x] **Step 5:** Revisão final por revisor independente (modelo mais capaz) com o Review Focus; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [x] **Step 6: Commit** `docs: close marco 7`

---

## Status

**Implementado e revisado** (revisão independente: 1 crítico, 4 importantes e 11 menores; críticos/importantes e a maior parte dos menores corrigidos — ver "Revisão final") na branch `marco-7-backup` (a partir de `marco-6-4-relatorios`). T1 a T10 concluídas; migração `0014`.

- **Testes (última execução completa):** `cargo test --features dev-seed --lib` 686 testes; `cargo test --test startup` 7 (os 4 de antes e 3 novos, agora executados com o aplicativo fechado); `cargo clippy --all-targets --features dev-seed -- -D warnings` limpo; `npx vitest run` 572 testes em 70 arquivos (`RatesPage` e outros testes por tempo falham só sob carga total e passam sozinhos); `npx tsc --noEmit`, `npm run build` e `cargo check` (valida CSP e permissões) limpos; `graphify update .` executado.
- **Auditoria:** `npm audit --omit=dev` 0 vulnerabilidades (o `shadcn` passou a `devDependencies`); `cargo audit` achou 1 vulnerabilidade em `ctutils` (do `argon2`), corrigida com `cargo update -p ctutils`; 6 avisos de crates sem manutenção/unsound de dependências Linux do Tauri aceitos (ver `docs/desenvolvimento/seguranca-operacional.md`).
- **Pendências de verificação (manuais, `npm run tauri dev` com o aplicativo real):** escolher a pasta de backup com o seletor; backup num pen drive; restaurar e voltar ao login; **trocar de computador** de ponta a ponta numa instalação limpa (cofre do Windows de verdade: gravar a chave, abrir o banco restaurado); modo de recuperação com um banco realmente corrompido; imprimir a chave de recuperação; antivírus de terceiros com os arquivos `.partial`; tempo de backup e de restauração com meses de dados.

## Desvios e decisões da execução

- A **pasta de backup** ficou no cartão Backup da tela Saúde do sistema (não no formulário de Configurações), para não mexer em `AppSettings` e seus testes; a configuração é a chave `backup_dir` em `settings`.
- Os testes de "fechamento não falha se o backup falhar" cobrem `spawn_auto` e o alerta (o disparo fica no comando `close_cash`, que ignora o resultado), não o serviço de fechamento.
- `reveal` com senha errada devolve `Validation("A senha está incorreta.")`, igual a `change_own_password` (o plano previa `InvalidCredentials`).
- **Defeito de desenho achado na execução:** a cópia de segurança pré-restauração usava o mesmo nome e a mesma **retenção diária** dos backups e, no mesmo dia, apagava o backup que estava sendo restaurado. Passou a ter nome próprio (`boreal-seguranca-…bak`), retenção própria (5) e fora da retenção diária.
- Restaurar troca o banco inteiro, então o histórico de backups voltava ao do backup; a cópia de segurança é registrada no banco restaurado (e conta como backup válido).
- `Ctx::production` lia uma configuração que só existe depois de migrar; passou a usar o padrão e `initialize` relê o tempo de inatividade depois das migrações.
- `unlock_with_recovery_key` (chave perdida com o banco existente) não estava no plano; entrou por causa de "perda de credencial do SO" (7.1).
- `PRAGMA page_size` volta como texto no SQLCipher; o tamanho do banco usa o arquivo (backup) ou trata os dois tipos (saúde).
- A API de backup do SQLite funcionou com SQLCipher nos dois sentidos; não foi preciso `sqlcipher_export`.
- Sem registro em `system_events` de integridade (o banco com problema não é confiável); fica só o log técnico.


## Revisão final

Corrigido (com teste): **C1** a troca de computador falhava no fluxo real (a primeira abertura já cria um banco vazio): agora o banco virgem é substituído e banco com dados é recusado; **I1** comandos de recuperação sem sessão só rodam no estado que os justifica (`guard`) e soltam a conexão antes de trocar o arquivo; **I2** falha da volta pela cópia de segurança vira mensagem que aponta o arquivo; **I3** cópia de segurança cai na pasta padrão se a configurada sumiu; **I4** tela de erro permite alternar desbloquear ↔ restaurar e cobre `schema_too_new`; menores: mutex de backup, `.partial` antigos, WAL no espaço, `version_checked`, `-wal/-shm` no modo de recuperação, reinício do diálogo de restauração, `queryClient.clear()` após restaurar, registro pós-restauração com mensagem própria.
**Aceitos/adiados:** menores 6 e 11 (sem limite de tentativas da chave de recuperação, já documentado; ensaio com antivírus de terceiros). Testes: `cargo test --features dev-seed --lib` 692, clippy limpo, vitest das telas (377 + novos) verdes.

## Depois do fechamento do marco

Documentação de apresentação, fora das tarefas deste plano: novo `README.md`, `docs/como-funciona.md` e prints em `docs/imagens/` (procedimento em `docs/imagens/README.md`). Sem mudança de produto, por isso sem entrada no `CHANGELOG.md`. O badge de CI ficou de fora porque o `main` no GitHub ainda não tem o workflow.

## Marcos seguintes

- **Marco 8:** qualidade, empacotamento e implantação (instalador, assinatura, ensaio de implantação, treinamento).

## Self-Review

- **Cobertura da spec:** §14 (chave protegida; sem abrir banco sem chave — já existe; recuperação e troca de computador) → T1/T5; §15.1 (diário ao encerrar, destino configurável, acessibilidade e espaço, validação, 7 diários, remoção só após novo válido, alerta 24 h, não bloqueia) → T2/T3; §15.2 (só admin, valida, data, confirmação, cópia do estado atual, registra, inválido não restaura) → T5; §15.3 (transações interrompidas, validação ao iniciar, logs, espaço em disco sem gravação parcial) → T4/T3/T7; §16.1/16.2 (migrações versionadas, backup antes, falha não deixa inconsistente, compatibilidade, tela de configurações com versões, troca de computador manual) → T6/T5/T8; §20.2 (`backup_missing`, `disk_low`, `update_failed`) → T3/T6; roteiro 7.8/7.9 → T7/T8/T9; testes-chave (backup→restaurar idêntico; sem chave não abre; disco cheio; crash; migração que falha; troca de computador ponta a ponta) → T2/T3/T4/T5/T6.
- **Consistência de tipos:** `SecretKey` (T1) usada por T2 e T5; `DiskProbe` e `backup_runs` (T2) usados por T3 e T8; `system_events` (T2) usados por T3, T4, T6; `App` com `RwLock<Arc<AppState>>` (T4) usado pela nova máquina e pela recuperação (T5); clientes TS (T8) espelham os modelos de T2 a T5.
- **Migração:** `0014`; as contagens de versão dos testes de migração sobem de 13 para 14.
- **Proporção:** corpos de função ficam com o implementador; os testes trazem valores exatos (24 h, 7 dias, 1 GiB, 14 dias, `RESTAURAR`, 64 caracteres, nomes de arquivo).
- **Fora deste plano:** cópia automática para um segundo dispositivo, agendamento por horário, criptografia com senha própria do backup, assistente gráfico de implantação (Marco 8), atualização automática pela internet.
