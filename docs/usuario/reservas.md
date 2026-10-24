# Reservas

**Quem pode usar:** a recepção e os administradores. Algumas ações pedem uma permissão extra (veja cada seção).

## Situações de uma reserva

| Situação | Significa |
|---|---|
| **Pendente** | Criada, esperando o sinal. **Já bloqueia o quarto.** |
| **Confirmada** | Sinal recebido (ou confirmada manualmente). |
| **Hospedado** / **Concluída** | Estadia em andamento / encerrada (veja [Hospedagem](hospedagem.md)). |
| **Cancelada** | Cancelada; o registro continua guardado e o quarto fica livre. |
| **No-show** | O hóspede não chegou; marcado por uma pessoa. |

## Criar uma reserva

1. Em **Reservas**, clique em **Nova reserva** (ou clique num dia livre no **Mapa de quartos**).
2. Procure e escolha o **hóspede**, o **quarto**, a **entrada** e a **saída**, e informe o número de pessoas.
3. O sistema mostra na hora se o quarto está livre e o **preço de cada noite**. Se faltar tarifa ou houver conflito, ele diz quais datas e quais reservas atrapalham.
4. Clique em **Salvar reserva**.

**Resultado:** "Reserva RES-000001 criada.", com o valor total, o **sinal esperado** e o **prazo do sinal**. Passar da capacidade do quarto só é possível com a permissão de exceder capacidade.

## A tela da reserva

Mostra hóspede, quarto, período, valores (tabela, desconto e combinado), sinal esperado e recebido, prazo do sinal, o preço de cada noite com a regra usada, as observações e o **histórico** de alterações (quem, quando, valores antes e depois e a justificativa).

## Alterar datas ou quarto

Exige a permissão **Alterar reserva**.

1. Na reserva, clique em **Alterar datas ou quarto**.
2. Mude o quarto, a entrada ou a saída e clique em **Ver prévia**. **Nada é gravado ainda.**
3. Leia a prévia: **preço anterior**, **preço novo** e **diferença**. As noites que continuam **mantêm o preço combinado**; as noites novas usam a **tabela atual**.
4. Escreva a **justificativa** e clique em **Confirmar alteração**.

Se os preços ou a disponibilidade mudarem enquanto você olhava a prévia, o sistema avisa, mostra uma prévia nova e pede para você conferir de novo antes de gravar.

## Desconto

Exige a permissão **Aplicar desconto**. Escolha **valor em reais** ou **percentual**, informe a justificativa e clique em **Aplicar**. O desconto é calculado sobre o valor de tabela e **substitui** o desconto anterior. O sinal esperado é recalculado.

## Registrar sinal recebido

Clique em **Registrar sinal recebido** e informe o valor. Quando o total recebido alcança o sinal esperado, a reserva passa a **Confirmada**. (Neste momento o registro é manual; os recebimentos completos chegam com o módulo financeiro.) O sistema pede uma confirmação mostrando o valor, porque neste momento o registro não pode ser corrigido. Também é possível **Confirmar reserva** manualmente.

## Cancelar

1. Clique em **Cancelar reserva**.
2. O sistema mostra quantos dias faltam, o **sinal já pago**, a **devolução** e a **retenção** calculadas pela política. Nada acontece até você clicar em **Confirmar cancelamento**.
3. Com a permissão **Exceção de cancelamento**, marque **Aplicar exceção à política**, informe o valor retido (de zero até o sinal pago) e a justificativa.

A política padrão (provisória, ajustável em **Configurações**): 7 dias ou mais antes → devolve 100% do sinal; de 3 a 6 dias → 50%; menos que isso → nada.

## "Sinal vencido" e "Para revisar"

- **Sinal vencido** é só um aviso: a reserva continua pendente e o quarto continua bloqueado. **Nada é cancelado sozinho.**
- Na lista de **Reservas**, o quadro **Reservas para revisar** reúne as reservas com sinal vencido e as cuja chegada já passou. Abra cada uma e decida.
- **Marcar no-show** aparece a partir do dia da chegada, pede confirmação e só é aceito depois do horário previsto de chegada mais a tolerância (configurável). O sinal pago fica retido.

## Se algo der errado

| Mensagem | O que fazer |
|---|---|
| "O quarto não está disponível…" | Veja a reserva citada; escolha outro quarto ou outras datas. |
| "Faltam tarifas para: …" | Peça a um administrador para cadastrar as tarifas (veja [Tarifas](tarifas.md)). |
| "O registro foi alterado por outra operação…" | A tela recarrega; confira os dados e repita a ação. |
| "Os preços ou a disponibilidade mudaram desde a prévia…" | Leia a nova prévia e confirme de novo. |
