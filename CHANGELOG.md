# Changelog

Todas as mudanças relevantes do Boreal PMS são registradas aqui.
Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [Unreleased]

### Added
- Fundação do sistema: aplicativo desktop com banco de dados local criptografado, cuja chave fica no cofre de credenciais do Windows.
- Login com usuário e senha, troca de senha obrigatória no primeiro acesso, bloqueio temporário da conta após tentativas inválidas e bloqueio da sessão por inatividade.
- Perfis de administrador e recepção, com permissões verificadas no sistema e não apenas na tela.
- Registro de auditoria que não pode ser alterado nem apagado.

### Security
- Senhas guardadas com Argon2id; o banco de dados não abre sem a chave correta.
- Política de segurança de conteúdo (CSP) ativada na interface.
