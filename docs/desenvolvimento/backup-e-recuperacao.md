# Backup, restauração e recuperação (Marco 7)

Plano: `docs/superpowers/plans/2026-11-03-marco-7-backup-e-recuperacao.md`; decisões K1 a K15 e Anexo A.9 da spec. **Migração `0014`** (`backup_runs`, `system_events`, configuração `backup_dir`).

## Formato do backup (K2)

- Arquivo `boreal-AAAA-MM-DD-HHMMSS.bak` (hora local), **criptografado com a mesma chave do banco**. A cópia é feita pela **API de backup do SQLite** (`rusqlite::backup`) a partir da conexão aberta para um arquivo `.partial` já com `PRAGMA key`; funcionou com SQLCipher nos dois sentidos (banco → arquivo e arquivo → banco). Antes de fechar, a cópia volta ao `journal_mode = DELETE` para ser **um arquivo só** (sem `-wal`/`-shm`).
- **Cópias de segurança** (antes de restaurar ou de migrar) usam `boreal-seguranca-AAAA-…bak`: **fora** da retenção diária (para nunca apagarem um backup, em particular o que está sendo restaurado — defeito encontrado e corrigido na execução) e com retenção própria (as 5 mais novas).

## Validação e retenção (K4, K5)

`BackupService::try_backup`: pasta existe/é criada e aceita escrita (arquivo de teste) → espaço livre ≥ 2× o tamanho do banco + 64 MB (sonda `DiskProbe`, `fs4::available_space`) → cópia para `.partial` → `validate_file` (abre com a chave, `integrity_check = ok`, `foreign_key_check` vazio, mesma versão do esquema e mesma contagem da auditoria que a origem, lidas **sob a mesma trava** da cópia) → renomeia → retenção. Qualquer falha apaga o parcial e os auxiliares. A retenção (`domain::backup::retain`) mantém o mais novo de cada um dos 7 dias mais recentes com backup e **só toca arquivos com o padrão exato do nome**. O histórico vai para `backup_runs` (melhor esforço; em banco antigo, sem a tabela, o backup vale mesmo assim).

## Gatilhos (K3)

`spawn_auto` (thread, nunca propaga erro): depois de `close_cash` (no comando, não no serviço) e `BackupService::catch_up` na abertura (se o último válido tem mais de 24 h). Manual: `run` (`ManageSettings`).

## Restauração (K7 a K9)

- `preview_restore` (`RestoreBackup`): valida o arquivo e compara contagens (reservas, hospedagens, recebimentos, hóspedes) com o banco atual; avisa se há atividade depois do backup; recusa arquivo inválido, ausente ou de esquema mais novo, **sem tocar em nada**.
- `restore`: confirmação `RESTAURAR` (sem diferenciar caixa) + senha → cópia de segurança **obrigatória** (`pre_restore`; sem ela nada é alterado) → `copy_into_live` (API de backup **para dentro da conexão aberta**, `journal_mode = DELETE` durante a cópia, depois WAL de novo; reaplica migrações; `check_database`) → se falhar, volta pela cópia de segurança → `destroy_all` nas sessões → no banco restaurado registra a execução da cópia de segurança, o evento `restore` e a auditoria (`user_id` nulo, nome do administrador no texto).
- **Nova máquina** (`restore_on_new_computer`) e **banco com problema** (`recover_corrupted`) não têm sessão: exigem a chave de recuperação e abrem o backup com ela **antes** de qualquer escrita; a chave só vai para o cofre depois; o banco com problema é renomeado para `boreal-com-problema-…db`. `unlock_with_recovery_key` reabre o banco existente quando a chave não está no cofre.
- O `App` guarda `RwLock<Result<Arc<AppState>>>`: depois de uma recuperação, `replace` troca o estado e os comandos seguem funcionando.

## Abertura do aplicativo (`app::initialize_with`) (K11, K12)

Abrir (a chave certa ou erro, sem tocar no arquivo) → `check_database` (integridade e vínculos; problema ⇒ `IntegrityProblem`, modo de recuperação) → `Ctx` → **migrações pendentes num banco com dados** ⇒ backup `pre_migration` obrigatório (falhou ⇒ `UpdateBackupFailed`, nada migra) → `run_with` (cada migração em transação própria; falha ⇒ `UpdateFailed` e o banco anterior fica intacto) → registro: `startup-status.json` (falha) vira o evento `update_failed` na abertura seguinte; migrações aplicadas geram `update_ok`. O alerta **Falha na atualização** dura até um `update_ok` posterior.

## Alertas e disco (K6, K10)

`collect_system_findings` (somente leitura): `backup_missing` (nenhum `backup_runs` `ok` nas últimas 24 h; o detalhe traz a data local do último e o motivo da última falha), `disk_low` (livre < max(1 GiB, 3× o banco)), `update_failed`. São críticos, **só `ManageSettings`** os vê/conta e não se dispensam. `Ctx.system_alerts` liga isso (desligado nos testes que não tratam do assunto). `SQLITE_FULL` vira `AppError::DiskFull` (código `disk_full`: "Nada foi salvo."); a transação já foi desfeita pelo SQLite.

## Logs (K13)

`logging::Logger` (em `Ctx.log`): `logs/boreal-AAAA-MM-DD.log`, uma linha `hora nível evento mensagem`, 500 caracteres no máximo, 14 dias, **nunca propaga erro**. `scrub` tira caminhos completos (fica só o nome do arquivo), sequências hexadecimais ≥ 32 e valores depois de `senha=`/`password:`/`chave=`; datas `DD/MM/AAAA` não são confundidas com caminhos.

## Procedimento de reversão de migração

1. Feche o aplicativo. 2. Reinstale a versão anterior do aplicativo. 3. Abra o aplicativo anterior em modo de recuperação, ou, se o banco estiver íntegro e na versão antiga, ele abre normalmente. Se o banco foi migrado e a versão antiga o recusa (`SchemaTooNew`), restaure a cópia `boreal-seguranca-…` feita **antes** da migração (tela de recuperação ou **Saúde do sistema → Restaurar**), com a chave de recuperação. Os dados feitos depois da migração estão no banco migrado, que nesse caso deve ser guardado.

## Limitações conhecidas

- Sem gancho ao fechar a janela: pousada que não fecha caixa e não abre o aplicativo por dias fica sem backup novo (o alerta avisa).
- O backup usa a mesma chave do banco; trocar a chave do banco exigiria reexportar os backups antigos.
- A restauração mantém o aplicativo aberto e copia por cima da conexão; o ensaio em disco cheio e com antivírus de terceiros fica para o Marco 8.
- `unlock_with_recovery_key` e o modo de recuperação não têm limite de tentativas (quem tem a chave tem o banco).
