# CLAUDE.md — Boreal

## Projeto

Boreal é um PMS desktop offline para a operação interna de uma única pousada, usado em um computador da recepção.

A fonte de verdade funcional é `local/spec.md`. Leia-a antes de implementar regras de negócio, fluxos críticos, mudanças de dados ou decisões arquiteturais.

## Stack

- Desktop: Rust + Tauri
- Interface: React + TypeScript
- Estilos: Tailwind CSS + shadcn/ui
- Navegação: React Router
- Formulários e validação: React Hook Form + Zod
- Estado de interface: Zustand
- Consultas e mutações: TanStack Query
- Banco local: SQLite
- Datas: date-fns
- Gráficos: Recharts

O frontend nunca é a fonte de verdade para disponibilidade, valores, permissões, saldos ou validações críticas. Essas regras devem ser validadas novamente no backend Rust no momento da gravação.

## Princípios de produto

O Boreal deve ser simples e intuitivo para a equipe da recepção.

- Use a linguagem da pousada: chegada, saída, quarto pronto, limpeza, bloqueio, recebimento e conta do hóspede.
- Priorize o trabalho de hoje: chegadas, saídas, quartos, limpeza, reservas e pendências.
- Preserve o contexto: abrir uma reserva ou quarto não deve fazer a pessoa perder a agenda em que estava.
- Mostre claramente o estado atual, o que exige ação, o que impede uma ação e o resultado após salvar.
- Prefira uma ação principal clara por tela ou contexto.
- Use textos específicos em ações importantes: `Confirmar check-in`, `Concluir limpeza`, `Registrar pagamento`.
- Não use cor como única forma de comunicar estado; combine cor, rótulo e, quando útil, ícone.
- Preserve dados preenchidos quando uma validação falhar e explique como corrigir o problema.
- Confirme apenas ações críticas, financeiras, destrutivas ou irreversíveis.
- Use datas em `DD/MM/AAAA`, horários em `HH:MM` e valores em reais, por exemplo `R$ 150,00`.

## Regras críticas

- Reservas devem respeitar disponibilidade, capacidade, tarifas, bloqueios e intervalo de limpeza.
- Alterações de reserva devem mostrar valor anterior, novo valor e diferença antes da confirmação.
- Check-in, checkout, desconto, estorno, devolução, cancelamento, alteração de reserva e liberação de quarto exigem as permissões e justificativas definidas na spec.
- Lançamentos financeiros confirmados não são editados silenciosamente; correções acontecem por estorno e relançamento.
- Operações relacionadas devem usar transações no backend.
- Não permita gravações parciais, duplicidades ou sobrescrita silenciosa.
- Auditoria de negócio deve ser registrada na mesma transação da operação crítica.
- O aplicativo precisa continuar útil offline e confirmar claramente quando uma operação foi salva localmente.
- Falhas de backup não devem bloquear a recepção, mas precisam gerar alerta persistente.

## Segurança e privacidade

- Nunca exponha senhas, chaves, dados bancários ou informações sensíveis em logs, mensagens de erro, fixtures ou documentação.
- Não adicione banco real, backups reais, credenciais ou dados pessoais ao repositório.
- Respeite permissões no backend, e não apenas ocultando botões na interface.
- Preserve histórico e auditoria; não apague registros que possuem dependências históricas.

## Implementação

Antes de alterar código:

1. Leia `local/spec.md`, `README.md`, documentação relevante e instruções locais.
2. Use o Graphify para entender módulos e relações antes de mudanças não triviais.
3. Identifique regras de negócio, impactos em dados, permissões, auditoria e testes.
4. Prefira mudanças pequenas, coesas e consistentes com a arquitetura existente.

Durante a implementação:

- Mantenha separadas as camadas de comandos, serviços, repositórios, modelos, autenticação, auditoria e migrações.
- Valide entradas no frontend para boa experiência, mas revalide regras críticas no backend.
- Use valores monetários como centavos inteiros.
- Diferencie datas de calendário de instantes temporais.
- Não introduza API HTTP local se a comunicação Tauri já atende ao caso.
- Não implemente integração bancária, fiscal, nuvem, sincronização entre computadores ou estoque sem uma decisão explícita.
- Não altere regras de negócio aprovadas sem sinalizar o impacto e pedir confirmação.

## Testes

Ao alterar comportamento, adicione ou atualize testes proporcionais ao risco.

Dê prioridade a testes para:

- Disponibilidade, tarifas, capacidade, reservas e conflitos.
- Check-in, limpeza, checkout e bloqueios.
- Pagamentos parciais, divididos, devoluções e estornos.
- Permissões, auditoria e histórico.
- Fechamento de caixa.
- Backup, restauração, criptografia, migrações e recuperação.
- Persistência após fechar e reabrir o aplicativo.

Execute os comandos de teste, lint e formatação já configurados pelo projeto antes de finalizar alterações relevantes. Não invente comandos: confira os scripts e a documentação existentes.

## Documentação e changelog

Ao implementar uma funcionalidade, correção ou mudança relevante, atualize a documentação aplicável no mesmo trabalho.

### CHANGELOG.md

Atualize `CHANGELOG.md` na seção `Unreleased` usando:

- `Added` para funcionalidades novas.
- `Changed` para mudanças de comportamento.
- `Fixed` para correções.
- `Removed` para remoções.
- `Security` para alterações de segurança.

Descreva impacto percebido pelo produto, não detalhes internos de refatoração.

Exemplo:

```md
## [Unreleased]

### Added
- Adicionado bloqueio de checkout com saldo pendente, com exceção autorizada e auditada.
```

### Documentação para usuários

Crie ou atualize arquivos em `docs/usuario/`.

A documentação para usuários finais deve:

- Usar linguagem simples e sem jargão técnico.
- Explicar objetivo, quando utilizar, passo a passo e resultado esperado.
- Informar avisos, permissões ou condições relevantes.
- Orientar como agir diante de erros comuns.
- Usar imagens somente quando realmente ajudarem a concluir a tarefa.

### Documentação para desenvolvimento

Crie ou atualize arquivos em `docs/desenvolvimento/`.

A documentação para a equipe deve registrar:

- Regras de negócio e fluxos.
- Decisões técnicas relevantes e justificativas.
- Permissões, validações, auditoria e efeitos no banco.
- Migrações, contratos internos e integrações.
- Casos de erro, limitações conhecidas e estratégia de testes.

Documente sempre mudanças que afetem fluxo de usuário, regras de negócio, dados, permissões, auditoria, segurança, backup, recuperação ou operação financeira.

Não crie documentação para alterações internas triviais sem impacto duradouro.

A implementação só está completa quando código, testes, `CHANGELOG.md` e documentação aplicável estiverem atualizados.

## Planos de implementação

Os planos ficam em `docs/superpowers/plans/`. O plano é o registro vivo do trabalho, e não só um ponto de partida.

**A cada ciclo** (tarefa concluída, correção da revisão, mudança de escopo ou fim de marco), atualize o plano correspondente no mesmo trabalho:

- Marque os passos concluídos (`- [x]`); deixe `- [ ]`, com uma nota, o que não foi executado ou verificado.
- Atualize a seção `Status` no topo: situação, última execução de testes e pendências de verificação.
- Registre desvios do plano e decisões tomadas (o que mudou, por quê e o custo se estiver errado).
- Registre pendências adiadas e itens que a revisão deixou de fora.
- Ajuste interfaces, nomes e comandos do plano que deixaram de refletir o código.
- Ao iniciar um novo marco, crie um plano novo e liste-o em "Marcos seguintes" do anterior.

Não marque um passo como concluído sem evidência (teste ou comando executado).

## Graphify

Use o Graphify para navegar alterações não triviais antes de pesquisar o código manualmente.

- Consulte `graphify-out/GRAPH_REPORT.md` quando a tarefa envolver arquitetura, dependências ou impacto entre módulos.
- Prefira consultas focadas, por exemplo:

```bash
graphify query "o que conecta reservas à disponibilidade?"
```

- Após mudanças de código ou documentação, atualize o grafo:

```bash
graphify update .
```

### Pull na branch dev

Sempre que executar `git pull` na branch `dev`, execute imediatamente:

```bash
graphify update .
```

A atualização deve ocorrer após o pull terminar com sucesso e antes de iniciar análise, implementação ou documentação.

Se a atualização falhar, informe a falha e não considere o grafo atualizado.

## Checklist antes de finalizar

- [ ] A implementação segue `local/spec.md`.
- [ ] Regras críticas foram validadas no backend.
- [ ] Permissões e auditoria foram consideradas.
- [ ] Não há alteração financeira silenciosa ou perda de histórico.
- [ ] Foram adicionados ou atualizados testes adequados.
- [ ] `CHANGELOG.md` foi atualizado quando aplicável.
- [ ] A documentação para usuário e/ou desenvolvimento foi atualizada quando aplicável.
- [ ] O plano em `docs/superpowers/plans/` foi atualizado (passos, status, desvios e pendências).
- [ ] O Graphify foi atualizado após as mudanças.
- [ ] Não foram incluídos dados reais, credenciais, bancos ou backups.