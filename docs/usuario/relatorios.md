# Relatórios

## Objetivo

Consultar na tela, filtrar e levar para fora do sistema (CSV para planilha e PDF para imprimir ou arquivar) os números da pousada.

## Quem pode usar

- **Todas as pessoas** veem os relatórios do dia a dia: **Ocupação**, **Reservas**, **Hospedagens** e **Cancelamentos e no-shows**.
- **Só o administrador** vê os relatórios financeiros: **Recebimentos e devoluções**, **Saldos pendentes**, **Despesas extras dos hóspedes**, **Despesas da pousada**, **Contas a pagar** e **Fechamentos de caixa**.

## Passo a passo

1. Abra **Relatórios** no menu.
2. Escolha o **Relatório**. A tela mostra uma frase explicando o que ele traz e só os filtros que ele aceita.
3. Ajuste os filtros (todos são opcionais e podem ser combinados):
   - **De** e **Até**: o período (no máximo 366 dias; começa nos últimos 30 dias).
   - **Situação**, **Quarto**, **Forma de pagamento**, **Categoria** e **Texto** (nome do hóspede, código ou fornecedor).
4. Clique em **Gerar relatório**. A tabela mostra as linhas e, no rodapé, os totais.
5. Para levar o resultado: **Exportar CSV** ou **Exportar PDF**. O sistema pergunta onde salvar. Depois aparece "Arquivo salvo em …" e o botão **Abrir arquivo**, de onde você imprime.

O endereço da tela guarda o relatório e os filtros: se você recarregar a página, a mesma consulta volta.

## O que cada relatório mostra

| Relatório | Data de referência | Observações |
|---|---|---|
| Ocupação | cada dia do período | Quartos, ocupados, livres e a taxa; no rodapé, a média. |
| Reservas | data de chegada | Valor combinado somado no rodapé. |
| Hospedagens | dia do check-in | Saldo de cada conta. |
| Cancelamentos e no-shows | dia do cancelamento ou da marcação | Valor retido e devolução prevista. |
| Recebimentos e devoluções | dia em que foi lançado | Uma linha por forma de pagamento (débito e crédito separados). Devolução aparece com valor negativo; o rodapé é o líquido. |
| Saldos pendentes | situação de agora | Contas em andamento com saldo a receber ou crédito do hóspede. Não usa período. |
| Despesas extras dos hóspedes | dia do lançamento | Consumos e extras, com os estornos. |
| Despesas da pousada | data de **competência** | Mostra também quanto foi **pago no período** (data efetiva do pagamento). |
| Contas a pagar | data de vencimento | O que foi pago, o que falta, a situação e o último pagamento. |
| Fechamentos de caixa | dia fechado | Esperado, contado e diferença por forma, com a justificativa. |

## Sobre o CSV

- Abre direto no Excel em português (separador **ponto e vírgula**). Se precisar, troque o **Separador do CSV** para vírgula antes de exportar.
- Valores saem como número (`150,00`) e datas como `DD/MM/AAAA`.
- O arquivo tem **as mesmas linhas da tela**.

## Avisos

- Cada relatório mostra até **5.000 linhas**. Se houver mais, a tela, o CSV e o PDF avisam: refine os filtros.
- Relatório sem linhas mostra "Nenhum registro para estes filtros."

## Erros comuns

- **"A data final não pode ser antes da inicial."** ou **"O período pode ter no máximo 366 dias."**: corrija as datas; nada é consultado antes.
- **"Não foi possível gravar neste local. Escolha outra pasta e tente de novo."**: a pasta escolhida não aceita arquivos. Clique em **Tentar de novo** (ou exporte para outra pasta). Nada é gravado pela metade.
- Não vejo os relatórios financeiros: eles são só do administrador.
