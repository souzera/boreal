import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ReceptionPage from "./ReceptionPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    receptionOverview: vi.fn(),
    refreshAlerts: vi.fn(),
    alertSummary: vi.fn(),
    roomMap: vi.fn(),
    cleaningQueue: vi.fn(),
    getReservation: vi.fn(),
    getStay: vi.fn(),
    getGuest: vi.fn(),
    listRoomStatus: vi.fn(),
  };
});

const FIXED_TODAY = new Date(2099, 9, 24, 12, 0, 0); // 24/10/2099

function overview(patch: Partial<api.ReceptionOverview> = {}): api.ReceptionOverview {
  return {
    date: "2099-10-24",
    summary: { arrivals: 2, departures: 1, occupied_rooms: 3, available_rooms: 4, total_rooms: 8, critical_alerts: 1 },
    arrivals: {
      total: 2,
      items: [
        { reservation_id: 5, code: "RES-000005", guest_name: "Ana Souza", room_code: "101", check_in_date: "2099-10-23", expected_time: "14:00", status: "confirmed", overdue: true, deposit_overdue: false },
        { reservation_id: 6, code: "RES-000006", guest_name: "Bruno Lima", room_code: "102", check_in_date: "2099-10-24", expected_time: "15:00", status: "pending", overdue: false, deposit_overdue: true },
      ],
    },
    departures: {
      total: 1,
      items: [{ stay_id: 3, code: "RES-000003", guest_name: "Carla Dias", room_code: "201", check_out_date: "2099-10-24", expected_time: "12:00", balance_cents: 8000, minutes_late: 60 }],
    },
    at_risk: {
      total: 1,
      items: [{ kind: "maintenance_conflict", title: "Reserva afetada por manutenção: Ana Souza", detail: "Quarto 101 em manutenção", link: "/manutencao/4" }],
    },
    ...patch,
  };
}

const roomMapData: api.RoomMap = {
  from: "2099-10-24",
  to: "2099-10-30",
  rooms: [{ id: 1, code: "101", category_name: "Standard", active: true, state: "available", state_label: "Disponível", free_at: null }],
  cells: [
    { room_id: 1, date: "2099-10-25", reservation_id: 5, code: "RES-000005", guest_name: "Ana Souza", status: "confirmed", is_check_in: true, is_check_out: false },
  ],
};

const routes = [
  { path: "/reservas/:id", element: <p>Detalhe da reserva</p> },
  { path: "/alertas", element: <p>Alertas</p> },
];

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: FIXED_TODAY });
  signIn();
  vi.mocked(api.receptionOverview).mockReset().mockResolvedValue(overview());
  vi.mocked(api.refreshAlerts).mockReset().mockResolvedValue(null);
  vi.mocked(api.alertSummary).mockReset().mockResolvedValue({ critical: 0, high: 0, normal: 0, total: 0, top: [] });
  vi.mocked(api.roomMap).mockReset().mockResolvedValue(roomMapData);
  vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
  vi.mocked(api.getReservation).mockReset().mockResolvedValue({
    reservation: {
      id: 5, code: "RES-000005", room_code: "101", guest_name: "Ana Souza", check_in_date: "2099-10-23", check_out_date: "2099-10-26",
      status: "confirmed", agreed_total_cents: 30000, deposit_paid_cents: 0, deposit_expected_cents: 9000,
    } as api.Reservation,
    nights: [], history: [], stay_id: null, payments: [],
  });
  vi.mocked(api.getStay).mockReset().mockResolvedValue({
    stay: { id: 3, code: "RES-000003", status: "in_house", room_code: "201", guest_name: "Carla Dias", check_out_date: "2099-10-24", expected_check_out_time: "12:00" } as api.Stay,
    occupants: [],
    account: { balance_cents: 8000 } as api.StayAccount,
  });
  vi.mocked(api.listRoomStatus).mockReset().mockResolvedValue([]);
});
afterEach(() => vi.useRealTimers());

async function open(search = "") {
  const view = renderPage(<ReceptionPage />, { path: "/", entry: { pathname: "/", search }, extraRoutes: routes });
  await screen.findByRole("heading", { name: "Recepção" });
  return view;
}

describe("ReceptionPage", () => {
  it("mostra a data e os cinco números do dia", async () => {
    await open();

    expect(screen.getByText("Hoje, 24/10/2099")).toBeInTheDocument();
    const strip = await screen.findByRole("list", { name: "Resumo do dia" });
    expect(within(strip).getByText("Chegadas").nextSibling).toHaveTextContent("2");
    expect(within(strip).getByText("Saídas").nextSibling).toHaveTextContent("1");
    expect(within(strip).getByText("3 de 8")).toBeInTheDocument();
    expect(within(strip).getByText("Quartos disponíveis").nextSibling).toHaveTextContent("4");
    expect(within(strip).getByRole("link", { name: /Alertas críticos/ })).toHaveAttribute("href", "/alertas");
  });

  it("abre em Chegadas; a aba vem da URL e aba inválida cai em Chegadas", async () => {
    await open("?aba=lixo");

    expect(await screen.findByRole("tab", { name: "Chegadas", selected: true })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Saídas" }));
    expect(await screen.findByRole("tab", { name: "Saídas", selected: true })).toBeInTheDocument();
  });

  it("?aba=saidas abre as saídas", async () => {
    await open("?aba=saidas");

    expect(await screen.findByRole("tab", { name: "Saídas", selected: true })).toBeInTheDocument();
    expect(await screen.findByText("Carla Dias")).toBeInTheDocument();
  });

  it("clicar no cartão Saídas seleciona a aba", async () => {
    await open();
    const strip = await screen.findByRole("list", { name: "Resumo do dia" });

    await userEvent.click(within(strip).getByRole("button", { name: /Saídas/ }));

    expect(await screen.findByRole("tab", { name: "Saídas", selected: true })).toBeInTheDocument();
  });

  it("chegada atrasada aparece marcada em texto e leva ao check-in", async () => {
    await open();

    const late = (await screen.findByText("Ana Souza", { selector: "strong" })).closest("li") as HTMLElement;
    expect(within(late).getByText("Atrasada")).toBeInTheDocument();
    expect(within(late).getByRole("link", { name: "Confirmar check-in" })).toHaveAttribute("href", "/reservas/5?abrir=checkin");
    const other = screen.getByText("Bruno Lima").closest("li") as HTMLElement;
    expect(within(other).queryByText("Atrasada")).toBeNull();
    expect(within(other).getByText("Sinal vencido")).toBeInTheDocument();
  });

  it("Ver detalhes abre o painel com o mapa ainda na tela; fechar devolve a lista", async () => {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: "Ver detalhes da reserva RES-000005" }));

    const panel = await screen.findByRole("complementary", { name: "Reserva RES-000005" });
    expect(panel).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.queryByRole("tablist")).toBeNull();

    await userEvent.click(within(panel).getByRole("button", { name: "Fechar painel" }));

    expect(await screen.findByRole("tab", { name: "Chegadas", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("complementary", { name: /Reserva/ })).toBeNull();
  });

  it("Esc fecha o painel logo depois de aberto, sem clicar nele", async () => {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: "Ver detalhes da reserva RES-000005" }));
    await screen.findByRole("complementary", { name: /Reserva/ });

    await userEvent.keyboard("{Escape}");

    expect(await screen.findByRole("tab", { name: "Chegadas", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("complementary", { name: /Reserva/ })).toBeNull();
  });

  it("clicar numa reserva do mapa abre o painel", async () => {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: /Ana Souza · Confirmada · chegada/ }));

    expect(await screen.findByRole("complementary", { name: "Reserva RES-000005" })).toBeInTheDocument();
  });

  it("recarregar com ?painel=hospedagem-3 reabre o mesmo registro", async () => {
    await open("?painel=hospedagem-3");

    expect(await screen.findByRole("complementary", { name: "Hospedagem RES-000003" })).toBeInTheDocument();
  });

  it("painel de registro inexistente avisa sem quebrar; valor inválido é ignorado", async () => {
    vi.mocked(api.getReservation).mockRejectedValue({ code: "not_found", message: "x" });
    const view = await open("?painel=reserva-999");
    expect(await screen.findByText("Não foi possível carregar este registro.")).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    view.router.navigate("/?painel=lixo");

    expect(await screen.findByRole("tab", { name: "Chegadas" })).toBeInTheDocument();
  });

  it("saídas mostram o saldo em palavras, o atraso em texto e só oferecem pagamento com saldo", async () => {
    vi.mocked(api.receptionOverview).mockResolvedValue(
      overview({
        departures: {
          total: 2,
          items: [
            { stay_id: 3, code: "RES-000003", guest_name: "Carla Dias", room_code: "201", check_out_date: "2099-10-24", expected_time: "12:00", balance_cents: 8000, minutes_late: 60 },
            { stay_id: 4, code: "RES-000004", guest_name: "Dani Reis", room_code: "202", check_out_date: "2099-10-24", expected_time: "12:00", balance_cents: 0, minutes_late: 0 },
          ],
        },
      }),
    );
    await open("?aba=saidas");

    const first = (await screen.findByText("Carla Dias")).closest("li") as HTMLElement;
    expect(within(first).getByText("Saldo a receber R$ 80,00")).toBeInTheDocument();
    expect(within(first).getByText("Atrasada há 60 min")).toBeInTheDocument();
    expect(within(first).getByRole("link", { name: "Registrar pagamento" })).toHaveAttribute("href", "/hospedagens/3?abrir=pagamento");
    expect(within(first).getByRole("link", { name: "Concluir check-out" })).toHaveAttribute("href", "/hospedagens/3?abrir=checkout");
    const second = screen.getByText("Dani Reis").closest("li") as HTMLElement;
    expect(within(second).getByText("Quitado")).toBeInTheDocument();
    expect(within(second).queryByRole("link", { name: "Registrar pagamento" })).toBeNull();
    expect(within(second).getByRole("link", { name: "Concluir check-out" })).toBeInTheDocument();
  });

  it("em risco mostra prioridade em texto e leva ao registro; vazio explica", async () => {
    await open("?aba=risco");
    const item = (await screen.findByText(/Reserva afetada por manutenção/)).closest("li") as HTMLElement;
    expect(within(item).getByText("Crítico")).toBeInTheDocument();
    expect(within(item).getByRole("link", { name: "Abrir registro" })).toHaveAttribute("href", "/manutencao/4");
  });

  it("listas vazias explicam o que significa", async () => {
    vi.mocked(api.receptionOverview).mockResolvedValue(
      overview({ arrivals: { items: [], total: 0 }, departures: { items: [], total: 0 }, at_risk: { items: [], total: 0 } }),
    );
    await open();
    expect(await screen.findByText("Nenhuma chegada pendente para hoje.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Saídas" }));
    expect(await screen.findByText("Nenhuma saída pendente para hoje.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Em risco" }));
    expect(await screen.findByText("Nenhum quarto ou reserva em risco agora.")).toBeInTheDocument();
  });

  it("avisa quando a lista passa do limite", async () => {
    vi.mocked(api.receptionOverview).mockResolvedValue(overview({ arrivals: { ...overview().arrivals, total: 55 } }));
    await open();

    expect(await screen.findByText(/Mostrando 2 de 55/)).toBeInTheDocument();
  });

  it("mantém alertas urgentes e a fila de limpeza", async () => {
    vi.mocked(api.cleaningQueue).mockResolvedValue([{ task: { id: 1 } } as api.CleaningItem, { task: { id: 2 } } as api.CleaningItem]);
    vi.mocked(api.alertSummary).mockResolvedValue({
      critical: 1, high: 0, normal: 0, total: 1,
      top: [{
        id: 1, kind: "departure_risk", kind_label: "Risco", severity: "critical", severity_label: "Crítico", title: "Risco para a próxima reserva: quarto 101",
        detail: "Detalhe", link: "/hospedagens/7", status: "active", first_seen_at: "", last_seen_at: "", resolved_at: null, resolved_by_name: null, resolution_note: null, version: 1,
      }],
    });
    await open();

    expect(await screen.findByRole("link", { name: /Risco para a próxima reserva: quarto 101/ })).toHaveAttribute("href", "/hospedagens/7");
    expect(screen.getByRole("link", { name: "Quartos aguardando limpeza: 2" })).toHaveAttribute("href", "/limpeza");
  });

  it("sem alertas e sem limpeza, diz que está tudo em ordem", async () => {
    await open();

    expect(await screen.findByText("Nenhum alerta ativo. Está tudo em ordem.")).toBeInTheDocument();
    expect(await screen.findByText("Nenhum quarto aguardando limpeza.")).toBeInTheDocument();
  });

  it("se a leitura falha, mostra o aviso e mantém o mapa", async () => {
    vi.mocked(api.receptionOverview).mockRejectedValue({ code: "internal", message: "x" });
    await open();

    expect(await screen.findByText("Não foi possível carregar a Recepção. Tente de novo em instantes.")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("table")).toBeInTheDocument());
  });
});
