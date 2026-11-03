# Design system do Boreal (Marco 6.1)

Identidade visual, tokens e componentes de base. Fonte: `local/brief.md` §5 e §6; plano: `docs/superpowers/plans/2026-10-31-marco-6-1-visual-e-menu.md`. Só frontend: nenhuma regra de negócio, permissão ou contrato com o backend muda.

## Tokens (`src/index.css`)

Todas as cores, raios e sombras vivem em variáveis CSS no bloco `:root` e são expostas ao Tailwind em `@theme inline`. **Não use valores soltos nas telas**; use as classes dos tokens (`bg-primary`, `text-muted-foreground`, `bg-state-blocked-bg`, `shadow-card`…).

| Grupo | Tokens | Observação |
|---|---|---|
| Neutros (Cool Neutral) | `background #F4F7FA`, `card/popover #FFFFFF`, `muted/secondary/accent #E7EDF3`, `border/input #CBD5E1`, `foreground #172033`, `muted-foreground #5B6B82` | O brief sugeria `#64748B`, que dá 4,43:1 sobre o fundo; foi escurecido para atingir AA (4,5:1). |
| Identidade (teal) | `primary #0F766E`, `primary-hover #115E59`, `primary-subtle #F0FDFA`, `primary-selected #CCFBF1`, `ring #0D9488` | Ação primária, item ativo do menu e foco. |
| Semânticas | `success`, `info`, `warning`, `danger`, cada uma com `-subtle` (fundo) e `-foreground` (texto) | Cores e texto escolhidos pelo plano (V1); trocar é editar as variáveis. |
| Estados dos quartos | `state-<estado>-bg/-fg/-border` para `occupied`, `awaiting-cleaning`, `cleaning`, `clean-waiting-interval`, `available`, `blocked` | Paleta própria: ocupado neutro; aguardando limpeza = atenção; em limpeza = informação; disponível = sucesso; bloqueado = erro; limpo aguardando intervalo = índigo próprio (para não parecer disponível). |
| Geometria | `--radius 0.5rem` (botões, campos), `--radius-card 0.75rem` (cards, diálogos), badges `rounded-md` | |
| Elevação (Soft Elevation) | `--shadow-card`, `--shadow-overlay` | Só em cards, painéis e diálogos/menus. |
| Fonte | `--font-sans: 'Onest Variable'` | Empacotada (`@fontsource-variable/onest`); nada é carregado da rede. |
| Números | `td, th { font-variant-numeric: tabular-nums }` | Valores e datas alinhados nas colunas. |

O **modo escuro** não faz parte da v1: o bloco `.dark` é o padrão do shadcn, sem projeto.

### Teste de contrato

`src/theme.test.ts` lê o CSS e falha se: faltar algum token; um par texto/fundo ficar abaixo de 4,5:1 (`foreground`, `muted-foreground`, botão primário, semânticas, estados dos quartos); o anel de foco ficar abaixo de 3:1; dois estados de quarto tiverem o mesmo fundo; a fonte não for a Onest; ou houver `http(s)://` / Google Fonts no CSS ou no `index.html`. Para mudar uma cor, edite `index.css` e rode `npx vitest run src/theme.test.ts`.

## Ícones

`src/components/Icon.tsx` é a **única** fonte de ícones: Material Symbols Rounded, preenchido (`FILL 1`, `wght 500`, `GRAD 0`, `opsz 24`), do pacote `material-symbols` empacotado no app.

```tsx
<Icon name="warning" size="xs" className="mr-1 align-middle" />
```

Tamanhos: `xs` 14 px (ações compactas), `sm` 16 px (padrão: botões e tabelas), `md` 20 px (navegação), `lg` 22 px (indicadores). O ícone é decorativo (`aria-hidden`): o significado vem sempre do rótulo ao lado, nunca só do ícone ou da cor.

Os mapas de estado em `src/lib/{roomState,reservation,stay,blocks}.ts` e `PayablesPage` guardam **nomes de símbolo**. `src/lib/statusIcons.test.ts` garante que todo estado tem ícone e rótulo, que os estados dos quartos usam as classes de token, e que **nenhum símbolo de texto antigo** (`●◐◑◔○■✓✔✖⚠▲▽↩⏸`) sobra no código.

## Logo, ícone e barra de título

- **Fonte do desenho:** `src-tauri/icons/source/boreal.svg` (quadrado arredondado com gradiente teal `#0F766E → #0B3F3B`, três fitas de aurora e uma estrela). O mesmo desenho está em `src/components/Logo.tsx` (`LogoMark` decorativo; `Logo` com o nome "Boreal PMS" como texto acessível) e em `public/boreal.svg` (favicon).
- **Regenerar os ícones do aplicativo** depois de mudar o SVG: `npx tauri icon src-tauri/icons/source/boreal.svg -o src-tauri/icons` e apagar as pastas `android/` e `ios/` (e `64x64.png`) que o comando também cria, porque o app é só desktop.
- **Barra de título** (`src/components/TitleBar.tsx`): a janela usa `decorations: false` em `tauri.conf.json` (com `minWidth` 1024 e `minHeight` 640) e a barra desenha o papel dela. Fica `sticky` no topo, **acima dos diálogos** (`z-[60]`), para minimizar e fechar nunca ficarem inacessíveis. Só aparece dentro do Tauri (`__TAURI_INTERNALS__`); no navegador e nos testes não renderiza nada.
- **Altura da barra:** a `TitleBar` define `--titlebar-h` (36px) no `:root`; as telas de altura total usam `min-h-[calc(100vh-var(--titlebar-h))]` em vez de `min-h-screen`. Em tela nova de página inteira, use o mesmo padrão.
- **Permissões** (`capabilities/default.json`): `core:window:allow-minimize`, `-toggle-maximize`, `-internal-toggle-maximize` (duplo clique), `-is-maximized`, `-close` e `-start-dragging`.
- **Limitações:** sem a moldura nativa, o Windows 11 não mostra os *snap layouts* ao passar o mouse no botão de maximizar (arrastar para as bordas e `Win+setas` continuam funcionando); fechar pela barra não passa pelo aviso de alterações não salvas, como já era com o botão nativo.
- **Pendente de conferência manual:** arrastar, redimensionar pelas bordas, maximizar/restaurar e o contraste dos botões em `npm run tauri dev`.

## Menu e ações globais

- O menu é dado, não JSX: `src/components/layout/NavItems.ts` (`NAV`, `visibleNav`). Para acrescentar um item, inclua-o em `NAV` com `icon` (nome do símbolo) e `to`; use `admin: true` ou `permission: "..."` para restringir. `NavItems.test.ts` falha se um destino não existir em `appRoutes`, se houver destino repetido ou se um item não tiver ícone.
- Itens ainda inexistentes ficam com `soon: true` (aparecem desabilitados com "Em breve"): hoje só **Relatórios** (6.4). **Visão gerencial** (`/gerencial`) e **Saúde do sistema** (`/saude`) são `admin: true`.
- A barra superior (`GlobalActions`) traz **Nova reserva**, **Buscar**, **Check-in**, **Receber pagamento** e **Check-out**, com atalhos de `useGlobalShortcuts`: `Ctrl+K` busca; `Alt+N`, `Alt+C`, `Alt+R` e `Alt+S` abrem nova reserva, check-in, receber pagamento e check-out. Não disparam com diálogo aberto, com a sessão bloqueada, com tecla repetida nem com outros modificadores. As teclas são reconhecidas pelo código físico (`KeyN`…).
- *Check-in* (`Alt+C`) e *Check-out* (`Alt+S`) abrem as abas **Chegadas** (`/?aba=chegadas`) e **Saídas** (`/?aba=saidas`) da Recepção; *Receber pagamento* (`Alt+R`) leva à lista de **Hospedagens**.
- **Painel lateral** (`SidePanel`): `aside` não modal para resumos ao lado da agenda; a seleção fica na URL (`?painel=`). Veja [Recepção](recepcao.md).

## Fora do escopo da 6.1

Estados vazios com ilustração, ajuda contextual e o modo escuro. O brief ainda pede validar os tons dos estados e a escala tipográfica em telas reais.
