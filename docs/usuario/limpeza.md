# Limpeza dos quartos

**Quem pode usar:** a recepção e os administradores. Não é preciso permissão extra para iniciar ou concluir uma limpeza.

## O que acontece quando o hóspede sai

Ao concluir o check-out, o quarto passa para **Aguardando limpeza** e entra na fila de **Limpeza**. O sistema registra a hora real da saída, mesmo que o hóspede tenha saído depois do horário previsto.

O quarto só volta a receber hóspedes quando a limpeza é concluída por uma pessoa **e** o intervalo mínimo de limpeza passou. O tempo sozinho nunca libera um quarto sujo.

## Os estados do quarto

Cada estado aparece com cor, símbolo e nome no **Mapa de quartos** (há uma legenda no alto da tela).

| Estado | Significa |
|---|---|
| **Ocupado** | Há hóspede no quarto. |
| **Aguardando limpeza** | O hóspede saiu e a limpeza ainda não começou. |
| **Em limpeza** | A limpeza foi iniciada. |
| **Limpo — aguardando intervalo** | A limpeza foi concluída; o quarto fica livre à hora indicada. |
| **Disponível** | Limpo e sem bloqueio: pode receber hóspedes. |
| **Em manutenção ou bloqueado** | O quarto está indisponível (veja *Bloqueios e manutenção*). |

## Limpar um quarto

1. Abra **Limpeza**. Os quartos aparecem do mais urgente ao menos urgente.
2. Clique em **Iniciar limpeza** no quarto.
3. Quando terminar, clique em **Concluir limpeza**.

**Resultado:** "Limpeza do quarto 101 concluída. O quarto fica livre às 14:00." O sistema guarda quem concluiu e a hora.

## Prioridade da limpeza

O sistema sugere a prioridade pela próxima chegada no quarto:

- **Urgente:** chegada em até 4 horas (ou já atrasada).
- **Alta:** chegada ainda hoje.
- **Normal:** chegada em outro dia.
- **Baixa:** sem reserva futura.

Esses prazos são provisórios, até a pousada aprová-los.

Para mudar, clique em **Ajustar prioridade**, escolha a nova prioridade e escreva a **justificativa**. Para voltar à sugestão, use **Voltar à sugerida**. O ajuste fica registrado.

## Intervalo mínimo de limpeza

É o tempo entre a limpeza concluída e a próxima hospedagem. O padrão da pousada fica em **Configurações** (60 minutos). Um administrador pode definir outro valor para um quarto em **Quartos → Intervalo de limpeza** (de 0 a 1440 minutos); deixando em branco, vale o padrão.

## Se der erro

- **"O quarto 101 está aguardando limpeza"** ao fazer check-in: conclua a limpeza na tela **Limpeza** antes.
- **"O intervalo mínimo de limpeza só termina às 15:00"**: espere esse horário ou, se for o caso, peça a um administrador para ajustar o intervalo do quarto.
- **"O registro foi alterado por outra operação"**: alguém mexeu na mesma limpeza. A fila é recarregada; confira e tente de novo.
