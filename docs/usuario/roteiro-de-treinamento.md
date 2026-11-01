# Roteiro de treinamento — Boreal PMS

Roteiro para apresentar o sistema à equipe da pousada, na ordem do dia de trabalho da recepção. Ele cresce junto com o sistema: cada módulo novo entra aqui quando a funcionalidade fica pronta. Os detalhes de cada tela estão nos guias de [docs/usuario](README.md); aqui está só o que mostrar, em que ordem e o que observar.

**Estado atual:** módulos 0 a 12 (até o Marco 6.2). Ainda **não existem** relatórios e documentos internos (6.4), a Recepção completa (6.3) nem backup e restauração. Avise isso à equipe.

## Antes de começar

- **Cenário de demonstração:** use um banco de desenvolvimento preparado com o [seed](../desenvolvimento/seed.md). Ele cria os quartos 101 a 104 e 201 a 204 em situações diferentes (hospedado, aguardando limpeza, em limpeza, bloqueado, com manutenção, com reservas futuras). **Nunca** treine em cima do banco real.
- **Quem participa:** recepção (usuário `recepcao`) e uma pessoa administradora. Algumas etapas pedem permissão extra; elas estão indicadas.
- **Duração sugerida:** cerca de 2 horas no total, com pausas entre os módulos.
- **Regras provisórias:** os valores marcados com ⚠ ainda aguardam aprovação da pousada (Anexo A da spec). Diga à equipe que podem mudar.

## Módulo 0 — Entrar e se orientar (10 min)

- **Objetivo:** entrar no sistema e saber onde fica cada coisa.
- **Quem:** todos.
- **Passo a passo:**
  1. Entre com usuário e senha. No primeiro acesso, o sistema pede a troca da senha.
  2. Mostre o menu lateral, agrupado: **Recepção**; **Reservas** (Reservas, Hóspedes); **Hospedagens**; **Quartos** (Mapa de quartos, Limpeza, Bloqueios, Manutenção, Cadastro de quartos); **Financeiro** (Fechamento de caixa e, com permissão, Contas a pagar); **Configurações** (Tarifas e Itens e serviços; Configurações e Usuários só para o administrador). **Alertas** mostra um selo com a contagem; **Relatórios** aparece esmaecido com "Em breve". A tela atual fica destacada no menu.
  2b. Mostre as ações do topo e seus atalhos: **Nova reserva** (`Alt+N`), **Buscar** (`Ctrl+K`), **Check-in** (`Alt+C`), **Receber pagamento** (`Alt+R`) e **Check-out** (`Alt+S`). Por enquanto Check-in leva à lista de reservas e Receber pagamento e Check-out levam à lista de hospedagens.
  3. Mostre a busca no topo e a tela **Recepção** ("Quartos aguardando limpeza").
  4. Deixe a tela parada até o bloqueio por inatividade e desbloqueie com a senha: o que estava aberto volta como estava.
- **O participante deve ver:** que a sessão bloqueada esconde os dados e que nada digitado se perde.
- **Erros comuns:** senha errada várias vezes bloqueia a conta por alguns minutos (⚠ 5 tentativas, 5 minutos).
- **Guia:** [Usuários e permissões](usuarios.md), [Configurações](configuracoes.md).

## Módulo 1 — Cadastros: hóspedes, quartos e itens (15 min)

- **Objetivo:** cadastrar quem e o que o sistema usa no dia a dia.
- **Quem:** recepção (hóspedes); administrador (quartos, categorias, itens).
- **Passo a passo:**
  1. Em **Hóspedes → Novo hóspede**, cadastre alguém com telefone e CPF. Digite um nome parecido com o de outro cadastro e mostre o **alerta de possível duplicidade**.
  2. Em **Quartos**, mostre a lista e o campo **Intervalo de limpeza** de cada quarto (administrador).
  3. Em **Itens e serviços**, mostre o catálogo (água, lavanderia…).
- **O participante deve ver:** CPF e telefone formatados, a escolha entre usar o cadastro existente ou criar outro, e que desativar preserva o histórico.
- **Guias:** [Hóspedes](hospedes.md), [Quartos e categorias](quartos.md).

## Módulo 2 — Tarifas (10 min)

- **Objetivo:** entender de onde vem o preço de cada noite.
- **Quem:** administrador.
- **Passo a passo:** em **Tarifas**, mostre o valor por dia da semana, uma temporada, um feriado e uma exceção de data; use o **Simular preço**.
- **O participante deve ver:** que a data de saída não gera diária e que noite sem tarifa impede a reserva.
- **Guia:** [Tarifas](tarifas.md).

## Módulo 3 — Reservas (20 min)

- **Objetivo:** criar, alterar e cancelar reservas com segurança.
- **Quem:** recepção; desconto, alteração e exceções pedem permissão extra.
- **Passo a passo:**
  1. **Reservas → Nova reserva** (ou clique em "Livre" no Mapa). Mostre a disponibilidade e o preço noite a noite aparecendo na hora.
  2. Tente reservar um período bloqueado ou já ocupado: o sistema lista **o que impede** (reservas ou bloqueios, com motivo e período).
  3. **Registrar sinal recebido** ⚠ (30% do total, valor provisório) e veja a reserva passar de Pendente para Confirmada.
  4. **Alterar datas ou quarto**: leia preço anterior, novo e diferença; nada é gravado antes de **Confirmar alteração**.
  5. **Cancelar reserva**: mostre a devolução e a retenção calculadas ⚠ (política provisória) e a exceção que exige permissão e justificativa.
- **O participante deve ver:** que reserva pendente já bloqueia o quarto, que o preço combinado não muda quando a tabela muda e que **nada é cancelado sozinho** (sinal vencido só gera aviso).
- **Erros comuns:** "O quarto já está reservado nesse período" — o texto diz qual reserva atrapalha.
- **Guia:** [Reservas](reservas.md), [Mapa de quartos](mapa-de-quartos.md).

## Módulo 4 — Mapa de quartos (10 min)

- **Objetivo:** ler a situação dos quartos de relance.
- **Quem:** todos.
- **Passo a passo:**
  1. Abra o **Mapa de quartos** e percorra a legenda dos seis estados: Ocupado, Aguardando limpeza, Em limpeza, Limpo — aguardando intervalo, Disponível, Em manutenção ou bloqueado.
  2. Navegue por **Próximo período** e **Hoje**.
  3. Clique num quarto "Aguardando limpeza": o sistema abre a fila de Limpeza. Volte e veja que o mapa continua no mesmo período.
- **O participante deve ver:** que cada estado aparece com cor, símbolo e nome (quem não distingue cores consegue ler) e que o quarto sujo nunca aparece como disponível.
- **Guia:** [Mapa de quartos](mapa-de-quartos.md), [Limpeza](limpeza.md).

## Módulo 5 — Chegada: check-in e hospedagem sem reserva (15 min)

- **Objetivo:** receber o hóspede.
- **Quem:** recepção; check-in antecipado e excesso de capacidade pedem permissão extra.
- **Passo a passo:**
  1. Abra uma reserva e clique em **Confirmar check-in**: confira ocupantes e o saldo inicial.
  2. Mostre os avisos de **check-in antecipado** e de **excesso de capacidade** (cada um com sua permissão).
  3. Em **Hospedagens → Hospedagem sem reserva**, receba alguém sem reserva: o sistema confere disponibilidade e preço antes.
  4. Tente hospedar no **quarto aguardando limpeza (102)**: o sistema recusa e diz o motivo.
- **O participante deve ver:** "Check-in confirmado e salvo neste computador." e as diárias já lançadas na conta.
- **Erros comuns:** quarto sujo ou antes do intervalo mínimo de limpeza; hóspede anterior ainda no quarto.
- **Guia:** [Hospedagem](hospedagem.md).

## Módulo 6 — Conta do hóspede (15 min)

- **Objetivo:** lançar consumos e corrigir erros sem apagar nada.
- **Quem:** recepção; ajuste e estorno pedem permissão extra.
- **Passo a passo:**
  1. Na hospedagem do quarto 101, **Lançar consumo** (item do catálogo e um avulso).
  2. **Lançar ajuste** com justificativa (desconto/cobrança).
  3. **Estornar lançamento** e relançar com o valor certo: o original continua no histórico.
  4. **Registrar pagamento** ⚠ (provisório: uma forma de pagamento por vez) e confira o saldo.
- **O participante deve ver:** que lançamento confirmado nunca é editado nem apagado e que o saldo é sempre a soma do livro de lançamentos.
- **Guia:** [Conta do hóspede](conta-do-hospede.md).

## Módulo 7 — Saída: check-out e saída tardia (15 min)

- **Objetivo:** encerrar a hospedagem com a conta resolvida.
- **Quem:** recepção; saldo pendente e dispensa de cobrança pedem permissão extra.
- **Passo a passo:**
  1. **Concluir check-out** com a conta quitada e leia o demonstrativo.
  2. Mostre o check-out com **saldo em aberto**: exceção, justificativa e autorizador registrados.
  3. Mostre a **saída tardia** ⚠ (cobrança prevista de 50% da última diária, provisória): **Cobrar** ou **Dispensar** (permissão e justificativa) e o aviso de risco para a próxima reserva.
- **O participante deve ver:** "Check-out concluído e salvo neste computador." e que o quarto passa a **Aguardando limpeza**.
- **Guia:** [Hospedagem](hospedagem.md).

## Módulo 8 — Limpeza, bloqueios e manutenção (25 min)

- **Objetivo:** manter o quarto certo no estado certo.
- **Quem:** recepção (limpeza e abrir ordens); bloqueios e liberação pedem a permissão de liberar quarto.
- **Passo a passo:**
  1. **Limpeza:** mostre a fila ordenada por urgência ⚠ (prazos provisórios), **Iniciar limpeza**, **Concluir limpeza** e a mensagem "O quarto fica livre às HH:MM". Use **Ajustar prioridade** com justificativa.
  2. **Bloqueios → Criar bloqueio** em um quarto livre; tente em um quarto com reserva (recusado, com a lista) e mostre **Liberar quarto**.
  3. **Manutenção → Abrir ordem de manutenção** no quarto 202 (que tem reserva): bloqueio preventivo e reservas afetadas. Resolva a reserva, use **Aplicar indisponibilidade**, depois **Iniciar manutenção** e **Concluir manutenção**.
  4. Mostre que, **concluída a ordem, o quarto continua bloqueado** até alguém autorizado fazer **Liberar quarto**.
- **O participante deve ver:** que o tempo sozinho nunca libera um quarto sujo ou em manutenção e que o sistema nunca cancela reserva por conta própria.
- **Erros comuns:** "Resolva as reservas afetadas antes de iniciar a manutenção"; "Conclua a ordem antes de liberar o quarto".
- **Guias:** [Limpeza](limpeza.md), [Bloqueios e manutenção](bloqueios-e-manutencao.md).

## Módulo 9 — Receber e devolver (25 min)

- **Objetivo:** registrar o que entra e o que volta, sempre com a forma de pagamento.
- **Quem:** recepção; liberar recebimento duplicado pede a permissão **Liberar recebimento duplicado**.
- **Cenário:** hospedagem do quarto 101 (seed); uma reserva futura do quarto 204 e a reserva cancelada do quarto 201.
- **Passo a passo:**
  1. Na hospedagem do 101, veja a lista **Recebimentos**: um recebimento **dividido** (Pix R$ 60,00 + Dinheiro R$ 40,00) com uma devolução parcial já registrada.
  2. Clique em **Registrar pagamento**, informe R$ 100,00, clique em **Dividir em outra forma** e use Pix R$ 70,00 + Cartão de débito R$ 20,00: mostre **Falta R$ 10,00** e o botão bloqueado; corrija para R$ 30,00 e veja **Confere**. Confirme.
  3. Repita o mesmo pagamento no mesmo dia: o sistema avisa **possível recebimento duplicado**. Mostre **Voltar e corrigir** e, com a permissão, **Liberar recebimento duplicado** com justificativa.
  4. Em um recebimento, **Registrar devolução** com motivo; tente devolver mais do que resta (recusado).
  5. Abra a reserva cancelada do 201: **Devolução a fazer** e registre a devolução do sinal.
  6. Na reserva do 204, **Registrar pagamento** do sinal e veja a reserva passar a **Confirmada**.
- **O participante deve ver:** "Pagamento PAG-… registrado e salvo neste computador." e o saldo mudando na hora; que um pagamento salvo não se edita (corrige-se por devolução).
- **Erros comuns:** soma que não confere; devolução acima do recebido; pagamento em hospedagem já encerrada.
- ⚠ **Provisório:** bloqueio de duplicidade (valor, formas e dia), devolução sem permissão extra e devolução do cancelamento feita à mão.
- **Guia:** [Pagamentos e devoluções](pagamentos-e-devolucoes.md).

## Módulo 10 — Contas a pagar (15 min)

- **Objetivo:** controlar despesas e vencimentos da pousada.
- **Quem:** administrador, ou recepção com a permissão **Gerenciar contas a pagar**.
- **Cenário:** as três contas do seed (pendente, parcial e vencida).
- **Passo a passo:**
  1. Abra **Contas a pagar** e mostre as situações com texto e ícone (**Vencida** com ⚠).
  2. **Registrar pagamento da conta** na conta parcial, com um valor menor que o saldo, e depois o restante: a situação vai a **Paga**.
  3. **Registrar conta a pagar** marcando **Já paga à vista**.
  4. Alterne os totais entre **Competência** e **Pagamento** e explique a diferença com a conta paga em partes.
  5. Como administrador, crie e desative uma categoria (a desativada some das novas contas, o histórico fica).
- **O participante deve ver:** que a situação se atualiza sozinha e que pagamento acima do saldo é recusado.
- ⚠ **Provisório:** lista inicial de categorias.
- **Guia:** [Contas a pagar](contas-a-pagar.md).

## Módulo 11 — Fechamento de caixa (15 min)

- **Objetivo:** fechar o dia conferindo cada forma de pagamento.
- **Quem:** recepção.
- **Passo a passo:**
  1. Abra **Fechamento de caixa**: mostre recebido, devolvido e esperado por forma (débito e crédito separados).
  2. Digite os valores conferidos; em uma forma, digite um valor diferente e veja **Falta**/**Sobra** e o pedido de justificativa.
  3. **Concluir fechamento** e leia a confirmação com as diferenças.
  4. Reabra o dia: somente leitura, com quem fechou e quando. Faça um novo pagamento e veja **Movimentos depois do fechamento**.
  5. Mostre o alerta **Fechamento de caixa pendente** (aparece para dias anteriores sem fechamento) e que nada fica bloqueado.
- **O participante deve ver:** que diferença não impede o fechamento e que o fechamento não muda nenhum recebimento.
- **Erros comuns:** esquecer o valor de alguma forma; tentar fechar o mesmo dia duas vezes.
- ⚠ **Provisório:** pagamentos a fornecedores não entram no esperado (decisão da pousada pendente).
- **Guia:** [Fechamento de caixa](fechamento-de-caixa.md).

## Módulo 12 — Alertas e busca (20 min)

- **Objetivo:** saber o que exige atenção agora e achar qualquer registro rápido.
- **Quem:** todos; "Conta vencida" só aparece para quem tem **Gerenciar contas a pagar**.
- **Cenário:** o seed já deixa alertas prontos (reserva afetada por manutenção, conta vencida e a chegada de hoje do quarto 201).
- **Passo a passo:**
  1. Na **Recepção**, leia os alertas mais urgentes; o crítico vem primeiro. Mostre o selo do menu **Alertas**.
  2. Abra **Alertas**: prioridade em ícone e texto, "visto desde", **Abrir registro**. Clique em **Abrir registro** do conflito de manutenção e volte.
  3. Em um alerta de prioridade Normal ou Alta, **Dispensar alerta**: a justificativa é obrigatória; ele vai para o **Histórico** com o seu nome. Mostre que o alerta **crítico** não tem esse botão e explica por quê.
  4. Resolva a causa (por exemplo, faça o check-in da reserva de hoje) e mostre o alerta indo sozinho para o Histórico em até um minuto.
  5. Busca: `Ctrl+K`, digite `ana` (hóspedes, reservas, hospedagens e movimentos), depois `PAG-` e o número de um quarto (`101`). Mostre que 1 caractere só mostra a orientação e que "Mostrando 8 de N" pede para refinar.
- **O participante deve ver:** que nada é apagado, que crítico só some quando o problema é resolvido e que cada resultado abre o registro certo.
- **Erros comuns:** esquecer a justificativa; procurar "Conta vencida" sem a permissão.
- ⚠ **Provisório:** tipos e prioridades dos alertas (Anexo A.6).
- **Guias:** [Alertas](alertas.md), [Busca](busca.md).

## Fechamento do treinamento (5 min)

- Reforce: o sistema funciona **sem internet**, e cada tela confirma "salvo neste computador".
- Reforce: ao errar, **o que foi digitado continua na tela** e a mensagem diz como corrigir.
- Peça que cada pessoa refaça sozinha uma chegada, um consumo e uma saída no cenário de demonstração.

## Ainda por vir

Entram neste roteiro quando existirem: Recepção completa (6.3), relatórios e documentos internos (6.4); backup e restauração.
