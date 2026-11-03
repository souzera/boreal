# Segurança de sessões e autenticação

## Login e bloqueio de conta

- Senhas com Argon2id, sem normalização; mínimo de **6** caracteres; vazia ou só espaços é recusada.
- Usuário inexistente e senha errada devolvem o mesmo erro e custam o mesmo tempo (hash fictício calculado na criação do serviço).
- Ao atingir `max_failed_attempts`, a conta é bloqueada até `now + lockout_minutes` (instante UTC absoluto). Voltar o relógio só **alonga** o bloqueio. Depois que expira, o contador recomeça (`check_lock`).
- `unlock_session` e `change_own_password` também contam senhas erradas, para não virarem atalho de força bruta.

## Sessões

- Em memória, token de 32 bytes aleatórios.
- Bloqueio por inatividade usa relógio **monotônico** (`Instant`); o limite vem de `idle_lock_minutes` e é reaplicado ao salvar as configurações.
- A interface mantém o backend informado com um batimento de atividade (no máximo a cada 30 s); a página aberta continua montada, oculta e inerte sob a tela de bloqueio (`LockGate`), para não perder edições.
- Estados da sessão na interface: `anonymous`, `active`, `locked` (inatividade) e `expired` (o backend encerrou a sessão: senha redefinida, permissões ou perfil alterados, usuário desativado). Em `locked` e `expired` o `LockGate` mantém a página montada, mas oculta e inerte, e `LockContext` faz todo `DialogContent` deixar de ser exibido (diálogos em portal não podem ficar por cima da tela de bloqueio). Em `expired` a pessoa entra de novo na própria tela e continua de onde parou.
- O botão **Sair** é uma navegação para `/login`: com alterações não salvas, o aviso Salvar/Descartar/Continuar aparece antes; a sessão é encerrada ao chegar em `/login`.
- O cache de consultas (`queryClient`) é esvaziado ao sair e quando outra pessoa entra.
- O tempo de inatividade da interface (`idleMinutes`, no store) vem do backend ao entrar e é atualizado ao salvar as configurações.
- Trocar a própria senha encerra as outras sessões da pessoa; redefinir ou desativar encerra todas.

## Auditoria

`audit_log` é imutável (triggers) e recebe todo evento na mesma transação da operação. `audit::scrub` remove chaves de senha em qualquer nível. `AuditEntry.at` vem do relógio injetado. Hóspedes: CPF só mascarado.

## Erros

`AppError` serializa `code` estável + mensagem em português simples. `Database(detalhe)` mostra uma mensagem genérica; o detalhe fica só no `Debug`/logs. `Debug` de `NewUserInput` e `LoginResult` oculta senha e token.

## Interface (Tauri)

CSP restritiva em `tauri.conf.json` (`devCsp` acrescenta o servidor de desenvolvimento). Sem plugins não usados. Comandos Tauri são finos: desserializam, chamam o serviço e serializam.
