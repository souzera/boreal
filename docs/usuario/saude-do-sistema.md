# Saúde do sistema

## Objetivo

Reunir, para o administrador, tudo o que protege os dados: backups, restauração, chave de recuperação, espaço em disco e versões.

## Onde

Menu **Configurações → Saúde do sistema** (só o administrador vê).

## O que a tela mostra

| Área | O que mostra |
|---|---|
| **Backup** | A pasta dos backups, **Fazer backup agora**, os últimos backups (**Válido** ou **Falhou**, com o motivo) e os arquivos para **restaurar**. |
| **Chave de recuperação** | **Mostrar chave de recuperação** (com a senha), **Imprimir**. |
| **Espaço em disco** | Quanto espaço livre há, o recomendado e a situação em texto: **Espaço suficiente** ou **Pouco espaço em disco**. |
| **Versões e verificação** | Versão do aplicativo e do banco, o resultado da verificação feita ao abrir o sistema e a pasta dos **logs técnicos**. |

Em **Configurações → Sobre o sistema** aparece também o **último backup válido**.

## Alertas do administrador

Estes três alertas são **críticos** (não podem ser dispensados) e **só o administrador os vê**. Somem sozinhos quando o problema acaba.

| Alerta | Quando aparece | O que fazer |
|---|---|---|
| **Sem backup válido nas últimas 24 horas** | Nenhum backup válido no último dia (o texto diz a data do último e, se a última tentativa falhou, o motivo). | Abra **Saúde do sistema** e clique em **Fazer backup agora**. Resolva o motivo (pasta, espaço). |
| **Pouco espaço em disco** | O espaço livre ficou abaixo do maior entre 1 GB e três vezes o tamanho do banco. | Libere espaço ou escolha outra pasta para os backups. |
| **Falha na atualização** | Uma atualização do banco não foi concluída. Seus dados foram preservados. | Procure o suporte; se necessário, reinstale a versão anterior. O alerta some quando uma atualização é concluída. |

Se o disco encher no meio de uma operação, o sistema mostra **"Sem espaço em disco. Libere espaço e tente de novo. Nada foi salvo."** e nenhuma informação fica pela metade.

## Ao abrir o sistema

- O sistema **confere a integridade** do banco toda vez que abre. Se encontrar problema, ele **não abre normalmente**: aparece a tela **Recuperar o banco de dados**. Escolha um backup, digite a **chave de recuperação** e **RESTAURAR**. O banco com problema é guardado ao lado (`boreal-com-problema-…`), sem ser apagado.
- Ao **atualizar o sistema**, se o banco precisa mudar, o sistema faz **antes** um backup (`boreal-seguranca-…`). Se esse backup falhar, **nada é alterado** e o sistema avisa: libere espaço e abra de novo. Se a atualização falhar no meio, o banco anterior fica intacto.

## Logs técnicos

Ficam na pasta indicada em **Versões e verificação**, um arquivo por dia, guardados por 14 dias. Não têm senhas, chaves nem dados de hóspedes. Se o suporte pedir, envie os arquivos dessa pasta.
