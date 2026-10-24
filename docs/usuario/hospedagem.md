# Hospedagem: check-in, hospedagem sem reserva e check-out

**Quem pode usar:** a recepção e os administradores. Algumas ações pedem uma permissão extra (veja cada seção).

## Situações de uma hospedagem

| Situação | Significa |
|---|---|
| **Hospedado** | O hóspede já fez o check-in e a conta está aberta. |
| **Encerrada** | O check-out foi concluído. A conta continua disponível só para consulta. |

## Fazer o check-in de uma reserva

**Quando usar:** quando o hóspede chega e tem reserva pendente ou confirmada.

1. Abra a reserva e clique em **Confirmar check-in**.
2. Confira os ocupantes. O titular já aparece; use **Adicionar ocupante** para incluir quem vem junto (nome completo e, se quiser, documento).
3. Leia o **Saldo inicial**: diárias, menos o desconto e o sinal já recebido.
4. Clique em **Confirmar check-in**.

**Resultado:** "Check-in confirmado e salvo neste computador." A tela da hospedagem abre com as diárias já lançadas na conta, o desconto da reserva e o sinal como recebimento.

**Avisos e permissões**
- **Antes do horário previsto de entrada** o sistema avisa, mesmo que o quarto esteja livre. Só quem tem a permissão **Fazer check-in antecipado** consegue confirmar. O quarto precisa estar livre desde agora.
- **Mais pessoas que a capacidade do quarto** exige a permissão **Exceder a capacidade do quarto** e a marcação "Autorizar exceder a capacidade".
- Reservas canceladas, com no-show, já hospedadas ou concluídas não aceitam check-in.
- O sistema ainda **não confere se o quarto está limpo** (isso chega com o módulo de limpeza); ele confere apenas o intervalo mínimo entre uma saída e a próxima entrada.

**Se der erro:** a mensagem diz o motivo (por exemplo, "O quarto já está reservado nesse período: RES-000001"). O que você preencheu continua na tela.

## Hospedagem sem reserva

**Quando usar:** para quem chega sem reserva.

1. Em **Hospedagens**, clique em **Hospedagem sem reserva**.
2. Escolha o hóspede (ou cadastre um novo), o quarto e a **saída**. A entrada é hoje, agora.
3. Confira a disponibilidade e o **preço de cada noite**. Se faltar tarifa ou o quarto estiver ocupado, o sistema mostra o motivo e não deixa continuar.
4. Informe os ocupantes e clique em **Confirmar check-in**.

**Resultado:** "Hospedagem iniciada e salva neste computador." O código começa com **HOS-** e a hospedagem também aparece na lista de reservas com o rótulo "Sem reserva".

## A tela da hospedagem

Mostra o hóspede, o quarto, os horários previstos, quando o check-in foi feito, os ocupantes, o **saldo** (em palavras: "Saldo a receber", "Quitado" ou "Crédito do hóspede"), a conta lançamento por lançamento e os recebimentos. A lista **Hospedagens** mostra quem está hospedado agora, com o saldo e os avisos de saída atrasada.

Para lançar consumos, corrigir lançamentos e registrar pagamentos, veja [Conta do hóspede](conta-do-hospede.md).

## Concluir o check-out

1. Na hospedagem, clique em **Concluir check-out**.
2. Leia o **demonstrativo**: diárias, consumos e despesas, descontos e ajustes, recebimentos, devoluções e o saldo final.
3. Se a conta estiver quitada, clique em **Concluir check-out**.

**Resultado:** "Check-out concluído e salvo neste computador." A hospedagem fica **Encerrada** e a reserva passa a **Concluída**.

### Se houver saldo em aberto

O sistema **não liquida nem apaga nada sozinho**. Você pode registrar o pagamento antes. Se o hóspede sair com saldo, quem tem a permissão **Concluir check-out com saldo pendente** marca "Concluir com saldo em aberto" e escreve a justificativa. Ficam registrados o saldo, a justificativa, quem autorizou e quando. Se o hóspede pagou a mais, a tela mostra "Valor a devolver ao hóspede"; concluir também exige essa permissão.

### Saída depois do horário (saída tardia)

- Há uma **tolerância** (padrão: 30 minutos, em **Configurações**). Dentro dela, nada é cobrado.
- Passou da tolerância: o demonstrativo mostra "Saída X min depois do previsto; cobrança prevista R$ …". **Nenhuma diária é criada automaticamente.** Você escolhe **Cobrar saída tardia** (entra na conta) ou **Dispensar cobrança**.
- Dispensar exige a permissão **Dispensar cobrança de saída tardia** e uma justificativa. O atraso e o valor calculado continuam registrados.
- A tolerância pode ser ajustada para uma hospedagem específica por quem tem a permissão **Alterar datas ou quarto de reservas**: na tela da hospedagem, em "Tolerância de saída tardia desta hospedagem" (em branco = padrão).
- **Valor provisório:** a cobrança prevista é uma porcentagem da última diária (padrão 50%), ajustável em **Configurações**, até a pousada aprovar a regra.

### Aviso para a próxima reserva

Se a saída atrasou e o tempo de limpeza não cabe antes da próxima chegada no mesmo quarto, a lista de hospedagens e o check-out mostram "Risco para a próxima reserva". É só um aviso: nenhuma reserva é alterada ou cancelada.
