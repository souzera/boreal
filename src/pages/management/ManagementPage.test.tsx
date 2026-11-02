import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fireEvent } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import ManagementPage from "./ManagementPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, managementDashboard: vi.fn() };
});

// Os gráficos não medem tamanho no jsdom; os números são conferidos nas tabelas equivalentes.
vi.mock("recharts", () => {
  const Box = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Box, LineChart: Box, BarChart: Box, PieChart: Box, Pie: Box, Cell: () => null, Line: () => null, Bar: () => null,
    CartesianGrid: () => null, XAxis: () => null, YAxis: () => null, Tooltip: () => null,
  };
});

const FIXED_TODAY = new Date(2099, 9, 24, 12, 0, 0); // 24/10/2099

function dashboard(patch: Partial<api.ManagementDashboard> = {}): api.ManagementDashboard {
  return {
    from: "2099-10-24",
    to: "2099-10-25",
    rooms_total: 4,
    occupancy: [
      { date: "2099-10-24", occupied: 3, rooms: 4, rate_bp: 7500 },
      { date: "2099-10-25", occupied: 1, rooms: 4, rate_bp: 2500 },
    ],
    arrivals: 2,
    departures: 1,
    in_house_now: 3,
    receipts_by_method: [
      { method: "cash", label: "Dinheiro", received_cents: 4000, refunded_cents: 2000, net_cents: 2000 },
      { method: "pix", label: "Pix", received_cents: 10000, refunded_cents: 0, net_cents: 10000 },
      { method: "debit", label: "Cartão de débito", received_cents: 0, refunded_cents: 0, net_cents: 0 },
      { method: "credit", label: "Cartão de crédito", received_cents: 0, refunded_cents: 0, net_cents: 0 },
    ],
    receipts_daily: [
      { date: "2099-10-24", net_cents: 12000 },
      { date: "2099-10-25", net_cents: -3000 },
    ],
    balances: { to_receive_count: 2, to_receive_cents: 15000, credit_count: 1, credit_cents: 1500 },
    payables: { overdue_count: 1, overdue_cents: 50000, open_cents: 60000, by_category: [{ category_name: "Energia do mês", open_cents: 60000 }] },
    ...patch,
  };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: FIXED_TODAY });
  signIn();
  vi.mocked(api.managementDashboard).mockReset().mockResolvedValue(dashboard());
});
afterEach(() => vi.useRealTimers());

function open() {
  return renderPage(<ManagementPage />, { path: "/gerencial" });
}

describe("ManagementPage", () => {
  it("abre com o período de hoje", async () => {
    open();

    await screen.findByRole("list", { name: "Números do período" });
    expect(api.managementDashboard).toHaveBeenCalledWith("t", "2099-10-24", "2099-10-24");
    expect(screen.getByRole("button", { name: "Hoje", pressed: true })).toBeInTheDocument();
  });

  it("Últimos 7 dias e Este mês recalculam as datas", async () => {
    open();
    await screen.findByRole("list", { name: "Números do período" });

    await userEvent.click(screen.getByRole("button", { name: "Últimos 7 dias" }));
    await vi.waitFor(() => expect(api.managementDashboard).toHaveBeenLastCalledWith("t", "2099-10-18", "2099-10-24"));

    await userEvent.click(screen.getByRole("button", { name: "Este mês" }));
    await vi.waitFor(() => expect(api.managementDashboard).toHaveBeenLastCalledWith("t", "2099-10-01", "2099-10-24"));
  });

  it("data final antes da inicial mostra o erro e não consulta", async () => {
    open();
    await screen.findByRole("list", { name: "Números do período" });
    vi.mocked(api.managementDashboard).mockClear();

    fireEvent.change(screen.getByLabelText("Até"), { target: { value: "2099-10-20" } });

    expect(await screen.findByText("A data final não pode ser antes da inicial.")).toBeInTheDocument();
    expect(api.managementDashboard).not.toHaveBeenCalled();
  });

  it("mostra os números em reais e em palavras", async () => {
    open();

    const list = await screen.findByRole("list", { name: "Números do período" });
    const value = (label: string) => within(list).getByText(label).nextSibling;
    expect(value("Chegadas")).toHaveTextContent("2");
    expect(value("Hospedados agora")).toHaveTextContent("3");
    expect(value("Recebido no período")).toHaveTextContent("R$ 120,00");
    expect(value("Saldos a receber")).toHaveTextContent("R$ 150,00 (2)");
    expect(value("Créditos de hóspedes")).toHaveTextContent("R$ 15,00 (1)");
    expect(value("Contas vencidas")).toHaveTextContent("R$ 500,00 (1)");
  });

  it("cada gráfico tem descrição e a tabela com os mesmos números", async () => {
    open();
    await screen.findByRole("list", { name: "Números do período" });

    const occupancy = screen.getByRole("region", { name: "Ocupação por dia" });
    expect(within(occupancy).getByRole("img", { name: /Ocupação por dia\. Percentual de quartos ocupados/ })).toBeInTheDocument();
    expect(within(occupancy).getByText("75,0%")).toBeInTheDocument();
    expect(within(occupancy).getByText("3 de 4")).toBeInTheDocument();

    const receipts = screen.getByRole("region", { name: "Recebimentos por dia" });
    expect(within(receipts).getByText("R$ 120,00")).toBeInTheDocument();
    expect(within(receipts).getByText("-R$ 30,00")).toBeInTheDocument();

    const payables = screen.getByRole("region", { name: "Contas em aberto por categoria" });
    expect(within(payables).getByText("Energia do mês")).toBeInTheDocument();
    expect(within(payables).getByText("R$ 600,00")).toBeInTheDocument();
  });

  it("a tabela por forma de pagamento separa débito e crédito e não inventa 'Sem modalidade'", async () => {
    open();
    await screen.findByRole("list", { name: "Números do período" });

    const methods = screen.getByRole("region", { name: "Recebimentos por forma de pagamento" });
    expect(within(methods).getByRole("cell", { name: "Cartão de débito" })).toBeInTheDocument();
    expect(within(methods).getByRole("cell", { name: "Cartão de crédito" })).toBeInTheDocument();
    expect(within(methods).queryByText("Sem modalidade")).toBeNull();
    const cash = within(methods).getByRole("cell", { name: "Dinheiro" }).closest("tr") as HTMLElement;
    expect(within(cash).getAllByRole("cell").map((c) => c.textContent)).toEqual(["Dinheiro", "R$ 40,00", "R$ 20,00", "R$ 20,00"]);
  });

  it("período sem movimento diz isso em vez de desenhar gráfico vazio", async () => {
    vi.mocked(api.managementDashboard).mockResolvedValue(
      dashboard({
        occupancy: [{ date: "2099-10-24", occupied: 0, rooms: 4, rate_bp: 0 }],
        receipts_daily: [{ date: "2099-10-24", net_cents: 0 }],
        payables: { overdue_count: 0, overdue_cents: 0, open_cents: 0, by_category: [] },
      }),
    );
    open();
    await screen.findByRole("list", { name: "Números do período" });

    expect(screen.getAllByText("Sem movimento no período.")).toHaveLength(3);
    expect(screen.queryAllByRole("img")).toHaveLength(0);
  });

  it("mostra a recusa do backend", async () => {
    vi.mocked(api.managementDashboard).mockRejectedValue({ code: "forbidden", message: "Você não tem permissão para esta ação." });
    open();

    expect(await screen.findByText("Você não tem permissão para esta ação.")).toBeInTheDocument();
  });

  it("quem não é administrador vê o aviso e nada é consultado", () => {
    signIn(recepcaoUser);
    open();

    expect(screen.getByText("Esta área é só para administradores.")).toBeInTheDocument();
    expect(api.managementDashboard).not.toHaveBeenCalled();
  });
});
