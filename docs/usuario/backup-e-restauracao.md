# Backup e restauração

## Objetivo

Guardar cópias do banco de dados da pousada, para recuperar tudo se o computador quebrar, for trocado ou se o arquivo for danificado.

## Quem pode usar

Somente o **administrador**, em **Saúde do sistema** (menu Configurações).

## Como o backup funciona

- Cada backup é um arquivo `boreal-AAAA-MM-DD-HHMMSS.bak`, **criptografado** com a mesma chave do banco. Sem a chave de recuperação ele não abre.
- Depois de gravar, o sistema **confere** o arquivo (abre com a chave, verifica a integridade). Só então ele vale; um arquivo que não passa na conferência é descartado.
- O backup é feito **automaticamente** em três situações: ao **concluir o fechamento de caixa**, ao **abrir o sistema** se o último backup válido tem mais de 24 horas, e quando você clica em **Fazer backup agora**.
- Ficam guardados os backups mais novos de cada um dos **7 últimos dias**. Os mais antigos só são apagados **depois** de um backup novo e válido.
- Se o backup falhar, a recepção **continua trabalhando normalmente**. O sistema registra o problema e mostra o alerta **Sem backup válido nas últimas 24 horas** para o administrador.

## Escolher onde guardar

1. Abra **Saúde do sistema** → **Backup**.
2. Em **Pasta dos backups**, clique em **Escolher pasta** e escolha um pen drive, disco externo ou pasta de rede. Sem escolher, o sistema usa a pasta `backups` ao lado do banco.
3. Para voltar ao padrão, **Usar a pasta padrão**.

Recomendação: copie os backups também para **outro dispositivo** de vez em quando (pen drive, disco externo ou nuvem pessoal). Um backup no mesmo computador não protege de um defeito do disco.

## Fazer um backup agora

Clique em **Fazer backup agora**. Aparece "Backup concluído: boreal-….bak". Se falhar, a mensagem diz o motivo (pasta indisponível, falta de espaço…). Em **Últimos backups** você vê cada tentativa, com a situação em texto (**Válido** ou **Falhou**).

## Restaurar um backup

Use quando precisar voltar o sistema a um momento anterior.

1. Em **Saúde do sistema** → **Backup**, procure o arquivo em **Restaurar um backup** (ou use **Escolher outro arquivo** para um arquivo de outro lugar).
2. Clique em **Restaurar o backup de DD/MM/AAAA HH:MM**.
3. Leia a **prévia**: a data do backup, quantas reservas, hospedagens, recebimentos e hóspedes ele contém e quantos o sistema tem agora. Se aparecer o aviso de que há atividade depois do backup, **tudo o que foi feito depois dele será perdido**.
4. Digite **RESTAURAR** e a sua senha e clique em **Restaurar este backup**.
5. O sistema guarda antes uma **cópia de segurança** do estado atual (arquivo `boreal-seguranca-…`), restaura, e **todas as pessoas precisam entrar de novo**.

Um backup que não passa na conferência, que não abre com a chave deste sistema ou que foi feito por uma **versão mais nova** do Boreal **não pode ser restaurado**: a tela explica o motivo e nada é alterado.

Se a restauração não puder ser concluída, o sistema volta ao estado de antes pela cópia de segurança.

## Trocar de computador

1. No computador antigo, faça um backup e copie o arquivo `.bak` para um pen drive. Tenha à mão a **chave de recuperação** (veja o guia [Chave de recuperação](chave-de-recuperacao.md)).
2. Instale o Boreal no computador novo e abra. Na tela **Configuração inicial**, clique em **Já tenho um backup**.
3. **Escolha o arquivo de backup**, digite a **chave de recuperação** e clique em **Restaurar neste computador**.
4. Entre com o seu usuário e senha de sempre.

Só funciona num computador **sem banco de dados**. Com chave errada nada é criado.

## Avisos e erros comuns

- **"Não há espaço suficiente na pasta de backup."**: libere espaço ou escolha outra pasta.
- **"A pasta de backup não está acessível."**: o pen drive não está conectado ou a pasta foi apagada. Conecte e tente de novo.
- **"A chave de recuperação não abre este backup"**: a chave digitada tem um erro ou é de outro sistema.
- Nunca apague arquivos `boreal-…` à mão antes de ter outro backup válido.
