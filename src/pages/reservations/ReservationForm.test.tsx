import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ReservationFormPage from "./ReservationFormPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listGuests: vi.fn(),
    listRooms: vi.fn(),
    checkRoomAvailability: vi.fn(),
    quoteStay: vi.fn(),
    getSettings: vi.fn(),
    createReservation: vi.fn(),
    myPermissions: vi.fn(),
  };
});

const ana: api.Guest = {
  id: 7, code: "HOS-000007", full_name: "Ana Souza", phone: "11999990000", cpf: null, email: "", address: "",
  active: true, version: 1,
};
const room101: api.Room = {
  id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "",
  active: true, version: 1,
};
const settings: api.AppSettings = {
  max_failed_attempts: 5, lockout_minutes: 5, idle_lock_minutes: 10, default_check_in_time: "14:00",
  default_check_out_time: "12:00", late_checkout_tolerance_minutes: 30, no_show_tolerance_minutes: 60,
  cleaning_interval_minutes: 60, deposit_percent_bp: 3000, deposit_due_days_after_booking: 3,
  deposit_due_days_before_arrival: 1,
};
const quote: api.Quote = {
  nights: [
    { date: "2099-10-25", amount_cents: 10000, rule: "weekday" },
    { date: "2099-10-26", amount_cents: 10000, rule: "weekday" },
    { date: "2099-10-27", amount_cents: 10000, rule: "weekday" },
  ],
  total_cents: 30000,
  missing_dates: [],
};

const routes = [
  { path: "/reservas/:id", element: <p>Reserva aberta</p> },
  { path: "/hospedes/novo", element: <p>Novo hóspede</p> },
];

beforeEach(() => {
  signIn();
  vi.mocked(api.listGuests).mockReset().mockResolvedValue({ items: [ana], total: 1, page: 1, page_size: 8 });
  vi.mocked(api.listRooms).mockResolvedValue({ items: [room101], total: 1, page: 1, page_size: 100 });
  vi.mocked(api.checkRoomAvailability).mockReset().mockResolvedValue({ available: true, conflicts: [] });
  vi.mocked(api.quoteStay).mockReset().mockResolvedValue(quote);
  vi.mocked(api.getSettings).mockResolvedValue({ settings, version: 1 });
  vi.mocked(api.createReservation).mockReset().mockResolvedValue({
    id: 9, code: "RES-000009", agreed_total_cents: 30000, deposit_expected_cents: 9000, deposit_due_date: "2099-10-24",
  });
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
});

function open(entry = "/reservas/nova") {
  const view = renderPage(<ReservationFormPage />, { path: "/reservas/nova", entry, extraRoutes: routes });
  return view;
}

async function pickGuest() {
  await userEvent.type(screen.getByLabelText(/Hóspede/), "ana");
  await userEvent.click(await screen.findByRole("button", { name: /Ana Souza/ }));
}

async function fill({ guest = true, from = "2099-10-25", to = "2099-10-28", occupants = "2" } = {}) {
  if (guest) await pickGuest();
  await userEvent.selectOptions(await screen.findByLabelText(/Quarto/), "1");
  await userEvent.type(screen.getByLabelText(/Entrada/), from);
  await userEvent.type(screen.getByLabelText(/Saída/), to);
  await userEvent.clear(screen.getByLabelText(/Pessoas/));
  await userEvent.type(screen.getByLabelText(/Pessoas/), occupants);
}

describe("ReservationFormPage", () => {
  it("campos obrigatórios com * e legenda, e erros por campo", async () => {
    open();

    expect(screen.getByText("* campo obrigatório")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));

    expect(await screen.findByText("Escolha o hóspede.")).toBeInTheDocument();
    expect(screen.getByText("Escolha o quarto.")).toBeInTheDocument();
    expect(screen.getByText("Informe a data de entrada.")).toBeInTheDocument();
    expect(screen.getByText("Informe a data de saída.")).toBeInTheDocument();
    expect(api.createReservation).not.toHaveBeenCalled();
  });

  it("mostra disponibilidade, preço noite a noite, total e sinal esperado ao vivo", async () => {
    open();
    await fill();

    expect(await screen.findByText("Quarto disponível no período.")).toBeInTheDocument();
    expect(await screen.findByText("R$ 300,00")).toBeInTheDocument();
    expect(screen.getByText(/Sinal esperado: R\$ 90,00 \(30%\)/)).toBeInTheDocument();
    expect(screen.getAllByText("Dia da semana").length).toBe(3);
    expect(api.quoteStay).toHaveBeenLastCalledWith("t", 1, "2099-10-25", "2099-10-28");
  });

  it("conflito: mostra a reserva que bloqueia e impede criar", async () => {
    vi.mocked(api.checkRoomAvailability).mockResolvedValue({
      available: false,
      conflicts: [
        { reservation_id: 4, code: "RES-000004", guest_name: "Bruno Lima", check_in_date: "2099-10-26", check_out_date: "2099-10-29", status: "confirmed" },
      ],
    });
    open();
    await fill();

    expect(await screen.findByText(/RES-000004/)).toBeInTheDocument();
    expect(screen.getByText(/Bruno Lima/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Criar reserva" })).toBeDisabled();
  });

  it("datas sem tarifa são listadas e impedem criar", async () => {
    vi.mocked(api.quoteStay).mockResolvedValue({
      nights: [quote.nights[0]], total_cents: 10000, missing_dates: ["2099-10-26", "2099-10-27"],
    });
    open();
    await fill();

    expect(await screen.findByText(/Faltam tarifas para: 26\/10\/2099, 27\/10\/2099/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Criar reserva" })).toBeDisabled();
  });

  it("saída antes da entrada e entrada no passado são recusadas na tela", async () => {
    open();
    await fill({ from: "2099-10-28", to: "2099-10-25" });
    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));
    expect(await screen.findByText("A saída deve ser depois da entrada.")).toBeInTheDocument();

    await userEvent.clear(screen.getByLabelText(/Entrada/));
    await userEvent.type(screen.getByLabelText(/Entrada/), "2000-01-01");
    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));
    expect(await screen.findByText("A data de entrada não pode estar no passado.")).toBeInTheDocument();
    expect(api.createReservation).not.toHaveBeenCalled();
  });

  it("cria a reserva e abre a tela dela", async () => {
    open();
    await fill();
    await screen.findByText("Quarto disponível no período.");
    await userEvent.type(screen.getByLabelText(/Observações/), "Chegada tarde");

    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));

    await waitFor(() =>
      expect(api.createReservation).toHaveBeenCalledWith("t", {
        room_id: 1, guest_id: 7, check_in_date: "2099-10-25", check_out_date: "2099-10-28",
        expected_check_in_time: null, expected_check_out_time: null, occupants: 2, notes: "Chegada tarde",
        exceed_capacity: false,
      }),
    );
    expect(await screen.findByText("Reserva aberta")).toBeInTheDocument();
  });

  it("erro do backend mantém o formulário preenchido", async () => {
    vi.mocked(api.createReservation).mockRejectedValue(new api.ApiError("validation", "O quarto já está reservado nesse período."));
    open();
    await fill();
    await screen.findByText("Quarto disponível no período.");

    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));

    expect(await screen.findByText("O quarto já está reservado nesse período.")).toBeInTheDocument();
    expect(screen.getByLabelText(/Entrada/)).toHaveValue("2099-10-25");
  });

  it("acima da capacidade: sem permissão não deixa criar", async () => {
    open();
    await fill({ occupants: "3" });

    expect(await screen.findByText(/comporta 2 pessoa/)).toBeInTheDocument();
    expect(screen.queryByLabelText("Autorizar exceder a capacidade")).toBeNull();
    expect(screen.getByRole("button", { name: "Criar reserva" })).toBeDisabled();
  });

  it("acima da capacidade: com a permissão, pede autorização e envia", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["exceed_capacity"]);
    open();
    await fill({ occupants: "3" });

    await userEvent.click(await screen.findByLabelText("Autorizar exceder a capacidade"));
    await screen.findByText("Quarto disponível no período.");
    await userEvent.click(screen.getByRole("button", { name: "Criar reserva" }));

    await waitFor(() => expect(api.createReservation).toHaveBeenCalledWith("t", expect.objectContaining({ occupants: 3, exceed_capacity: true })));
  });

  it("vem preenchida quando aberta pelo mapa de quartos", async () => {
    open("/reservas/nova?room=1&date=2099-10-25");

    await waitFor(() => expect(screen.getByLabelText(/Quarto/)).toHaveValue("1"));
    expect(screen.getByLabelText(/Entrada/)).toHaveValue("2099-10-25");
  });

  it("sair com alterações pede decisão", async () => {
    const { router } = open();
    await userEvent.type(screen.getByLabelText(/Observações/), "algo");

    act(() => void router.navigate("/outra"));

    expect(await screen.findByText("Alterações não salvas")).toBeInTheDocument();
  });
});
