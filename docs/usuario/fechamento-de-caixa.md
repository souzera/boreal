# Fechamento de caixa

## Objetivo

Conferir, no fim do dia, o que o sistema espera ter recebido em cada forma de pagamento e comparar com o que foi contado.

## Quando usar

No fim de cada dia. Não existe "abertura" de caixa. Se um dia ficar sem fechamento, **nada é bloqueado**: o Dashboard mostra **Fechamento pendente: DD/MM/AAAA** e você pode fechar aquele dia depois.

## Passo a passo

1. Abra **Fechamento de caixa** (ou clique no aviso do Dashboard). O dia de hoje vem escolhido; para outro dia use **Dia a fechar** (não é possível escolher dia futuro).
2. Para cada forma de pagamento a tela mostra o **recebido**, o **devolvido** e o **esperado** (recebido menos devolvido). Débito e crédito aparecem separados.
3. Digite o **valor conferido** de **todas** as formas, mesmo as zeradas. Se no dia saiu mais dinheiro (devoluções) do que entrou, o esperado fica negativo e o conferido também pode ser negativo (por exemplo, `-100,00`).
4. Quando há diferença, a tela diz **Falta** ou **Sobra** e pede uma **justificativa** naquela linha. A diferença **não impede** o fechamento, mas precisa ser explicada.
5. Clique em **Concluir fechamento**, confira as diferenças que serão registradas e confirme.

**Resultado:** "Fechamento de DD/MM/AAAA concluído e salvo neste computador." Ficam gravados quem fechou, quando, os valores esperados e conferidos, as diferenças e as justificativas.

## O que o fechamento não faz

- **Não altera nenhum recebimento**: diferenças não são "corrigidas" automaticamente.
- Um dia fechado não pode ser fechado de novo nem alterado.
- Pagamentos feitos **depois** do fechamento no mesmo dia continuam permitidos; a tela do dia fechado mostra **Movimentos depois do fechamento**. Eles não entram no fechamento já gravado.
- **Pagamentos a fornecedores** aparecem numa linha à parte e **não entram** no valor esperado.
- Valores recebidos antes do módulo financeiro, sem forma de pagamento registrada, aparecem como **Sem modalidade** (só aparece quando há valor).

## Erros comuns

- **"Informe o valor conferido, por exemplo R$ 0,00."**: falta preencher alguma forma.
- **"Explique a diferença de …"**: escreva a justificativa da linha.
- **"Esse dia já foi fechado."**: abra o dia para consultar o fechamento gravado.

> ⚠ Provisório, aguardando decisão da pousada: se o fornecedor é pago com o dinheiro do caixa da recepção, esses pagamentos precisarão ser descontados do esperado em dinheiro (Anexo A.5 da especificação).
