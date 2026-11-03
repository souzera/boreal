# Changelog

Todas as mudanças relevantes do Boreal PMS são registradas aqui.
Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [Unreleased]

### Added
- Fundação do sistema: aplicativo desktop com banco de dados local criptografado, cuja chave fica no cofre de credenciais do Windows.
- Login com usuário e senha, troca de senha obrigatória no primeiro acesso, bloqueio temporário da conta após tentativas inválidas e bloqueio da sessão por inatividade.
- Perfis de administrador e recepção, com permissões verificadas no sistema e não apenas na tela.
- Registro de auditoria que não pode ser alterado nem apagado.
- Tela de **Usuários**: criar pessoas, redefinir senha, trocar perfil, desativar/reativar e liberar permissões extras para a recepção (desconto, estorno, exceder capacidade, exceção de cancelamento, alterar reserva, liberar quarto).
- Tela de **Configurações**: dados da pousada (com logotipo), tempo de inatividade, tentativas e bloqueio da conta, horários padrão de check-in/check-out, tolerâncias e intervalo de limpeza; mostra versão do aplicativo, do banco e data da última atualização.
- Cadastro de **quartos e categorias**, com busca, filtro, ordenação e desativação que preserva o histórico.
- Cadastro de **hóspedes** com código sequencial, CPF e telefone formatados e validados, busca sem diferenciar acentos e **alerta de possível duplicidade** (nome, telefone ou CPF) com escolha entre usar o cadastro existente ou criar outro.
- Busca geral no topo da tela.
- Aviso ao sair de uma tela com alterações não salvas (Salvar, Descartar ou Continuar editando).

- **Tarifas** por categoria (dia da semana, temporada, feriado e exceção de data), calendário de feriados e simulador de preço. A data de saída não gera diária e noite sem tarifa impede a reserva.
- **Reservas**: lista com filtros, nova reserva com disponibilidade e preço de cada noite ao vivo, e tela da reserva com histórico. O quarto fica bloqueado desde a reserva pendente e o preço combinado de cada noite não muda quando a tabela muda.
- **Alterar datas ou quarto** com prévia (preço anterior, novo e diferença) e justificativa; nada é gravado antes da confirmação, e uma prévia desatualizada é recusada.
- **Cancelamento** com devolução e retenção calculadas pela política (provisória e configurável), exceção com permissão e justificativa, **no-show manual** e aviso de **sinal vencido** (nada é cancelado automaticamente).
- **Desconto** com permissão e justificativa.
- **Mapa de quartos**: grade quarto × dia, com chegada, saída e situação em texto.
- Configurações do sinal e da política de cancelamento, com os valores marcados como provisórios.
- **Hospedagem**: **check-in** da reserva com ocupantes, saldo inicial e avisos de check-in antecipado e de excesso de capacidade (cada um com sua permissão); **hospedagem sem reserva** com disponibilidade e preço conferidos antes de iniciar; lista de **Hospedagens** com saldo e avisos.
- **Conta do hóspede** em livro de lançamentos que nunca se edita: diárias, desconto e sinal vindos da reserva, **consumos e despesas extras** (avulsos ou do catálogo de **Itens e serviços**), **ajustes** com justificativa e **estorno com relançamento** que preserva o lançamento original e impede cobrança duplicada.
- **Check-out** com demonstrativo (diárias, consumos, descontos, recebimentos, devoluções e saldo); com saldo em aberto ou crédito do hóspede só com permissão, justificativa e autorizador registrados — nada é liquidado nem apagado sozinho.
- **Saída tardia**: tolerância por hospedagem, cobrança prevista provisória (50% da última diária, configurável) que a recepção cobra ou dispensa (com permissão e justificativa), sem criar diária; aviso de **risco para a próxima reserva** quando o atraso encosta na limpeza.
- Novas permissões concedíveis à recepção: **Fazer check-in antecipado**, **Concluir check-out com saldo pendente** e **Dispensar cobrança de saída tardia**.
- **Limpeza dos quartos**: ao concluir o check-out, o quarto fica **aguardando limpeza** e entra numa fila ordenada por urgência (pela próxima chegada, com ajuste manual justificado). Iniciar e concluir a limpeza registram quem e quando. O quarto só volta a receber hóspedes depois da limpeza concluída e do intervalo mínimo, que agora pode ser definido por quarto.
- **Estados do quarto no mapa**: Ocupado, Aguardando limpeza, Em limpeza, Limpo — aguardando intervalo, Disponível e Em manutenção ou bloqueado, sempre com cor, símbolo e nome, e legenda. A data do mapa fica no endereço, então abrir a fila de limpeza e voltar não perde a agenda. O Dashboard mostra quantos quartos aguardam limpeza.
- **Bloqueios de quarto** por período ou até a liberação manual, que impedem reservas, alterações, check-in e hospedagem sem reserva no período; **liberar o quarto** exige a permissão e uma justificativa.
- **Ordens de manutenção** (aberta, em andamento, aguardando resolução, concluída) com **reservas afetadas** listadas e bloqueio preventivo até serem resolvidas. Concluir a ordem não libera o quarto: a liberação continua dependendo de quem tem autorização.
- **Recebimentos divididos**: o sinal e os pagamentos da conta aceitam várias formas de pagamento (dinheiro, Pix, débito, crédito e transferência), com a soma conferida na tela e no sistema; cada recebimento tem número próprio (`PAG-…`).
- **Bloqueio de recebimento duplicado**: mesmo valor, formas e dia na mesma reserva ou hospedagem não é gravado até alguém com permissão liberar, com justificativa registrada.
- **Devoluções** ligadas ao recebimento original, parciais ou totais, com motivo, divididas por forma de pagamento e com número próprio (`DEV-…`); a reserva cancelada mostra **Devolução a fazer**.
- **Contas a pagar** com fornecedor, vencimento e categoria, pagamentos parciais, despesas pagas à vista, situação (pendente, parcial, paga e vencida) e totais por competência ou por data de pagamento; categorias de despesa editáveis e desativáveis.
- **Fechamento de caixa** diário por forma de pagamento (débito e crédito separados), com diferença justificada, sem alterar recebimentos e sem bloquear o dia seguinte; o Dashboard avisa dos dias sem fechamento.
- Permissões **Liberar recebimento duplicado** e **Gerenciar contas a pagar**, concedíveis à recepção.

- **Alertas persistentes** com prioridade (Crítico, Alto e Normal, sempre com ícone e texto), tela **Alertas** com filtros e histórico, selo de contagem no menu e os mais urgentes na tela **Recepção**: chegadas e saídas de hoje, chegada atrasada, sinal vencido, saída atrasada, risco para a próxima reserva, reserva afetada por manutenção, limpeza urgente, conta vencida, saldo pendente na saída e fechamento de caixa pendente. Cada alerta abre o registro de origem e some sozinho quando o problema é resolvido.
- **Dispensar alerta** com justificativa (fica no histórico e na auditoria); alertas críticos não podem ser dispensados; nenhum alerta é apagado.
- **Busca global** por hóspedes, reservas, hospedagens e movimentos financeiros (nome sem acento, telefone, CPF mascarado, códigos, número do quarto), com resultados agrupados, limitados a 8 por grupo e respeitando permissões.
- **Recepção** como tela inicial de trabalho: resumo do dia (chegadas, saídas, quartos ocupados e disponíveis, alertas críticos), mapa dos próximos 7 dias e abas **Chegadas**, **Saídas** e **Em risco**, com atraso e saldo em texto e ações rápidas (**Confirmar check-in**, **Registrar pagamento**, **Concluir check-out**) que abrem direto o diálogo certo.
- **Painel lateral** que mostra o resumo de uma reserva, hospedagem, hóspede ou quarto sem esconder o mapa; fecha com **Fechar painel** ou `Esc` e volta a abrir ao recarregar a página.
- **Visão gerencial** (só administrador): ocupação por dia, recebimentos líquidos por dia e por forma de pagamento, saldos a receber e créditos, contas vencidas e em aberto por categoria, com gráficos e a tabela equivalente de cada um; período padrão hoje.
- **Logo e ícone do Boreal**: fitas de aurora em teal, usadas no ícone do aplicativo (janela, barra de tarefas e instalador), no menu lateral, na tela de entrada e na aba do navegador.
- **Barra de título própria**: a janela deixou de ter a moldura padrão do Windows e ganhou uma barra com o logo, área para arrastar (duplo clique maximiza) e os botões **Minimizar**, **Maximizar/Restaurar** e **Fechar**, sempre visível, inclusive na tela de entrada e sobre os diálogos.

### Changed
- **Novo visual do Boreal**: identidade teal com neutros frios, fonte Onest, ícones Material Symbols (empacotados, funcionam sem internet) e cantos e sombras suaves. Os estados dos quartos, reservas, bloqueios, manutenção e contas continuam combinando cor, ícone e rótulo.
- **Menu lateral reorganizado** em grupos (Recepção, Reservas, Hospedagens, Quartos, Financeiro, Configurações), com o item da tela atual destacado; Relatórios e Alertas aparecem como "Em breve". A tela inicial agora se chama **Recepção**.
- O aviso de dias sem fechamento de caixa na Recepção virou o alerta **Fechamento de caixa pendente**; a busca do topo orienta quando o texto tem menos de 2 caracteres.
- **Ações rápidas no topo** (Nova reserva, Buscar, Check-in, Receber pagamento e Check-out) com atalhos de teclado (`Ctrl+K`, `Alt+N`, `Alt+C`, `Alt+R`, `Alt+S`), que não disparam com uma janela aberta ou com a sessão bloqueada.
- Reservas passam a ter as situações **Hospedado** e **Concluída**; a lista de reservas identifica hospedagens **sem reserva**.
- O **check-in** e a hospedagem sem reserva agora exigem o quarto **limpo**: recusam quarto aguardando limpeza, em limpeza ou antes do intervalo mínimo depois da limpeza.
- A tela de **Configurações** ganhou a cobrança de saída tardia (valor provisório).
- A senha mínima passou a ter 6 caracteres.
- Mensagens de erro do banco de dados agora aparecem em linguagem simples, sem texto técnico.
- Ao bloquear a sessão por inatividade, a tela aberta é preservada e volta como estava após desbloquear.
- O sinal da reserva passa a ser um recebimento de verdade e acompanha a hospedagem no check-in (antes era só um valor na reserva); o botão **Registrar sinal recebido** virou **Registrar pagamento**.
- O cancelamento continua calculando a devolução pela política, mas a devolução em si passa a ser registrada à parte quando o dinheiro volta.
- **Check-in** (`Alt+C`) e **Check-out** (`Alt+S`) do topo agora abrem as abas **Chegadas** e **Saídas** da Recepção, em vez das listas de reservas e hospedagens.
- A tela inicial deixou de ser o Dashboard simples e passou a ser a Recepção completa.

### Fixed
- Ao reiniciar com a chave de criptografia incorreta, o sistema agora explica o problema em vez de mostrar erro técnico.
- Um usuário com senha temporária não consegue mais executar nenhuma operação além de trocar a senha.
- Desativar ou redefinir a senha de alguém encerra imediatamente as sessões abertas dessa pessoa.
- Errar a senha na tela de bloqueio ou ao trocar a própria senha passa a contar para o bloqueio da conta.

- Quando um administrador encerra a sessão de alguém (troca de senha, permissões ou perfil), a pessoa vê "Sessão encerrada" e entra de novo na mesma tela, sem perder o que estava digitando.
- Diálogos abertos (por exemplo, a conferência de hóspedes duplicados) não ficam mais visíveis por cima da tela de bloqueio por inatividade.
- Fechar um formulário em janela (quarto, categoria, usuário, senha, permissões) com dados digitados agora pergunta antes de descartar.
- O botão **Sair** respeita as alterações não salvas.
- Telefones digitados ou colados com +55 não são mais gravados com o número errado.
- Clicar duas vezes em "Criar outro cadastro" não cria dois hóspedes.
- O novo tempo de inatividade vale na interface assim que é salvo, e os dados em cache de quem saiu não aparecem para a próxima pessoa.

### Removed
- Plugin de abertura de links externos, que não era usado.

### Security
- Senhas guardadas com Argon2id; o banco de dados não abre sem a chave correta.
- Política de segurança de conteúdo (CSP) ativada na interface.
- Logotipo enviado é conferido pelo conteúdo do arquivo (PNG/JPEG) e limitado a 512 KB.
- CPF aparece mascarado nos registros de auditoria; e-mail, endereço e telefone não são gravados neles.
- Liberação de recebimento duplicado e todas as operações de contas a pagar exigem permissão verificada no sistema, com auditoria na mesma operação.
