# Recepção (Marco 6.3)

Tela inicial de trabalho. Plano: `docs/superpowers/plans/2026-11-02-marco-6-3-recepcao-e-gerencial.md`; decisões R1 a R8 e Anexo A.7 da spec. **Sem migração**: tudo é leitura composta.

## Backend (`ReceptionService::overview`)

Uma chamada (`reception_overview`) devolve `ReceptionOverview { date, summary, arrivals, departures, at_risk }`.

- **Chegadas**: `reservation_repo::pending_arrivals` (`pending`/`confirmed`, chegada **hoje ou antes**, por data e horário). `overdue` = horário previsto + tolerância de no-show já passou (mesma regra do detector de alertas); `deposit_overdue` vem da reserva.
- **Saídas**: hospedagens `in_house` com saída hoje ou antes; `minutes_late` de `StayService::assess_departure`; `balance_cents` de `stay_repo::balance`.
- **Em risco**: alertas ativos `departure_risk` e `maintenance_conflict` via `AlertService::list` (respeita permissões por tipo).
- **Resumo**: quartos ocupados/disponíveis pelo estado calculado de `RoomStatusService` (só quartos ativos); `critical_alerts` = `AlertService::summary().critical`.
- Cada lista traz até 50 itens e o **total completo** (`Group<T>`, o mesmo tipo da busca).
- Somente leitura: não grava nem audita. Exige sessão válida; sem permissão extra.

- O foco vai para o painel ao abrir (e ao trocar de registro): é isso que faz o `Esc` funcionar sem clicar nele.
- O painel do hóspede abre pelo botão **Ver hóspede** do painel da reserva (`/?painel=hospede-N`); o quarto, pelo código dele no mapa (só quartos ativos).
- Uma reserva pendente/confirmada que nunca chegou continua em **Chegadas** (com **Atrasada**) até ser marcada como no-show ou cancelada, mesmo depois da data de saída; a ação é abrir a reserva.

Adiado (menores da revisão): `?abrir=` decide com dados que podem estar em cache antes do refetch (o backend recusa de qualquer forma); a reavaliação de alertas roda em `useReceptionOverview` e em `useAlertSummary` (duas rodadas por minuto na Recepção).

Limitação conhecida: a chamada percorre as hospedagens em andamento com consultas por item (N+1), desprezível para uma pousada; não medido com meses de dados.

## Frontend

- `ReceptionPage` (rota `/`) usa `useReceptionOverview` (reavalia alertas, lê a visão, 60 s, sem repetir tentativa), `roomMap` de 7 dias e `RoomMapGrid`.
- `RoomMapGrid` é a grade extraída do Mapa de quartos; a página Mapa e a Recepção usam o mesmo componente.
- **Painel lateral** (`SidePanel`): `aside` nomeado, **não modal** (sem `role="dialog"`, sem prender foco). `Esc` com o foco dentro fecha. A seleção fica na URL: `?painel=reserva-5|hospedagem-3|hospede-7|quarto-2` (valor inválido é ignorado; id inexistente mostra "Não foi possível carregar este registro."). A aba fica em `?aba=chegadas|saidas|risco`.
- **Ações rápidas** só navegam: `/reservas/:id?abrir=checkin` e `/hospedagens/:id?abrir=pagamento|checkout`. `useOpenFromQuery` abre o diálogo **uma vez**, remove o parâmetro (`replace`) e, quando a ação não cabe mais (reserva fechada, hospedagem encerrada), mostra um aviso em vez de abrir. As regras e permissões continuam nos diálogos e no backend.
- Atalhos: `Alt+C` → `/?aba=chegadas`, `Alt+S` → `/?aba=saidas`; `GlobalActions` compara `pathname + search` para não empilhar histórico.

## Desvios do plano

- O cartão **Alertas críticos** leva a `/alertas` sem filtro, porque a tela de Alertas não lê filtros da URL.

## Testes

Backend: `services/reception_service.rs` (chegadas atrasadas, canceladas/hospedadas fora, saídas de hoje e passadas, estado dos quartos, riscos, limite de 50, sessão). Frontend: `RoomMapGrid`, `SidePanel`, `useOpenFromQuery`, painéis, `ReceptionPage`, `?abrir=` em `ReservationPage`/`StayPage`, atalhos.
