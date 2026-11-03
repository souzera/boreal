# Pagamentos e devoluções

## Objetivo

Registrar o dinheiro que entra (sinal da reserva e pagamentos da conta do hóspede) e o dinheiro que volta para o hóspede, sempre com a forma de pagamento de cada parte.

## Quando usar

- O hóspede paga o **sinal** de uma reserva, parte da conta durante a hospedagem ou o saldo na saída.
- Uma cobrança foi feita em duplicidade ou a reserva foi cancelada e o sinal precisa ser devolvido.

## Registrar um pagamento

1. Abra a **reserva** (para o sinal) ou a **hospedagem** (para a conta) e clique em **Registrar pagamento**.
2. Informe o **valor recebido** e escolha a **forma de pagamento**: dinheiro, Pix, cartão de débito, cartão de crédito ou transferência bancária. Débito e crédito são sempre formas diferentes.
3. Se o hóspede pagou com mais de uma forma, clique em **Dividir em outra forma**, escolha a forma e o valor de cada parte. A tela mostra a soma e diz se **Confere**, se **Falta** ou se **Sobra** dinheiro. O botão só funciona quando a soma é exatamente o valor recebido.
4. Confira o resumo (por exemplo, "Registrar pagamento de R$ 100,00 dividido em Pix R$ 60,00 + Dinheiro R$ 40,00?") e clique em **Confirmar pagamento**.

**Resultado:** "Pagamento PAG-000001 de R$ 100,00 registrado e salvo neste computador." O saldo da conta (ou o sinal recebido) é atualizado na hora. Ao receber o sinal esperado, a reserva passa a **Confirmada**.

**Atenção:** depois de salvo, o pagamento não pode ser editado. Se foi digitado errado, registre uma **devolução** e lance o pagamento certo.

## Recebimento repetido (possível duplicidade)

Se já existe, na mesma reserva ou hospedagem, um recebimento do **mesmo valor, com as mesmas formas de pagamento, no mesmo dia**, o sistema **não grava** e mostra o recebimento parecido.

- Se foi engano, clique em **Voltar e corrigir**.
- Se de fato foram dois pagamentos iguais (por exemplo, dois hóspedes), quem tem a permissão **Liberar recebimento duplicado** informa a justificativa e clica em **Liberar recebimento duplicado**. Ficam registrados quem liberou, quando e por quê.
- Sem a permissão, chame um administrador.

## Registrar uma devolução

1. Na lista **Recebimentos** da reserva ou da hospedagem, clique em **Registrar devolução** na linha do recebimento original.
2. Informe o **valor a devolver** (no máximo o que ainda resta daquele recebimento), a forma de pagamento da devolução (pode ser dividida) e o **motivo**.
3. Confira o resumo e clique em **Confirmar devolução**.

**Resultado:** "Devolução DEV-000001 de R$ 30,00 registrada e salva neste computador." A devolução fica ligada ao recebimento original e o saldo da conta é ajustado.

## Reserva cancelada com sinal para devolver

Ao cancelar, o sistema calcula quanto do sinal volta ao hóspede (pela política de cancelamento) e a reserva mostra **Devolução a fazer: R$ …**. **O sistema não devolve nada sozinho**: quando o dinheiro for devolvido, use **Registrar devolução** no recebimento do sinal. O aviso diminui a cada devolução registrada. Devolver **mais** do que a política calculou só é possível para quem tem a permissão **Autorizar exceções de cancelamento**.

Devolver parte do sinal de uma reserva que ainda está aberta reduz o "sinal recebido" da reserva (a reserva continua como está; use **Confirmar reserva** ou receba o restante conforme o caso).

## Erros comuns

- **"A soma das formas de pagamento precisa ser R$ …"**: ajuste os valores até aparecer "Confere".
- **"Só é possível devolver até R$ … deste recebimento"**: o recebimento já foi devolvido em parte; confira os valores na lista.
- **"A hospedagem já foi encerrada. Não é possível mudar a conta."**: pagamentos só entram enquanto a hospedagem está em andamento; uma devolução ainda pode ser registrada.
- **"O registro foi alterado por outra operação"**: abra a tela de novo e repita.

> ⚠ Valores provisórios que aguardam aprovação da pousada: o bloqueio de duplicidade considera mesmo valor, formas e dia; a devolução do sinal só é registrada pela recepção (sem permissão extra). Veja o Anexo A.5 da especificação.
