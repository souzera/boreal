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
- **Desconto** com permissão e justificativa, e registro manual do **sinal recebido** (provisório até o módulo financeiro).
- **Mapa de quartos**: grade quarto × dia, com chegada, saída e situação em texto.
- Configurações do sinal e da política de cancelamento, com os valores marcados como provisórios.
- **Hospedagem**: **check-in** da reserva com ocupantes, saldo inicial e avisos de check-in antecipado e de excesso de capacidade (cada um com sua permissão); **hospedagem sem reserva** com disponibilidade e preço conferidos antes de iniciar; lista de **Hospedagens** com saldo e avisos.
- **Conta do hóspede** em livro de lançamentos que nunca se edita: diárias, desconto e sinal vindos da reserva, **consumos e despesas extras** (avulsos ou do catálogo de **Itens e serviços**), **ajustes** com justificativa e **estorno com relançamento** que preserva o lançamento original e impede cobrança duplicada.
- **Registro de pagamento provisório** na conta (uma forma de pagamento por vez), até o módulo financeiro.
- **Check-out** com demonstrativo (diárias, consumos, descontos, recebimentos, devoluções e saldo); com saldo em aberto ou crédito do hóspede só com permissão, justificativa e autorizador registrados — nada é liquidado nem apagado sozinho.
- **Saída tardia**: tolerância por hospedagem, cobrança prevista provisória (50% da última diária, configurável) que a recepção cobra ou dispensa (com permissão e justificativa), sem criar diária; aviso de **risco para a próxima reserva** quando o atraso encosta na limpeza.
- Novas permissões concedíveis à recepção: **Fazer check-in antecipado**, **Concluir check-out com saldo pendente** e **Dispensar cobrança de saída tardia**.
- **Limpeza dos quartos**: ao concluir o check-out, o quarto fica **aguardando limpeza** e entra numa fila ordenada por urgência (pela próxima chegada, com ajuste manual justificado). Iniciar e concluir a limpeza registram quem e quando. O quarto só volta a receber hóspedes depois da limpeza concluída e do intervalo mínimo, que agora pode ser definido por quarto.
- **Estados do quarto no mapa**: Ocupado, Aguardando limpeza, Em limpeza, Limpo — aguardando intervalo, Disponível e Em manutenção ou bloqueado, sempre com cor, símbolo e nome, e legenda. A data do mapa fica no endereço, então abrir a fila de limpeza e voltar não perde a agenda. O Dashboard mostra quantos quartos aguardam limpeza.
- **Bloqueios de quarto** por período ou até a liberação manual, que impedem reservas, alterações, check-in e hospedagem sem reserva no período; **liberar o quarto** exige a permissão e uma justificativa.
- **Ordens de manutenção** (aberta, em andamento, aguardando resolução, concluída) com **reservas afetadas** listadas e bloqueio preventivo até serem resolvidas. Concluir a ordem não libera o quarto: a liberação continua dependendo de quem tem autorização.

### Changed
- Reservas passam a ter as situações **Hospedado** e **Concluída**; a lista de reservas identifica hospedagens **sem reserva**.
- O **check-in** e a hospedagem sem reserva agora exigem o quarto **limpo**: recusam quarto aguardando limpeza, em limpeza ou antes do intervalo mínimo depois da limpeza.
- A tela de **Configurações** ganhou a cobrança de saída tardia (valor provisório).
- A senha mínima passou a ter 6 caracteres.
- Mensagens de erro do banco de dados agora aparecem em linguagem simples, sem texto técnico.
- Ao bloquear a sessão por inatividade, a tela aberta é preservada e volta como estava após desbloquear.

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
