import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ReservationsPage from "./ReservationsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listReservations: vi.fn(),
    listReservationsNeedingReview: vi.fn(),
    listRooms: vi.fn(),
  };
});

const base: api.Reservation = {
  id: 1,
  code: "RES-000001",
  room_id: 1,
  room_code: "101",
  guest_id: 7,
  guest_name: "Ana Souza",
  check_in_date: "2026-10-25",
  check_out_date: "2026-10-28",
  expected_check_in_time: "14:00",
  expected_check_out_time: "12:00",
  nights: 3,
  occupants: 2,
  status: "pending",
  list_total_cents: 30000,
  discount_cents: 0,
  discount_reason: null,
  agreed_total_cents: 30000,
  deposit_expected_cents: 9000,
  deposit_paid_cents: 0,
  deposit_due_date: "2026-10-24",
  deposit_overdue: false,
  notes: "",
  cancelled_at: null,
  cancellation_fee_cents: null,
  version: 1,
  created_at: "2026-10-21T12:00:00Z",
  origin: "booking",
};
const confirmed: api.Reservation = {
  ...base, id: 2, code: "RES-000002", guest_name: "Bruno Lima", status: "confirmed", deposit_overdue: false,
};
const overdue: api.Reservation = { ...base, id: 3, code: "RES-000003", guest_name: "Carla Dias", deposit_overdue: true };

const routes = [
  { path: "/reservas/nova", element: <p>Nova reserva aberta</p> },
  { path: "/reservas/:id", element: <p>Reserva aberta</p> },
];

beforeEach(() => {
  signIn();
  vi.mocked(api.listReservations)
    .mockReset()
    .mockResolvedValue({ items: [base, confirmed, overdue], total: 3, page: 1, page_size: 20 });
  vi.mocked(api.listReservationsNeedingReview).mockReset().mockResolvedValue([]);
  vi.mocked(api.listRooms).mockResolvedValue({
    items: [
      { id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, cleaning_interval_minutes: null, version: 1 },
    ],
    total: 1,
    page: 1,
    page_size: 100,
  });
});

async function open() {
  const view = renderPage(<ReservationsPage />, { path: "/reservas", extraRoutes: routes });
  await screen.findByText("Ana Souza");
  return view;
}

function row(code: string) {
  return within(screen.getByText(code).closest("tr") as HTMLElement);
}

describe("ReservationsPage", () => {
  it("lista reservas com datas, valor e situação em texto e ícone", async () => {
    await open();

    const r = row("RES-000001");
    expect(r.getByText("101")).toBeInTheDocument();
    expect(r.getByText("25/10/2026")).toBeInTheDocument();
    expect(r.getByText("28/10/2026")).toBeInTheDocument();
    expect(r.getByText("R$ 300,00")).toBeInTheDocument();
    expect(r.getByText(/Pendente/)).toBeInTheDocument();
    expect(row("RES-000002").getByText(/Confirmada/)).toBeInTheDocument();
  });

  it("destaca sinal vencido com texto, sem alterar a reserva", async () => {
    await open();

    expect(row("RES-000003").getByText("Sinal vencido")).toBeInTheDocument();
    expect(row("RES-000001").queryByText("Sinal vencido")).toBeNull();
  });

  it("filtros são enviados ao backend", async () => {
    await open();

    await userEvent.selectOptions(screen.getByLabelText("Situação"), "confirmed");
    await waitFor(() =>
      expect(api.listReservations).toHaveBeenLastCalledWith("t", expect.objectContaining({ status: ["confirmed"], page: 1 })),
    );
    await userEvent.selectOptions(screen.getByLabelText("Quarto"), "1");
    await waitFor(() => expect(api.listReservations).toHaveBeenLastCalledWith("t", expect.objectContaining({ room_id: 1 })));
    await userEvent.type(screen.getByLabelText("Buscar reserva"), "bruno");
    await waitFor(() => expect(api.listReservations).toHaveBeenLastCalledWith("t", expect.objectContaining({ text: "bruno" })));
    await userEvent.type(screen.getByLabelText("De"), "2026-11-01");
    await waitFor(() => expect(api.listReservations).toHaveBeenLastCalledWith("t", expect.objectContaining({ from: "2026-11-01" })));
  });

  it("avisa quando há reservas que precisam de uma decisão", async () => {
    vi.mocked(api.listReservationsNeedingReview).mockResolvedValue([
      { reservation: base, reasons: ["no_show_candidate", "deposit_overdue"] },
    ]);
    await open();

    const alert = await screen.findByRole("region", { name: "Reservas para revisar" });
    expect(within(alert).getByText(/1 reserva precisa de uma decisão/)).toBeInTheDocument();
    expect(within(alert).getByText(/chegada vencida/i)).toBeInTheDocument();
    expect(within(alert).getByText(/sinal vencido/i)).toBeInTheDocument();
    expect(within(alert).getByRole("link", { name: /RES-000001/ })).toHaveAttribute("href", "/reservas/1");
  });

  it("não mostra o aviso quando não há nada para revisar", async () => {
    await open();

    expect(screen.queryByRole("region", { name: "Reservas para revisar" })).toBeNull();
  });

  it("Nova reserva e clicar numa linha navegam", async () => {
    await open();

    await userEvent.click(screen.getByRole("link", { name: "Nova reserva" }));
    expect(await screen.findByText("Nova reserva aberta")).toBeInTheDocument();
  });

  it("clicar na linha abre a reserva", async () => {
    await open();

    await userEvent.click(screen.getByText("Bruno Lima"));

    expect(await screen.findByText("Reserva aberta")).toBeInTheDocument();
  });

  it("sem resultados orienta a pessoa", async () => {
    vi.mocked(api.listReservations).mockResolvedValue({ items: [], total: 0, page: 1, page_size: 20 });
    renderPage(<ReservationsPage />, { path: "/reservas", extraRoutes: routes });

    expect(await screen.findByText("Nenhuma reserva encontrada.")).toBeInTheDocument();
  });
});
