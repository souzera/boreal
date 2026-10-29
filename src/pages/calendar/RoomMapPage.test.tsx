import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import RoomMapPage from "./RoomMapPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, roomMap: vi.fn() };
});

const FIXED_TODAY = new Date(2099, 9, 24, 12, 0, 0); // 24/10/2099

function map(from: string, patch: Partial<api.RoomMap> = {}): api.RoomMap {
  return {
    from,
    to: from,
    rooms: [
      { id: 1, code: "101", category_name: "Standard", active: true, state: "available", state_label: "Disponível", free_at: null },
      { id: 2, code: "102", category_name: "Luxo", active: false, state: null, state_label: null, free_at: null },
    ],
    cells: [
      { room_id: 1, date: "2099-10-25", reservation_id: 5, code: "RES-000005", guest_name: "Ana Souza", status: "confirmed", is_check_in: true, is_check_out: false },
      { room_id: 1, date: "2099-10-26", reservation_id: 5, code: "RES-000005", guest_name: "Ana Souza", status: "confirmed", is_check_in: false, is_check_out: true },
    ],
    ...patch,
  };
}

const routes = [
  { path: "/limpeza", element: <p>Fila de limpeza</p> },
  { path: "/reservas/:id", element: <p>Detalhe da reserva</p> },
  { path: "/reservas/nova", element: <p>Nova reserva</p> },
];

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: FIXED_TODAY });
  signIn();
  vi.mocked(api.roomMap).mockReset().mockImplementation(async (_t, from, days) => map(from, { to: `${days}` }));
});
afterEach(() => vi.useRealTimers());

async function open() {
  const view = renderPage(<RoomMapPage />, { path: "/mapa", extraRoutes: routes });
  await screen.findByRole("table");
  return view;
}

describe("RoomMapPage", () => {
  it("abre em hoje + 6 dias e mostra a grade com hóspede e situação em texto", async () => {
    await open();

    expect(api.roomMap).toHaveBeenCalledWith("t", "2099-10-24", 7);
    const table = screen.getByRole("table");
    expect(within(table).getByRole("columnheader", { name: /24\/10/ })).toBeInTheDocument();
    expect(within(table).getByRole("columnheader", { name: /30\/10/ })).toBeInTheDocument();
    const arrival = within(table).getByRole("button", { name: /Ana Souza.*Confirmada.*chegada/i });
    expect(arrival).toBeInTheDocument();
    expect(within(table).getByRole("button", { name: /Ana Souza.*saída/i })).toBeInTheDocument();
  });

  it("quarto inativo aparece esmaecido e identificado em texto", async () => {
    await open();

    expect(screen.getByText("102 · Luxo (inativo)")).toBeInTheDocument();
  });

  it("navega para o período anterior, o próximo e volta para hoje", async () => {
    await open();

    await userEvent.click(screen.getByRole("button", { name: "Próximo período" }));
    await waitFor(() => expect(api.roomMap).toHaveBeenLastCalledWith("t", "2099-10-31", 7));

    await userEvent.click(screen.getByRole("button", { name: "Período anterior" }));
    await userEvent.click(screen.getByRole("button", { name: "Período anterior" }));
    await waitFor(() => expect(api.roomMap).toHaveBeenLastCalledWith("t", "2099-10-17", 7));

    await userEvent.click(screen.getByRole("button", { name: "Hoje" }));
    await waitFor(() => expect(api.roomMap).toHaveBeenLastCalledWith("t", "2099-10-24", 7));
  });

  it("clicar numa reserva abre a reserva", async () => {
    await open();

    await userEvent.click(screen.getByRole("button", { name: /Ana Souza.*chegada/i }));

    expect(await screen.findByText("Detalhe da reserva")).toBeInTheDocument();
  });

  it("clicar numa célula livre abre a nova reserva já com quarto e data", async () => {
    const { router } = await open();

    await userEvent.click(screen.getByRole("button", { name: "Reservar quarto 101 em 24/10/2099" }));

    expect(await screen.findByText("Nova reserva")).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/reservas/nova");
    expect(router.state.location.search).toBe("?room=1&date=2099-10-24");
  });

  it("quarto inativo não oferece células livres para reservar", async () => {
    await open();

    expect(screen.queryByRole("button", { name: /Reservar quarto 102/ })).toBeNull();
  });

  it("mostra a mensagem do backend quando o mapa não carrega", async () => {
    vi.mocked(api.roomMap).mockRejectedValue(new api.ApiError("validation", "O mapa mostra de 1 a 31 dias por vez."));
    renderPage(<RoomMapPage />, { path: "/mapa", extraRoutes: routes });

    expect(await screen.findByText("O mapa mostra de 1 a 31 dias por vez.")).toBeInTheDocument();
  });

  it("cada quarto mostra o estado em rótulo e símbolo, e a legenda lista os seis estados", async () => {
    vi.mocked(api.roomMap).mockImplementation(async (_t, from) =>
      map(from, {
        rooms: [
          { id: 1, code: "101", category_name: "Standard", active: true, state: "occupied", state_label: "Ocupado", free_at: null },
          { id: 2, code: "102", category_name: "Standard", active: true, state: "awaiting_cleaning", state_label: "Aguardando limpeza", free_at: null },
          { id: 3, code: "103", category_name: "Standard", active: true, state: "cleaning", state_label: "Em limpeza", free_at: null },
          { id: 4, code: "104", category_name: "Standard", active: true, state: "clean_waiting_interval", state_label: "Limpo — aguardando intervalo", free_at: "2099-10-24T14:00" },
          { id: 5, code: "105", category_name: "Standard", active: true, state: "available", state_label: "Disponível", free_at: null },
          { id: 6, code: "106", category_name: "Standard", active: true, state: "blocked", state_label: "Em manutenção ou bloqueado", free_at: null },
        ],
        cells: [],
      }),
    );
    await open();

    const row104 = within(screen.getByRole("rowheader", { name: /104/ }));
    expect(row104.getByText(/Limpo — aguardando intervalo/)).toBeInTheDocument();
    expect(row104.getByText("livre às 14:00")).toBeInTheDocument();
    expect(within(screen.getByRole("rowheader", { name: /106/ })).getByText(/Em manutenção ou bloqueado/)).toBeInTheDocument();
    const legend = within(screen.getByRole("list", { name: "Legenda dos estados do quarto" }));
    expect(legend.getAllByRole("listitem")).toHaveLength(6);
    expect(legend.getByText(/Aguardando limpeza/)).toBeInTheDocument();
  });

  it("quarto aguardando limpeza leva à fila de limpeza", async () => {
    vi.mocked(api.roomMap).mockImplementation(async (_t, from) =>
      map(from, { rooms: [{ id: 2, code: "102", category_name: "Standard", active: true, state: "awaiting_cleaning", state_label: "Aguardando limpeza", free_at: null }], cells: [] }),
    );
    await open();

    await userEvent.click(within(screen.getByRole("rowheader", { name: /102/ })).getByRole("link", { name: /Aguardando limpeza/ }));

    expect(await screen.findByText("Fila de limpeza")).toBeInTheDocument();
  });

  it("a data do mapa fica no endereço: voltar da fila de limpeza não perde a agenda", async () => {
    const { router } = await open();
    await userEvent.click(screen.getByRole("button", { name: "Próximo período" }));
    await waitFor(() => expect(router.state.location.search).toBe("?de=2099-10-31"));
  });

  it("abre na data do endereço quando ela vem na URL", async () => {
    renderPage(<RoomMapPage />, { path: "/mapa", entry: "/mapa?de=2099-11-07", extraRoutes: routes });

    await screen.findByRole("table");
    expect(api.roomMap).toHaveBeenCalledWith("t", "2099-11-07", 7);
  });
});
