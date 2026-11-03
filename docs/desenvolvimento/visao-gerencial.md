# Visão gerencial (Marco 6.3)

Dashboard do administrador. Plano: `docs/superpowers/plans/2026-11-02-marco-6-3-recepcao-e-gerencial.md`; decisões R5 a R7. **Sem migração**; somente leitura.

## Backend (`ManagementService::dashboard(token, from, to)`)

- **Permissão**: `ViewFinancialReports` (administrador, não concedível). A recepção recebe `Forbidden`.
- **Período**: `from ≤ to`, datas válidas e no máximo 366 dias (inclusive); senão `Validation`.
- **Ocupação por dia**: noites de `reservation_nights` de reservas `pending|confirmed|checked_in|completed` ÷ quartos ativos **hoje**, em pontos-base (arredondado); `0` quando não há quartos ativos. Todo dia do período aparece.
- **Chegadas/saídas** do período: reservas ativas pela data de chegada/saída; **hospedados agora**: hospedagens `in_house`.
- **Recebimentos**: por forma (`closing_repo::received/refunded` em `utc_bounds`) e por **dia local** (`date(received_at, '±N seconds')`), líquido = recebido − devolvido; dia sem movimento aparece com zero e pode ser negativo. As cinco formas sempre aparecem; `unspecified` só quando existe.
- **Saldos**: soma de `stay_repo::balance` das hospedagens em andamento, separando a receber e crédito (situação atual).
- **Contas a pagar**: vencidas (`due_date < hoje`, saldo > 0) e em aberto por categoria (só com saldo).
- Valores em centavos inteiros; nada é gravado nem auditado.

## Frontend

`ManagementPage` (rota `/gerencial`, item de menu `admin: true`): período Hoje/7 dias/Mês/livre; cartões; três gráficos Recharts (`--chart-1..5`), cada um dentro de `ChartCard` com `role="img"` + descrição e a **tabela equivalente** em `<details>` "Ver os números"; tabela por forma de pagamento. Período invertido não consulta. Quem não é administrador vê o aviso e nada é consultado.

## Limitações

- O denominador da ocupação usa os quartos ativos de hoje, não o histórico de ativação.
- Saldos e contas são da situação atual, não do período.
- Relatórios com filtros e exportação ficam para o 6.4.

## Testes

Backend: `services/management_service.rs` (permissão, períodos inválidos, ocupação, zero quartos, movimento, dia local 23:30, dia negativo, dias vazios, saldos, contas). Frontend: `ManagementPage`, `NavItems`.
