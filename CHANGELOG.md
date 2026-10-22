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

### Changed
- A senha mínima passou a ter 6 caracteres.
- Mensagens de erro do banco de dados agora aparecem em linguagem simples, sem texto técnico.
- Ao bloquear a sessão por inatividade, a tela aberta é preservada e volta como estava após desbloquear.

### Fixed
- Ao reiniciar com a chave de criptografia incorreta, o sistema agora explica o problema em vez de mostrar erro técnico.
- Um usuário com senha temporária não consegue mais executar nenhuma operação além de trocar a senha.
- Desativar ou redefinir a senha de alguém encerra imediatamente as sessões abertas dessa pessoa.
- Errar a senha na tela de bloqueio ou ao trocar a própria senha passa a contar para o bloqueio da conta.

### Removed
- Plugin de abertura de links externos, que não era usado.

### Security
- Senhas guardadas com Argon2id; o banco de dados não abre sem a chave correta.
- Política de segurança de conteúdo (CSP) ativada na interface.
- Logotipo enviado é conferido pelo conteúdo do arquivo (PNG/JPEG) e limitado a 512 KB.
- CPF aparece mascarado nos registros de auditoria; e-mail, endereço e telefone não são gravados neles.
