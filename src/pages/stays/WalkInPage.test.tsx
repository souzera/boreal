import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import WalkInPage from "./WalkInPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listGuests: vi.fn(),
    listRooms: vi.fn(),
    quoteStay: vi.fn(),
    checkRoomAvailability: vi.fn(),
    walkIn: vi.fn(),
    myPermissions: vi.fn(),
  };
});

const guest: api.Guest = {
  id: 7, code: "HOSP-000007", full_name: "Ana Souza", phone: "11999990000", cpf: null, email: "", address: "", active: true, version: 1,
};

const quote = (patch: Partial<api.Quote> = {}): api.Quote => ({
  nights: [
    { date: "2099-10-21", amount_cents: 20000, rule: "weekday" },
    { date: "2099-10-22", amount_cents: 20000, rule: "weekday" },
  ],
  total_cents: 40000,
  missing_dates: [],
  ...patch,
});

const routes = [{ path: "/hospedagens/:id", element: <p>Tela da hospedagem</p> }];

beforeEach(() => {
  signIn();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
  vi.mocked(api.listGuests).mockReset().mockResolvedValue({ items: [guest], total: 1, page: 1, page_size: 8 });
  vi.mocked(api.listRooms).mockResolvedValue({
    items: [{ id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, cleaning_interval_minutes: null, version: 1 }],
    total: 1, page: 1, page_size: 100,
  });
  vi.mocked(api.quoteStay).mockReset().mockResolvedValue(quote());
  vi.mocked(api.checkRoomAvailability).mockReset().mockResolvedValue({ available: true, conflicts: [], blocks: [] });
  vi.mocked(api.walkIn).mockReset().mockResolvedValue(9);
});

async function fill() {
  renderPage(<WalkInPage />, { extraRoutes: routes });
  await userEvent.type(await screen.findByLabelText(/Hóspede/), "Ana");
  await userEvent.click(await screen.findByRole("button", { name: /Ana Souza/ }));
  await userEvent.selectOptions(await screen.findByLabelText(/Quarto/), "1");
  await userEvent.type(screen.getByLabelText(/Saída/), "2099-10-23");
}

describe("WalkInPage", () => {
  it("mostra o preço das noites e o total ao vivo", async () => {
    await fill();

    expect(await screen.findByText("R$ 400,00")).toBeInTheDocument();
    expect(await screen.findByText("Quarto disponível agora.")).toBeInTheDocument();
  });

  it("sem tarifa mostra as datas e bloqueia a confirmação", async () => {
    vi.mocked(api.quoteStay).mockResolvedValue(quote({ missing_dates: ["2099-10-22"], total_cents: 20000 }));
    await fill();

    expect(await screen.findByText(/Faltam tarifas para: 22\/10\/2099/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Confirmar check-in" })).toBeDisabled();
  });

  it("quarto ocupado lista os conflitos e bloqueia a confirmação", async () => {
    vi.mocked(api.checkRoomAvailability).mockResolvedValue({
      available: false,
      conflicts: [{ reservation_id: 1, code: "RES-000001", guest_name: "Caio", check_in_date: "2099-10-20", check_out_date: "2099-10-22", status: "checked_in" }],
      blocks: [],
    });
    await fill();

    expect(await screen.findByText(/RES-000001/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Confirmar check-in" })).toBeDisabled();
  });

  it("envia a hospedagem sem reserva e abre a tela da hospedagem", async () => {
    await fill();
    await screen.findByText("R$ 400,00");

    await userEvent.click(screen.getByRole("button", { name: "Confirmar check-in" }));

    await waitFor(() =>
      expect(api.walkIn).toHaveBeenCalledWith("t", {
        room_id: 1,
        guest_id: 7,
        check_out_date: "2099-10-23",
        expected_check_out_time: null,
        occupants: [{ full_name: "Ana Souza", guest_id: 7, document: "" }],
        exceed_capacity: false,
      }),
    );
    expect(await screen.findByText("Tela da hospedagem")).toBeInTheDocument();
  });

  it("erro do backend mantém o formulário preenchido", async () => {
    vi.mocked(api.walkIn).mockRejectedValue({ code: "validation", message: "O quarto já está reservado nesse período: RES-000001." });
    await fill();
    await screen.findByText("R$ 400,00");

    await userEvent.click(screen.getByRole("button", { name: "Confirmar check-in" }));

    expect(await screen.findByText("O quarto já está reservado nesse período: RES-000001.")).toBeInTheDocument();
    expect(screen.getByLabelText(/Saída/)).toHaveValue("2099-10-23");
  });

  it("sem escolher o hóspede, avisa sem chamar o backend", async () => {
    renderPage(<WalkInPage />, { extraRoutes: routes });

    await userEvent.click(await screen.findByRole("button", { name: "Confirmar check-in" }));

    expect(await screen.findByText("Escolha o hóspede.")).toBeInTheDocument();
    expect(api.walkIn).not.toHaveBeenCalled();
  });
});
