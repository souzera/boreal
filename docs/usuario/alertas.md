# Alertas

## Objetivo

Reunir num só lugar o que precisa da sua atenção: chegadas e saídas de hoje, sinais vencidos, saídas atrasadas, conflitos de manutenção, limpezas urgentes, contas vencidas, saldos pendentes e dias sem fechamento de caixa.

## Onde ver

- **Menu → Alertas**: o painel completo. Ao lado do nome aparece um selo com o total de alertas ativos (com o ícone de erro quando há algum **crítico**).
- **Recepção**: os alertas mais urgentes, com o botão de abrir o registro.

Os alertas são reavaliados ao abrir a tela e a cada minuto. Um alerta pode demorar até um minuto para aparecer depois que a situação acontece.

## Como ler um alerta

Cada alerta mostra: a **prioridade** (ícone e texto: **Crítico**, **Alto** ou **Normal**), o **tipo**, **o que aconteceu**, **desde quando** e o botão **Abrir registro**, que leva direto à reserva, hospedagem, ordem de manutenção, fila de limpeza, conta ou fechamento envolvido.

| Tipo | Prioridade | Quando aparece |
|---|---|---|
| Chegada hoje | Normal | Reserva com chegada hoje e sem check-in. |
| Saída hoje | Normal | Hospedagem que sai hoje. |
| Chegada atrasada | Alto | Passou o horário previsto mais a tolerância de no-show. Faça o check-in ou marque no-show. |
| Sinal vencido | Alto | O prazo do sinal passou. A reserva continua ativa; nada é cancelado sozinho. |
| Saída atrasada | Alto | Passou o horário de saída mais a tolerância. |
| Risco para a próxima reserva | **Crítico** | A saída atrasada pode atrasar a chegada da reserva seguinte no mesmo quarto. |
| Reserva afetada por manutenção | **Crítico** | Uma reserva cai no período de uma ordem de manutenção e o conflito ainda está aberto. |
| Limpeza urgente | Alto | Limpeza aberta com chegada em até 4 horas (ou prioridade ajustada para urgente). |
| Conta vencida | Alto | Conta a pagar com saldo vencida. **Só aparece para quem pode gerenciar contas a pagar.** |
| Saldo pendente na saída | Alto | Hospedagem que sai hoje (ou já deveria ter saído) com saldo diferente de zero. |
| Fechamento de caixa pendente | Normal | Dia anterior com recebimentos ou devoluções e sem fechamento. Nada fica bloqueado. |

## Como um alerta some

- **Sozinho**, quando o problema deixa de existir (o hóspede fez o check-in, o sinal foi pago, a limpeza foi concluída, o dia foi fechado…). Ele vai para o **Histórico**, com a hora em que foi resolvido.
- **Dispensando-o**, se não for crítico: clique em **Dispensar alerta**, escreva a **justificativa** e confirme. Ele sai da lista de ativos e fica no Histórico com o seu nome e o motivo. Enquanto o problema continuar, ele não volta; se o problema acabar e voltar depois, aparece um alerta novo.
- **Alertas críticos não podem ser dispensados**: só desaparecem quando o problema é resolvido. A tela avisa isso no próprio alerta.

Nada é apagado: o **Histórico** guarda tudo.

## Filtros

Use **Tipo** e **Prioridade** para filtrar, e as abas **Ativos** e **Histórico**.

## Erros comuns

- **"O registro foi alterado por outra operação"**: o alerta mudou enquanto você olhava; feche o diálogo e tente de novo.
- **"Explique por que o alerta está sendo dispensado."**: a justificativa é obrigatória.
- Não vejo "Conta vencida": esse tipo só aparece para quem tem a permissão **Gerenciar contas a pagar**.

> ⚠ Provisório: a lista de tipos, as prioridades e a regra de dispensa estão no Anexo A.6 da especificação, aguardando aprovação. Alertas de backup, espaço em disco e falha de atualização existem como tipos, mas só passam a aparecer quando esses recursos forem entregues.
