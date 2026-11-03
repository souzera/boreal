# Chave de recuperação

## Objetivo

Garantir que a pousada nunca perca os dados. O banco e os backups são criptografados; a **chave de recuperação** é o que os abre em qualquer computador.

## Por que ela importa

O sistema guarda a chave no cofre de credenciais do Windows deste computador. Se o computador quebrar, for trocado, ou se a credencial do Windows se perder, **só a chave de recuperação** reabre os dados.

> **Sem a chave de recuperação e sem este computador, os dados não podem ser recuperados por ninguém.**

## Quem pode ver

Somente o **administrador**, e **digitando a própria senha de novo**. Cada vez que a chave é mostrada, o sistema registra o fato (sem registrar a chave).

## Passo a passo

1. Abra **Saúde do sistema** → **Chave de recuperação**.
2. Clique em **Mostrar chave de recuperação**, digite a sua senha e clique em **Mostrar**.
3. A chave aparece em **9 grupos** (os 8 primeiros são a chave; o último é só de conferência).
4. Clique em **Imprimir** e guarde a folha **fora do computador** (por exemplo, num cofre ou com o proprietário). Depois clique em **Ocultar chave**.

Faça isso **assim que o sistema for instalado**, antes de usar os dados de verdade, e guarde a folha num lugar seguro. Quem tiver a chave e um backup consegue abrir os dados.

## Quando usar

- **Troca de computador**: veja [Backup e restauração](backup-e-restauracao.md).
- **Aparece "A chave de criptografia do banco não está disponível neste computador"** ao abrir o sistema (por exemplo, depois de trocar o usuário do Windows): a tela oferece **Desbloquear com a chave de recuperação**. Digite a chave e clique em **Desbloquear**; o banco que já está no computador reabre.
- **Banco com problema**: a tela oferece restaurar um backup com a chave (veja [Saúde do sistema](saude-do-sistema.md)).

## Erros comuns

- **"A chave de recuperação está incompleta."**: faltou algum grupo.
- **"Há um erro de digitação na chave de recuperação."**: confira cada grupo com a folha impressa (o último grupo é a conferência).
- **"A chave de recuperação não abre este backup"** ou **"…não abre o banco de dados deste computador"**: a chave é de outro sistema.
