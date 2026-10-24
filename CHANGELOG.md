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

### Changed
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
