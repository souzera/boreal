# Bloqueios e manutenção

## Bloqueios

**Quando usar:** para tirar um quarto de uso por um período (pintura, reforma pequena) ou até alguém liberá-lo.

**Quem pode:** quem tem a permissão **Liberar quarto**. Sem ela, a tela de **Bloqueios** só mostra a lista.

### Criar um bloqueio

1. Abra **Bloqueios** e clique em **Criar bloqueio**.
2. Escolha o quarto, o **início**, o **fim** (opcional) e o **motivo**. A noite do dia de fim **não** é bloqueada. Sem fim, o bloqueio vale até ser liberado.
3. Clique em **Criar bloqueio**.

**Resultado:** "Quarto 101 bloqueado." O quarto não aceita reservas, alterações para esse período, check-in nem hospedagem sem reserva.

**Se o quarto já tem reserva ou hóspede no período**, o bloqueio é recusado e a mensagem lista as reservas. Nesse caso, use uma **ordem de manutenção**.

### Liberar o quarto

1. Em **Bloqueios**, clique em **Liberar quarto** na linha do bloqueio.
2. Escreva a **justificativa** e confirme.

**Resultado:** "Quarto 101 liberado." O quarto volta a aceitar reservas e hóspedes. Bloqueios de manutenção só podem ser liberados depois que a ordem for concluída.

## Ordens de manutenção

**Quando usar:** quando o quarto precisa de conserto e pode haver reservas no período. Qualquer pessoa autenticada pode abrir e acompanhar ordens.

### Abrir uma ordem

1. Abra **Manutenção** e clique em **Abrir ordem de manutenção**.
2. Informe quarto, descrição, prioridade, início previsto, fim previsto (opcional), responsável e observações.

**Resultado:**
- **Sem reservas afetadas:** o quarto já fica bloqueado.
- **Com reservas afetadas:** o quarto recebe um **bloqueio preventivo** — não aceita novas reservas no período, mas as reservas existentes continuam valendo. A tela da ordem lista as reservas afetadas.

### Resolver as reservas afetadas

O sistema **nunca cancela nem altera reservas sozinho**. Para cada reserva afetada, remaneje o hóspede ou cancele a reserva pelo fluxo normal (a tela da reserva abre pelo link). Depois, na ordem, clique em **Aplicar indisponibilidade**: as reservas que saíram do período são marcadas como resolvidas. Se a solução foi feita fora do sistema, use **Registrar resolução** e escreva como foi resolvido.

Quando não restar nenhuma reserva a resolver, o bloqueio passa a valer de fato.

### Andamento da ordem

| Situação | Próximo passo |
|---|---|
| **Aberta** | **Iniciar manutenção** (só sem reservas a resolver) |
| **Em andamento** | **Marcar como aguardando resolução** |
| **Aguardando resolução** | **Concluir manutenção** |
| **Concluída** | O quarto **continua bloqueado** até ser liberado em **Bloqueios**. |

## Se der erro

- **"Resolva as reservas afetadas antes de iniciar a manutenção"**: use **Aplicar indisponibilidade** ou **Registrar resolução**.
- **"O quarto já tem um bloqueio nesse período"**: libere ou ajuste o bloqueio existente.
- **"Conclua a ordem de manutenção antes de liberar o quarto"**: conclua a ordem primeiro.
- **"Só quem tem a permissão de liberar quartos…"**: peça a um administrador.
