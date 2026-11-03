import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import StaysPage from "./StaysPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listStays: vi.fn() };
});

const stay: api.Stay = {
  id: 3, code: "RES-000003", reservation_id: 5, status: "in_house", room_id: 1, room_code: "101", guest_id: 7,
  guest_name: "Ana Souza", check_in_date: "2026-10-21", check_out_date: "2026-10-23", expected_check_in_time: "14:00",
  expected_check_out_time: "12:00", checked_in_at: "2026-10-21T17:00:00Z", checked_out_at: null, early_check_in: false,
  late_tolerance_minutes: null, version: 4,
};

function page(items: api.StaySummary[]): api.Page<api.StaySummary> {
  return { items, total: items.length, page: 1, page_size: 20 };
}

beforeEach(() => {
  signIn();
  vi.mocked(api.listStays).mockReset().mockResolvedValue(page([{ stay, balance_cents: 24000, minutes_late: 0, risk: null }]));
});

describe("StaysPage", () => {
  it("lista as hospedagens em andamento com o saldo em reais", async () => {
    renderPage(<StaysPage />);

    expect(await screen.findByText("Ana Souza")).toBeInTheDocument();
    expect(screen.getByText("RES-000003")).toBeInTheDocument();
    expect(screen.getByText("R$ 240,00")).toBeInTheDocument();
    expect(within(screen.getByRole("row", { name: /RES-000003/ })).getByText(/Hospedado/)).toBeInTheDocument();
    expect(api.listStays).toHaveBeenCalledWith("t", expect.objectContaining({ status: "in_house" }));
  });

  it("avisa saída atrasada e risco para a próxima reserva em texto", async () => {
    vi.mocked(api.listStays).mockResolvedValue(
      page([
        {
          stay,
          balance_cents: 0,
          minutes_late: 45,
          risk: { next_reservation_id: 9, next_code: "RES-000009", next_start: "2026-10-23T14:00", ready_at: "2026-10-23T14:30", overrun_minutes: 30 },
        },
      ]),
    );
    renderPage(<StaysPage />);

    expect(await screen.findByText(/Saída atrasada há 45 min/)).toBeInTheDocument();
    expect(screen.getByText(/Risco para a próxima reserva RES-000009/)).toBeInTheDocument();
  });

  it("mostra a mensagem de lista vazia", async () => {
    vi.mocked(api.listStays).mockResolvedValue(page([]));
    renderPage(<StaysPage />);

    expect(await screen.findByText("Nenhuma hospedagem em andamento.")).toBeInTheDocument();
  });

  it("troca para as hospedagens encerradas", async () => {
    renderPage(<StaysPage />);
    await screen.findByText("Ana Souza");

    await userEvent.selectOptions(screen.getByLabelText("Situação"), "checked_out");

    await waitFor(() => expect(api.listStays).toHaveBeenLastCalledWith("t", expect.objectContaining({ status: "checked_out" })));
  });

  it("oferece a hospedagem sem reserva", async () => {
    renderPage(<StaysPage />, { extraRoutes: [{ path: "/hospedagens/nova", element: <p>Tela do walk-in</p> }] });
    await screen.findByText("Ana Souza");

    await userEvent.click(screen.getByRole("link", { name: "Hospedagem sem reserva" }));

    expect(await screen.findByText("Tela do walk-in")).toBeInTheDocument();
    expect(within(document.body).queryByText("Ana Souza")).toBeNull();
  });
});
