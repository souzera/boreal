# Identidade visual: logo, ícone e barra de título

**Status:** implementado e verificado (vitest 506 testes em 65 arquivos, tsc e build limpos, cargo check ok) na branch `identidade-visual` (a partir de `marco-6-3-recepcao`). Sem migração e sem mudança de regra de negócio.

## O que foi pedido

"Fazer uma logo, ícone e customizar a barra do título." As perguntas de escolha (desenho de referência; só visual ou janela sem moldura) não foram respondidas e o trabalho seguiu com as escolhas abaixo.

## Decisões (revisar na tela)

| # | Decisão | Custo se estiver errada |
|---|---|---|
| I1 | Logo "aurora": quadrado arredondado em gradiente teal da identidade, três fitas de aurora subindo e uma estrela. Nome ao lado em texto (fonte do sistema), sem desenho de letras. | Trocar `boreal.svg` e `Logo.tsx` e regenerar os ícones (um comando). |
| I2 | Janela **sem moldura** (`decorations: false`) com barra própria de 36 px, sempre visível (também no login) e acima dos diálogos. | Voltar a `decorations: true` e remover `<TitleBar />` do `App`. |
| I3 | A barra só aparece dentro do Tauri; no navegador e nos testes não renderiza. | Nenhum. |
| I4 | Tamanho mínimo da janela 1024×640 e abrir centralizada. | Ajustar em `tauri.conf.json`. |

## Execução

- [x] Desenho em `src-tauri/icons/source/boreal.svg`; ícones regenerados por `npx tauri icon` (as pastas de Android e iOS e o `64x64.png` foram removidos). Conferido visualmente em 512 px e 32 px.
- [x] `src/components/Logo.tsx` (`LogoMark`, `Logo`); menu lateral, tela de entrada e favicon (`public/boreal.svg`).
- [x] `src/components/TitleBar.tsx` + `TitleBar.test.tsx` (5 testes: fora do Tauri não aparece; nome, área de arrastar e `--titlebar-h`; ações dos três botões; Restaurar quando maximizada; solta o ouvinte ao sair).
- [x] `App.tsx` mostra a barra em todas as situações; telas de altura total descontam `--titlebar-h`.
- [x] `tauri.conf.json` e `capabilities/default.json`; `cargo check` valida os nomes das permissões.
- [x] Documentação: `docs/desenvolvimento/design-system.md`, `CHANGELOG.md`, roteiro (Módulo 0, passo 2c).

## Pendências

- **Manual em `npm run tauri dev`:** arrastar, duplo clique, redimensionar pelas bordas, maximizar/restaurar, contraste dos botões, aparência do ícone na barra de tarefas e no instalador.
- Sem *snap layouts* do Windows 11 no botão de maximizar (limitação de janelas sem moldura).
- Fechar pela barra não passa pelo aviso de alterações não salvas (igual ao botão nativo anterior).
