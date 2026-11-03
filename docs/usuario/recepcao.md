# Recepção

## Objetivo

A **Recepção** é a primeira tela do sistema. Ela mostra o que precisa de ação **hoje**: quem chega, quem sai, quais quartos estão em risco e como estão os próximos 7 dias.

## Quando usar

Em todo início de turno e sempre que quiser saber "o que falta fazer agora?".

## O que aparece na tela

1. **Resumo do dia**, em cinco cartões: **Chegadas**, **Saídas**, **Quartos ocupados** (por exemplo, "3 de 8"), **Quartos disponíveis** e **Alertas críticos**. Clicar em **Chegadas** ou **Saídas** abre a lista correspondente; **Alertas críticos** leva à tela de Alertas.
2. **Próximos 7 dias**: o mapa de quartos a partir de hoje. Clique numa reserva para ver o resumo ao lado, sem sair da tela. Clique no código de um quarto para ver a situação dele. Clique em **Livre** para começar uma reserva naquele quarto e dia.
3. **Trabalho de hoje**, em três abas:
   - **Chegadas**: reservas que chegam hoje ou que já deviam ter chegado. Aparece **Atrasada** quando passou do horário previsto mais a tolerância, e **Sinal vencido** quando o prazo do sinal passou.
   - **Saídas**: hospedagens que saem hoje ou que já deviam ter saído, com a conta em palavras ("Saldo a receber R$ 80,00", "Quitado") e **Atrasada há N min** quando for o caso.
   - **Em risco**: conflitos que podem atrapalhar a próxima reserva (saída atrasada, manutenção sobre reserva).
4. **Alertas urgentes** e o atalho **Quartos aguardando limpeza**.

## Passo a passo

### Fazer o check-in de quem chegou
1. Na aba **Chegadas**, localize a pessoa.
2. Clique em **Confirmar check-in**. A reserva abre já com a janela de check-in na frente.
3. Confira os ocupantes e confirme.

### Registrar pagamento ou concluir a saída
1. Na aba **Saídas**, escolha a hospedagem.
2. **Registrar pagamento** só aparece quando há saldo a receber. **Concluir check-out** abre a janela de saída.
3. Se a ação não estiver mais disponível (por exemplo, a hospedagem já foi encerrada por outra pessoa), a tela avisa em vez de abrir.

### Ver detalhes sem perder a agenda
Clique em **Ver detalhes** (ou numa reserva do mapa). Um painel abre ao lado e **o mapa continua na tela**. Para fechar, use **Fechar painel** ou a tecla `Esc`. O painel tem o botão para abrir o registro completo. Se recarregar a página, o mesmo painel volta a abrir.

## Atalhos

| Atalho | O que faz |
|---|---|
| `Alt+C` | Abre a Recepção na aba **Chegadas** |
| `Alt+S` | Abre a Recepção na aba **Saídas** |
| `Alt+R` | Abre a lista de hospedagens para receber pagamento |
| `Alt+N` | Nova reserva |
| `Ctrl+K` | Buscar |

## Resultado esperado

Você enxerga, num só lugar, tudo o que está pendente hoje e chega ao registro certo em um clique.

## Avisos

- As listas mostram até 50 itens; se houver mais, a tela diz "Mostrando 50 de N" e leva à lista completa.
- Os números se atualizam sozinhos a cada minuto.
- Se não for possível carregar, aparece "Não foi possível carregar a Recepção. Tente de novo em instantes." e o que já estava na tela continua visível.

## Erros comuns

- **Não vejo uma reserva em Chegadas**: ela pode já ter sido cancelada, marcada como no-show ou ter feito o check-in. Procure em **Reservas**.
- **Não aparece "Registrar pagamento"**: a conta está quitada ou tem crédito do hóspede.
