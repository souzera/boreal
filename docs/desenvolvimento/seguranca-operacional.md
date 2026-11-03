# Segurança operacional (Marco 7, T9)

Decisões K1, K13 a K15 do plano do Marco 7.

## Segredos em memória

`domain::secret::SecretKey`: zera o conteúdo ao ser solto (`zeroize`) e o `Debug` mostra `SecretKey(***)`. A chave do banco é lida do cofre do Windows **uma vez** na abertura e fica em `Ctx.key`; é usada para gravar e validar backups e para a chave de recuperação. A chave de recuperação (`domain::recovery_key`) é a mesma chave em 8 grupos de 8 caracteres hexadecimais mais 4 de conferência (CRC-16); só aparece ao administrador, **com a senha**, e a auditoria registra apenas que foi mostrada.

## CSP

Produção e desenvolvimento: `default-src 'self'`, `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'self'`; produção só se conecta a `'self' ipc: http://ipc.localhost`. **Aceito:** `style-src 'unsafe-inline'` (Tailwind e Base UI usam estilos em linha; não há `script-src 'unsafe-inline'` nem `unsafe-eval`). `src/lib/hardening.test.ts` trava essas regras.

## Permissões do Tauri

`capabilities/default.json` (janela `main`): `core:default`, os seis `core:window:allow-*` da barra de título, `dialog:allow-save` e `dialog:allow-open`. **Nenhuma** permissão de sistema de arquivos, shell, rede, processo ou `opener` (abrir um arquivo exportado passa por um comando do backend que só aceita `.pdf`/`.csv` existentes, com caminho absoluto). O teste trava a **lista fechada**: acrescentar uma permissão exige mudar o teste de propósito.

## Dependências

- **`npm audit`** (executado em 03/11/2026): o `shadcn` (CLI, usado só pelo CSS `shadcn/tailwind.css` em tempo de build) estava em `dependencies` e trazia `express`, `body-parser` (alto), `ts-morph` e `minimatch` (moderados), `deepmerge` (baixo). **Corrigido:** movido para `devDependencies`; `npm audit --omit=dev` ⇒ **0 vulnerabilidades**; o `dist` não contém esses pacotes. As 7 ocorrências restantes são só de ferramenta de desenvolvimento (CLI), sem correção sem salto de versão maior; ficam **aceitas** como risco de desenvolvimento e reavaliadas a cada atualização do `shadcn`.
- **`cargo audit`** (cargo-audit 0.22.2, 1.403 avisos, executado em 03/11/2026): **1 vulnerabilidade** em `ctutils` 0.4.3 (comparações não constantes; dependência do `argon2`, que faz o hash das senhas), **corrigida** com `cargo update -p ctutils` (0.4.8, sem mudar o `Cargo.toml`); testes de autenticação reexecutados. **6 avisos aceitos** (sem vulnerabilidade): crates marcados como *sem manutenção* ou *unsound* que entram só pelas dependências de **Linux** do Tauri (`zbus_names`, `zvariant`, `zvariant_derive`, `proc-macro-error`, `glib`) e uma versão *yanked* (`async-executor`); não fazem parte do build do Windows. Reavaliar a cada atualização do Tauri.

## Logs e erros

Os logs técnicos passam por `logging::scrub`; o teste `backup_restore_and_key_reveal_are_logged_without_any_secret` confere que chave, chave de recuperação, senha e caminhos completos não aparecem. Erros de integridade e de atualização mostram sempre a mesma mensagem; o detalhe (nomes de tabelas, no máximo) fica só no `Debug` e no log.
