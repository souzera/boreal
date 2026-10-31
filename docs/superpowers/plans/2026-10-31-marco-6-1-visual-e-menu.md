# Marco 6.1 — Identidade visual e menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aplicar a identidade do Boreal (teal, Cool Neutral, Onest, Material Symbols Rounded, Soft Elevation), reorganizar o menu lateral e a barra superior como no brief, sem mudar nenhuma regra de negócio.

**Architecture:** Só frontend. Tokens de tema centralizados em `src/index.css` (cores, estados dos quartos, raios, sombras, fonte), validados por um teste de contrato que confere contraste (WCAG) e ausência de recursos externos. Um componente `Icon` único passa a ser a única fonte de ícones; os símbolos de texto (`●`, `⚠`, `✓`…) usados como marca de estado viram `Icon`. O menu lateral vira grupos com rótulos; a barra superior ganha as ações globais com atalhos.

**Tech Stack:** React 19, Tailwind 4, shadcn/base-nova, Vitest. Dependências novas: `@fontsource-variable/onest` e `material-symbols` (empacotadas, nada de rede em tempo de execução); sai `@fontsource-variable/geist`.

**Spec:** `local/brief.md` (§1–§7 e §10) e `local/spec.md` §18. Roteiro: `local/plano-completo.md` §10 (Marco 6, parte 6.1 de 4). Isto é a parte **6.1**; 6.2 (alertas e busca), 6.3 (Recepção e visão gerencial) e 6.4 (relatórios e documentos) têm planos próprios.

**Branch:** `marco-6-1-visual`, a partir de `marco-5-financeiro` (Marco 5 ainda não integrado em `dev`). Sem migração de banco e sem comandos novos no backend.

## Decisões deste plano (chamadas minhas; revisar na tela pronta)

| # | Decisão | Por quê / custo se estiver errada |
|---|---|---|
| V1 | **Cores semânticas** (texto sobre fundo sutil): sucesso `#166534`/`#DCFCE7`, informação `#075985`/`#E0F2FE`, atenção `#92400E`/`#FEF3C7`, erro `#991B1B`/`#FEE2E2`. O teste de contraste exige ≥ 4,5:1; o implementador escurece o texto se algum par falhar. | O brief deixou em aberto. Trocar é mexer em variáveis CSS. |
| V2 | **Teal:** `--primary #0F766E`, hover `#115E59`, fundo sutil `#F0FDFA`, seleção `#CCFBF1`, foco `#0D9488` (≥ 3:1 sobre branco). | Referência do brief (`#0F766E`); variantes derivadas. |
| V3 | **Estados dos quartos** têm tokens próprios (`--state-*`) que *apontam* para as semânticas quando fazem sentido: aguardando limpeza = atenção; em limpeza = informação; disponível = sucesso; bloqueado/manutenção = erro; ocupado = neutro (`#E7EDF3`/`#172033`); limpo aguardando intervalo = índigo próprio (`#E0E7FF`/`#3730A3`), para não se confundir com disponível. | Brief §5.3: paleta operacional separada do feedback; ocupação e limpeza são dimensões distintas. |
| V4 | **Sem modo escuro** nesta parte: o bloco `.dark` fica como está (não projetado). | O brief não pede modo escuro; evita meia-implementação. |
| V5 | **Menu** em grupos com rótulo, todos visíveis (sem sanfona): Recepção · Reservas (Reservas, Hóspedes) · Hospedagens · Quartos (Mapa de quartos, Limpeza, Bloqueios, Manutenção, Cadastro de quartos) · Financeiro (Fechamento de caixa, Contas a pagar com permissão) · Relatórios e Alertas (**desabilitados, "Em breve"** até 6.4 e 6.2) · Configurações (Tarifas, Itens e serviços para todos; Configurações e Usuários só administrador). Quem via um item continua vendo (Tarifas e Itens seguem visíveis à recepção). | O brief lista oito entradas e não cita Hóspedes, Tarifas nem Itens; agrupei sem retirar acesso de ninguém. |
| V6 | **Ações globais:** *Nova reserva* → `/reservas/nova`; *Buscar* → foca a busca; *Check-in* → `/reservas`; *Receber pagamento* → `/hospedagens`; *Check-out* → `/hospedagens` (a pessoa escolhe a reserva ou a hospedagem e usa o botão específico). Atalhos: `Ctrl+K` buscar, `Alt+N` nova reserva, `Alt+C` check-in, `Alt+R` receber pagamento, `Alt+S` check-out; ignorados com diálogo aberto. | As telas finais de cada ação (com atalho direto ao check-in de hoje) chegam na 6.3; aqui a ação existe e leva ao lugar certo. |
| V7 | A tela inicial `/` passa a se chamar **Recepção** e mantém o conteúdo atual (limpeza e fechamento pendente); a Recepção de verdade é a 6.3. O **painel lateral reutilizável** fica para a 6.3, quando há quem o use (YAGNI). | Evita código sem uso. |
| V8 | Estados vazios com ilustração e a ajuda contextual ficam para a 6.3/6.4. | Fora do escopo visual básico. |

## Global Constraints

- Datas `DD/MM/AAAA`, horas `HH:MM`, valores `R$ 150,00` (brief §5.7). Textos em português claro.
- **Cor nunca é o único meio de comunicar estado:** cor + rótulo e, quando útil, ícone (brief §5.3, CLAUDE.md).
- **Offline:** fontes e ícones empacotados no bundle; nenhuma URL externa de fonte/ícone (brief §6.1, §7).
- Foco visível e navegação por teclado preservados (brief §6.2, WCAG 2.2).
- Ícones: Material Symbols **Rounded**, `FILL 1`, `wght 500`, `GRAD 0`, `opsz 24`; um único componente `Icon`; navegação 20 px, botões/tabelas 16 px, indicadores 20–22 px, ações compactas 14 px.
- Raios: cards 12 px, inputs 8 px, botões 8 px, diálogos 12 px, badges 6 px. Sombras sutis (Soft Elevation), nunca em todos os elementos.
- Neutros Cool Neutral: fundo `#F4F7FA`, superfície `#FFFFFF`, muted `#E7EDF3`, borda `#CBD5E1`, texto auxiliar `#64748B`, texto `#172033`. Sidebar branca.
- Números de dinheiro/data/hora com alinhamento tabular (`font-variant-numeric: tabular-nums`).
- Nenhuma regra de negócio, permissão ou contrato com o backend muda.
- Atualizar `CHANGELOG.md`, `docs/usuario/`, `docs/usuario/roteiro-de-treinamento.md`, `docs/desenvolvimento/`, este plano e o grafo (CLAUDE.md).
- Ambiente: `npm install --legacy-peer-deps`.

## Review Focus

1. **Contraste:** todo par texto/fundo dos tokens (principal, auxiliar, semânticas, estados dos quartos, botão primário) ≥ 4,5:1; foco ≥ 3:1 (T1).
2. **Menu sem rota quebrada ou acesso perdido:** todo link do menu existe em `appRoutes`; recepção e administrador veem exatamente os itens previstos (T5).
3. **Atalhos perigosos:** `Alt+N` etc. não disparam com diálogo aberto nem quando a sessão está bloqueada; `Ctrl+K` funciona com o foco num campo (T6).
4. **Offline:** nenhum `https://` de fonte/ícone no CSS nem no `index.html` (T1).
5. **Sobras de marca antiga:** nenhum símbolo `●◐◑◔○■✓✔✖⚠▲▽↩⏸` fora de `Icon`/testes de ícone, e todo estado de quarto/reserva/bloqueio/conta ainda tem ícone **e** rótulo (T3).

---

## File Structure

```
src/
  index.css                                  # tokens: cores, estados dos quartos, raios, sombras, fonte
  lib/contrast.ts (+ .test.ts)               # contrastRatio(hexA, hexB)
  theme.test.ts                              # contrato do tema (lê index.css)
  components/Icon.tsx (+ .test.tsx)          # Material Symbols Rounded
  components/layout/Sidebar.tsx              # grupos, ícones, estado ativo
  components/layout/NavItems.ts              # definição do menu (dados) usada por Sidebar e testes
  components/layout/GlobalActions.tsx        # Nova reserva · Buscar · Check-in · Receber pagamento · Check-out
  hooks/useGlobalShortcuts.ts (+ .test.tsx)
  components/layout/GlobalSearch.tsx         # aceita ref/foco; AppShell.tsx (cabeçalho e fundo)
  components/ui/{button,card,dialog,input,badge,select,textarea}.tsx   # raios e sombras
  components/{PageHeader,RoomStateBadge}.tsx
  lib/{roomState,reservation,stay,blocks}.ts # símbolos de texto -> nomes de ícone
  pages/** (marcas ⚠ ✓ ↩ … -> <Icon>)        # lista exata na T3
docs/usuario/*, docs/desenvolvimento/design-system.md
```

---

### Task 1: Dependências, fonte e tokens do tema

**Files:**
- Create: `src/lib/contrast.ts`, `src/lib/contrast.test.ts`, `src/theme.test.ts`
- Modify: `package.json`, `src/index.css`, `index.html` (se referenciar fonte)

**Interfaces:**
- Produces: `contrastRatio(foreground: string, background: string): number` (hex `#RRGGBB`, fórmula WCAG 2.x). Tokens CSS em `:root`: `--background --foreground --card --card-foreground --popover --popover-foreground --primary --primary-foreground --primary-hover --primary-subtle --primary-selected --secondary --secondary-foreground --muted --muted-foreground --accent --accent-foreground --border --input --ring --destructive`, semânticas `--success|--info|--warning|--danger` e `--*-foreground`/`--*-subtle` (fundo sutil + texto), estados `--state-occupied|awaiting-cleaning|cleaning|clean-waiting-interval|available|blocked` cada um com `-bg`, `-fg`, `-border`, raios `--radius` (8 px) e `--radius-card` (12 px), sombras `--shadow-card`, `--shadow-overlay`, e `--font-sans` com Onest. Classes Tailwind via `@theme inline`: `bg-state-occupied-bg`, `text-state-occupied-fg`, `border-state-occupied-border` (e as demais).

- [x] **Step 1: Write the failing tests**
  - `contrast.test.ts`: `contrastRatio("#000000","#FFFFFF")` ≈ 21; `contrastRatio("#777777","#FFFFFF")` ≈ 4,48 (toBeCloseTo 2 casas); simétrico; aceita minúsculas e `#abc` de 3 dígitos.
  - `theme.test.ts` (lê `src/index.css` com `node:fs`; extrai `--nome: #hex` do bloco `:root`):
    - `defines every required token` (lista acima);
    - `text pairs reach WCAG AA`: ≥ 4,5 para `foreground/background`, `foreground/card`, `muted-foreground/background`, `primary-foreground/primary`, `primary-foreground/primary-hover`, cada `*-foreground` sobre seu `*-subtle` (sucesso, informação, atenção, erro) e cada `state-*-fg` sobre `state-*-bg`;
    - `focus ring is visible`: `ring` sobre `card` ≥ 3;
    - `neutral palette matches the brief`: `background #F4F7FA`, `card #FFFFFF`, `muted #E7EDF3`, `border #CBD5E1`, `muted-foreground #64748B`, `foreground #172033`, `primary #0F766E`;
    - `radii and shadows exist`: `--radius` = `0.5rem`, `--radius-card` = `0.75rem`, `--shadow-card` e `--shadow-overlay` presentes;
    - `font is Onest and nothing is loaded from the network`: `index.css` importa `@fontsource-variable/onest`, `--font-sans` começa por `'Onest Variable'`, e nem `index.css` nem `index.html` contêm `http://` ou `https://` de fontes/ícones (`fonts.googleapis`, `fonts.gstatic`, `cdn`);
    - `numbers align`: `index.css` define `font-variant-numeric: tabular-nums` para `td, th`.
- [x] **Step 2: Run** `npx vitest run src/lib/contrast.test.ts src/theme.test.ts` — Expected: FAIL (arquivos/tokens inexistentes).
- [x] **Step 3: Implement** `contrast.ts`; `npm install --legacy-peer-deps @fontsource-variable/onest material-symbols` e `npm uninstall @fontsource-variable/geist`; reescrever o `:root` de `index.css` com os tokens (valores das Decisões V1–V3 e Global Constraints; `--ring: #0D9488`; `--destructive` = `--danger`), `@import "@fontsource-variable/onest"`, `--font-sans: 'Onest Variable', system-ui, sans-serif`, mapear os tokens no `@theme inline` (inclui `--color-state-*`), definir `--radius: 0.5rem`, `--radius-card`, sombras, e `td, th { font-variant-numeric: tabular-nums }`. Se algum par ficar abaixo de 4,5, escurecer o texto (não clarear o fundo).
- [x] **Step 4: Run** os mesmos testes e depois `npx vitest run` — Expected: PASS; o restante da suíte continua verde (nada de componente mudou ainda).
- [x] **Step 5: Commit** `feat: Boreal theme tokens, Onest font and contrast contract`

---

### Task 2: Componente `Icon` (Material Symbols Rounded)

**Files:**
- Create: `src/components/Icon.tsx`, `src/components/Icon.test.tsx`
- Modify: `src/index.css` (importar `material-symbols/rounded.css` — confirmar o caminho no pacote instalado — e a regra `.material-symbols-rounded { font-variation-settings: "FILL" 1, "wght" 500, "GRAD" 0, "opsz" 24 }`)

**Interfaces:**
- Produces: `type IconSize = "xs" | "sm" | "md" | "lg"` (14, 16, 20, 22 px) e `Icon({ name, size = "sm", className }: { name: string; size?: IconSize; className?: string })` renderiza `<span class="material-symbols-rounded" aria-hidden="true" data-icon={name} style={{ fontSize }}>{name}</span>` (o texto é o nome da ligadura). Também `export const iconSizePx: Record<IconSize, number>`.

- [x] **Step 1: Write the failing test** `Icon.test.tsx`: `renders the symbol name as a hidden ligature` (`aria-hidden="true"`, `data-icon="warning"`, classe `material-symbols-rounded`); `size maps to pixels` (`xs` 14, `sm` 16 padrão, `md` 20, `lg` 22 em `style.fontSize`); `merges className`; `index.css imports the packaged material-symbols stylesheet and sets FILL 1` (lê o CSS como em `theme.test.ts`).
- [x] **Step 2: Run** `npx vitest run src/components/Icon.test.tsx` — Expected: FAIL.
- [x] **Step 3: Implement** `Icon.tsx` e a importação/regra no CSS.
- [x] **Step 4: Run** o teste — Expected: PASS; `npx tsc --noEmit` limpo.
- [x] **Step 5: Commit** `feat: shared Icon component with packaged Material Symbols`

---

### Task 3: Marcas de estado viram `Icon` e os estados usam os tokens

**Files:**
- Modify: `src/lib/roomState.ts`, `src/lib/reservation.ts`, `src/lib/stay.ts`, `src/lib/blocks.ts`, `src/components/RoomStateBadge.tsx`, e as telas com marcas em texto: `src/pages/closing/ClosingPage.tsx`, `src/pages/payables/PayablesPage.tsx`, `src/pages/payments/{ComponentsEditor,DuplicateDialog}.tsx`, `src/pages/reservations/ReservationPage.tsx`, `src/pages/maintenance/MaintenanceOrderPage.tsx`, `src/pages/stays/{CheckInDialog,CheckoutDialog,StaysPage}.tsx` e onde mais `rg "[●◐◑◔○■✓✔✖⚠▲▽↩⏸]" src --glob "!*.test.*"` encontrar; os `*.test.tsx` que afirmam o glifo.
- Test: `src/components/RoomStateBadge.test.tsx` (novo), `src/lib/statusIcons.test.ts` (novo) e os testes de tela afetados.

**Interfaces:**
- Consumes: `Icon` (T2), tokens `--state-*` (T1).
- Produces: nos mapas de `lib/*`, o valor de ícone passa de glifo a **nome de símbolo** (`ROOM_STATE_STYLE[state].icon`, `PRIORITY_ICON`, `statusIcon` de reservas, `stayStatusIcon`, ícones de bloqueio/manutenção, `statusIcon` de contas a pagar). `ROOM_STATE_STYLE[state].className` passa a usar as classes de token (`bg-state-<estado>-bg text-state-<estado>-fg border-state-<estado>-border`). Nomes: ocupado `bed`, aguardando limpeza `cleaning_services`, em limpeza `autorenew`, limpo aguardando intervalo `hourglass_top`, disponível `check_circle`, bloqueado `block`; atenção/alerta `warning`; confere `check_circle`; devolução `undo`; prioridade `keyboard_double_arrow_up`/`keyboard_arrow_up`/`remove`/`keyboard_arrow_down`; reservas pendente `schedule`, confirmada `verified`, hospedada `bed`, concluída `task_alt`, cancelada `cancel`, no-show `person_off`; bloqueio preventivo `shield`, ativo `block`, aguardando liberação `pause_circle`, liberado `lock_open`; ordem aberta `radio_button_unchecked`, em andamento `autorenew`, aguardando resolução `pause_circle`, concluída `task_alt`; conta a pagar pendente `radio_button_unchecked`, parcial `timelapse`, paga `check_circle`, vencida `warning`.

- [x] **Step 1: Write the failing tests**
  - `statusIcons.test.ts`: `every room state has an icon, a label and token classes` (para cada `RoomStateKey`: `icon` não vazio sem caracteres de glifo, rótulo definido, `className` contém `bg-state-` e `text-state-`); o mesmo para status de reserva, hospedagem, bloqueio, ordem de manutenção, prioridade.
  - `RoomStateBadge.test.tsx`: `shows icon and label` (`[data-icon="cleaning_services"]` e texto "Aguardando limpeza"); `shows when a cleaned room becomes free` ("livre às 11:00"); `renders as a link when given a destination`.
  - `no old text markers remain`: teste que percorre `src/**/*.{ts,tsx}` (exceto `*.test.*`) e falha se achar `[●◐◑◔○■✓✔✖⚠▲▽↩⏸]` fora de comentários.
  - Ajustar, nos testes de tela existentes, as asserções de glifo (`toHaveTextContent("⚠")`, `"⚠ Vencida"` etc.) para a presença de `[data-icon="warning"]` + o texto do rótulo.
- [x] **Step 2: Run** `npx vitest run src/lib/statusIcons.test.ts src/components/RoomStateBadge.test.tsx` — Expected: FAIL (glifos ainda no código).
- [x] **Step 3: Implement** a troca nos `lib/*`, em `RoomStateBadge` e em cada tela listada: onde havia `<span aria-hidden>{glifo} </span>` ou o glifo dentro de um texto, usar `<Icon name=… size="xs|sm" />` seguido do rótulo; mensagens de texto simples (por exemplo `⚠ Saída 45 min depois do previsto…`) mantêm o texto e ganham o `<Icon name="warning" />` antes.
- [x] **Step 4: Run** `npx vitest run` (suíte inteira), `npx tsc --noEmit` — Expected: PASS; o teste de "sobras" verde.
- [x] **Step 5: Commit** `feat: status marks use Material Symbols and room states use theme tokens`

---

### Task 4: Menu lateral em grupos

**Files:**
- Create: `src/components/layout/NavItems.ts`
- Modify: `src/components/layout/Sidebar.tsx`, `src/components/layout/Sidebar.test.tsx`, `src/components/layout/AppShell.test.tsx` (se necessário)
- Test: `src/components/layout/NavItems.test.ts` (novo)

**Interfaces:**
- Consumes: `Icon` (T2), `usePermissions().can`, `useSessionStore` (papel).
- Produces: `type NavLeaf = { label: string; to?: string; icon: string; soon?: boolean; admin?: boolean; permission?: string }`, `type NavGroup = { label: string; icon: string; items: NavLeaf[] }`, `export const NAV: (NavLeaf | NavGroup)[]` na ordem das Decisões (V5) e `visibleNav(nav, { role, can }): (NavLeaf | NavGroup)[]` (puro; remove itens `admin` para recepção e `permission` sem a permissão; grupo sem itens some).

- [x] **Step 1: Write the failing tests**
  - `NavItems.test.ts`: `every link points to a real route` (cada `to` casa com algum `path` de `appRoutes`, tratando `:param` e prefixos); `no duplicate destinations`; `admin sees Configurações and Usuários, reception does not` (`visibleNav`); `payables only with the permission`; `Relatórios and Alertas are soon and have no destination`; `Tarifas and Itens e serviços stay visible to reception`.
  - `Sidebar.test.tsx` (ajustes e novos): grupos aparecem com rótulo (`Reservas`, `Quartos`, `Financeiro`, `Configurações` como cabeçalho, não link); `the active link is marked with aria-current and has a visible indicator`; itens "Em breve" têm `aria-disabled` e `title="Em breve"`; `recepção não vê o link Configurações` (agora por `queryByRole("link", { name: "Configurações" })`); Mapa de quartos, Limpeza, Bloqueios, Manutenção e Cadastro de quartos dentro do grupo Quartos; cada item com `[data-icon]`.
- [x] **Step 2: Run** `npx vitest run src/components/layout` — Expected: FAIL.
- [x] **Step 3: Implement** `NavItems.ts` (dados + `visibleNav`) e reescrever `Sidebar.tsx` renderizando a partir de `visibleNav`; cabeçalho do produto "Boreal PMS"; sidebar branca com borda e `shadow` discreto; ícones de 20 px; ativo = fundo `primary-selected`, texto `primary`, `aria-current="page"` e barra lateral de 3 px (indicador que não depende só de cor); rótulos de grupo em 12 px/500 `muted-foreground`.
- [x] **Step 4: Run** `npx vitest run src/components/layout` e a suíte inteira — Expected: PASS.
- [x] **Step 5: Commit** `feat: grouped sidebar menu with icons and route-integrity test`

---

### Task 5: Barra superior com ações globais e atalhos

**Files:**
- Create: `src/hooks/useGlobalShortcuts.ts`, `src/hooks/useGlobalShortcuts.test.tsx`, `src/components/layout/GlobalActions.tsx`, `src/components/layout/GlobalActions.test.tsx`
- Modify: `src/components/layout/AppShell.tsx`, `src/components/layout/GlobalSearch.tsx`, `src/components/layout/AppShell.test.tsx`

**Interfaces:**
- Consumes: `useNavigate`, `Icon`.
- Produces: `export const SHORTCUTS = { search: "Ctrl+K", newReservation: "Alt+N", checkIn: "Alt+C", receivePayment: "Alt+R", checkOut: "Alt+S" }`; `useGlobalShortcuts({ onSearch, onNavigate }: { onSearch: () => void; onNavigate: (to: string) => void }): void` registra um `keydown` no `window`; `GlobalSearch` passa a aceitar `ref` (ou `focusSignal`) para ser focado; `GlobalActions` renderiza cinco botões com ícone, texto e a dica do atalho (`title` e `aria-keyshortcuts`).
- Destinos (V6): `/reservas/nova`, `/reservas`, `/hospedagens`, `/hospedagens`.

- [x] **Step 1: Write the failing tests**
  - `useGlobalShortcuts.test.tsx`: `Ctrl+K calls onSearch even when an input has focus`; `Alt+N navigates to /reservas/nova` (e `Alt+C` → `/reservas`, `Alt+R` e `Alt+S` → `/hospedagens`); `ignores shortcuts while a dialog is open` (existe `[role="dialog"]` no documento); `ignores key repeat`; `ignores unrelated keys and Alt+N with extra modifiers`.
  - `GlobalActions.test.tsx`: `shows the five actions with icon and label`; `each action navigates`; `Buscar focuses the search field`; `shortcut hint is exposed` (`aria-keyshortcuts`).
  - `AppShell.test.tsx`: com sessão **bloqueada** as ações e o atalho não navegam nem aparecem utilizáveis (o conteúdo continua oculto como hoje); com sessão ativa o cabeçalho mostra as ações, a busca e o usuário.
- [x] **Step 2: Run** `npx vitest run src/hooks/useGlobalShortcuts.test.tsx src/components/layout` — Expected: FAIL.
- [x] **Step 3: Implement** o hook, `GlobalActions` e a integração em `AppShell` (cabeçalho em superfície branca com borda inferior; ações à direita da busca; "Sair" e o nome do usuário no canto); quando `LockGate` bloqueia, o hook não é registrado.
- [x] **Step 4: Run** a suíte inteira — Expected: PASS.
- [x] **Step 5: Commit** `feat: global actions and keyboard shortcuts in the top bar`

---

### Task 6: Geometria, elevação e tipografia dos componentes

**Files:**
- Modify: `src/components/ui/{button,card,dialog,input,badge,select,textarea,table}.tsx`, `src/components/PageHeader.tsx`, `src/components/layout/AppShell.tsx` (fundo `background`), `src/pages/Dashboard.tsx` (título **Recepção**), `src/pages/Dashboard.test.tsx`
- Test: `src/components/ui/geometry.test.tsx` (novo)

**Interfaces:**
- Consumes: tokens de raio/sombra (T1).
- Produces: botões e inputs com raio de 8 px, cards e diálogos com raio de 12 px e `shadow-card`/`shadow-overlay`, badges com 6 px; `PageHeader` com título 26 px/650; foco visível (`focus-visible:ring-*` com `--ring`); a página inicial exibe o título "Recepção" (conteúdo inalterado).

- [x] **Step 1: Write the failing tests** `geometry.test.tsx`: renderiza `Button`, `Input`, `Card`, `Dialog` aberto e `Badge` e confere as classes de raio/sombra esperadas (`rounded-lg` mapeado a 8 px pelo `--radius`, `rounded-xl` do card/diálogo mapeado a 12 px — verificar também em `index.css` que `--radius-xl` = `0.75rem`), `focus-visible:ring` presente em botão e input, e que o `PageHeader` usa `text-[26px]`/`font-semibold`; `Dashboard.test.tsx`: o título é "Recepção".
- [x] **Step 2: Run** `npx vitest run src/components/ui src/pages/Dashboard.test.tsx` — Expected: FAIL.
- [x] **Step 3: Implement** os ajustes (preferir tokens em `index.css` a valores soltos nos componentes; sombras só em cards, diálogos e menus).
- [x] **Step 4: Run** a suíte inteira, `npx tsc --noEmit`, `npm run build` — Expected: PASS.
- [x] **Step 5: Commit** `feat: Boreal geometry, elevation and page header typography`

---

### Task 7: Documentação, verificação e encerramento

**Files:**
- Create: `docs/desenvolvimento/design-system.md`
- Modify: `CHANGELOG.md`, `docs/usuario/README.md`, `docs/usuario/roteiro-de-treinamento.md` (menu do Módulo 0, nomes dos itens e atalhos), guias de `docs/usuario/*` que citam o menu antigo, `docs/desenvolvimento/README.md`, `local/plano-completo.md` (marcar 6.1), este plano

- [x] **Step 1: Docs:** `design-system.md` (tokens e como mudá-los, paleta dos estados e a relação com as semânticas, `Icon` e a lista de nomes usados, fonte empacotada, teste de contraste, como acrescentar item de menu em `NavItems.ts`, atalhos); `CHANGELOG.md` (`Changed`: novo visual, menu agrupado, ações no topo e atalhos, ícones e fonte empacotados; "Recepção" como nome da tela inicial); guias de usuário com o menu novo e a tabela de atalhos; roteiro (Módulo 0) com o menu e os atalhos.
- [x] **Step 2: Verificação completa:** `npx vitest run`, `npx tsc --noEmit`, `npm run build`, e `cargo test`/`clippy` só para confirmar que nada do backend mudou; registrar contagens no Status.
- [x] **Step 3:** `graphify update .`; anotar no Status.
- [x] **Step 4:** Revisão final da branch por revisor independente (modelo mais capaz), em especial o contraste, o menu por papel, os atalhos e as sobras de glifos; corrigir Críticos/Importantes (TDD); registrar menores adiados; atualizar Status, desvios e pendências.
- [x] **Step 5: Commit** `docs: close marco 6.1`

---

## Status

**Implementado e revisado** na branch `marco-6-1-visual` (a partir de `marco-5-financeiro`). T1 a T7 concluídas; revisão final independente feita (abaixo). Sem migração e sem mudança no backend.

- **Testes (última execução completa):** `npx vitest run` 425 testes em 57 arquivos, `npx tsc --noEmit` limpo e `npm run build` ok; `git diff marco-5-financeiro..HEAD -- src-tauri` vazio (backend intocado); `graphify update .` executado.
- **Pendências de verificação (manuais, responsável):** `npm run tauri dev` com a interface real, em especial: (a) se o WebView2 deixa `Alt+letra` chegar ao JavaScript sem apitar nem abrir menu do sistema; (b) se a barra de busca do WebView (`Ctrl+F`) casa o nome das ligaduras dos ícones; (c) largura do cabeçalho em 1024 e 1280 px; (d) se os tons dos estados dos quartos e a escala tipográfica agradam em tela real (o brief pede essa validação).

## Desvios e decisões da execução

- `--muted-foreground` ficou `#5B6B82`, e não o `#64748B` do brief (4,43:1 sobre o fundo; precisa de 4,5:1). A importação do Material Symbols e a regra `FILL 1` entraram na T1 (o teste de tema as exige).
- `NavGroup` não tem ícone próprio (só os itens). Os itens "Em breve" são `span` com `aria-disabled`.
- `PageHeader.description` virou `ReactNode` (renderizado em `div`) para aceitar ícone.
- O foco passou de `ring-3 ring-ring/50` (1,8:1) para um anel sólido de 2 px (`ring-ring`, ≥ 3:1) nos componentes de base; o hover do botão primário usa o token `primary-hover` (AA) em vez de `primary/80`.
- Slip de TDD: os testes de `Sidebar`/`AppShell` da T4 e da T5 foram escritos junto com a mudança; o RED observado foi o de módulo inexistente.

## Revisão final

Revisor independente (modelo mais capaz). Nenhum Crítico. Corrigidos em uma passada, cada um com teste que falhou antes:

- **Importante** — hover do botão primário falhava AA (`primary/80` ≈ 3,8:1) e o token `primary-hover` não era usado: agora `hover:bg-primary-hover` em botão e badge.
- **Importante** — foco quase invisível no botão primário (≈ 1,5:1): anel sólido de 2 px em botão, campo, seleção, texto, caixa de marcação e badge.
- **Importante** — cabeçalho quebrava em 3 linhas em janelas de 1024–1280 px: a busca agora encolhe (`flex-1 min-w-48`) e, abaixo de `xl`, as ações mostram só o ícone (o nome fica para leitores de tela).
- **Menores corrigidos (baratos):** caixa fixa de `1em` no `Icon` (evita o salto de layout enquanto a fonte de ícones carrega); botões pequenos e compactos com raio de 8 px; `alertdialog` também bloqueia os atalhos; repetir o atalho na própria tela não empilha histórico; a referência dos atalhos é atualizada em `useLayoutEffect`; testes de sub-rotas ativas no menu, de atalhos com diálogo real aberto e de `Ctrl+K` dentro de diálogo.

**Menores adiados:** a fonte de ícones inteira pesa 5,4 MB (subconjunto de ~40 nomes é possível depois); falta um teste de `Alt+tecla` com uma página suja sob `UnsavedChangesGuard`; as larguras do cabeçalho foram estimadas, não medidas; os pontos manuais acima.

## Marcos seguintes

- **6.2 — Alertas e busca:** alertas persistentes (tipos, prioridade, histórico, crítico não apagável, painel central; migração `0012`) e busca global por permissão. O item **Alertas** do menu fica "Em breve" até lá.
- **6.3 — Recepção e visão gerencial:** Recepção como tela inicial (resumo, mapa de 7 dias, chegadas, saídas, risco, painel lateral reutilizável, ações rápidas) e área gerencial só para administrador (métricas e gráficos). Ações globais passam a abrir o fluxo direto.
- **6.4 — Relatórios e documentos:** 10 relatórios com filtros, CSV, PDFs e documentos internos versionados; item **Relatórios** do menu.

## Self-Review

- **Cobertura do brief:** identidade teal, Cool Neutral, contraste, raios e sombras → T1/T6; Onest e números tabulares → T1; Material Symbols e escala de ícones → T2/T3; estados dos quartos com paleta própria e cor+rótulo+ícone → T1/T3; navegação lateral e ações globais com atalhos → T4/T5; Recepção/painel lateral/estados vazios → adiados para a 6.3 (V7, V8); "O que evitar" (recursos externos, cor sozinha) → testes de T1 e T3.
- **Consistência de tipos:** `Icon`/`IconSize` (T2) usado por T3, T4, T5; nomes de tokens (T1) usados em `RoomStateBadge` (T3) e no teste de geometria (T6); `NAV`/`visibleNav` (T4) usados no teste de rotas.
- **Proporção:** corpos ficam com o implementador; testes trazem valores exatos (hex, razões de contraste, raios, atalhos, nomes de ícone).
