import { screen } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import { GuestPanel } from "./GuestPanel";
import { ReservationPanel } from "./ReservationPanel";
import { RoomPanel } from "./RoomPanel";
import { StayPanel } from "./StayPanel";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, getReservation: vi.fn(), getStay: vi.fn(), getGuest: vi.fn(), listRoomStatus: vi.fn() };
});

const FIXED_TODAY = new Date(2099, 9, 24, 12, 0, 0); // 24/10/2099

function reservation(patch: Partial<api.Reservation> = {}): api.ReservationDetail {
  return {
    reservation: {
      id: 5, code: "RES-000005", room_code: "101", guest_name: "Ana Souza", check_in_date: "2099-10-24", check_out_date: "2099-10-26",
      status: "confirmed", agreed_total_cents: 30000, deposit_paid_cents: 5000, deposit_expected_cents: 9000, ...patch,
    } as api.Reservation,
    nights: [], history: [], stay_id: null, payments: [],
  };
}

function stay(status: "in_house" | "checked_out", balance: number): api.StayDetail {
  return {
    stay: {
      id: 3, code: "RES-000003", status, room_code: "101", guest_name: "Ana Souza", check_out_date: "2099-10-26", expected_check_out_time: "12:00",
    } as api.Stay,
    occupants: [],
    account: { balance_cents: balance } as api.StayAccount,
  };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: FIXED_TODAY });
  signIn();
});
afterEach(() => vi.useRealTimers());

const noop = () => {};

describe("ReservationPanel", () => {
  it("resume a reserva e oferece o check-in e a página completa", async () => {
    vi.mocked(api.getReservation).mockResolvedValue(reservation());
    renderPage(<ReservationPanel id={5} onClose={noop} />);

    expect(await screen.findByText("Ana Souza")).toBeInTheDocument();
    expect(screen.getByText("24/10/2099 a 26/10/2099")).toBeInTheDocument();
    expect(screen.getByText("R$ 300,00")).toBeInTheDocument();
    expect(screen.getByText("R$ 50,00 de R$ 90,00")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Confirmar check-in" })).toHaveAttribute("href", "/reservas/5?abrir=checkin");
    expect(screen.getByRole("link", { name: "Abrir reserva completa" })).toHaveAttribute("href", "/reservas/5");
  });

  it("não oferece o check-in antes da chegada nem em reserva cancelada", async () => {
    vi.mocked(api.getReservation).mockResolvedValue(reservation({ check_in_date: "2099-10-27" }));
    const first = renderPage(<ReservationPanel id={5} onClose={noop} />);
    await screen.findByText("Ana Souza");
    expect(screen.queryByRole("link", { name: "Confirmar check-in" })).toBeNull();
    first.queryClient.clear();
  });

  it("avisa quando o registro não carrega", async () => {
    vi.mocked(api.getReservation).mockRejectedValue({ code: "not_found", message: "x" });
    renderPage(<ReservationPanel id={999} onClose={noop} />);

    expect(await screen.findByText("Não foi possível carregar este registro.")).toBeInTheDocument();
  });
});

describe("StayPanel", () => {
  it("mostra o saldo em palavras e as ações da hospedagem em andamento", async () => {
    vi.mocked(api.getStay).mockResolvedValue(stay("in_house", 8000));
    renderPage(<StayPanel id={3} onClose={noop} />);

    expect(await screen.findByText("Saldo a receber R$ 80,00")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Registrar pagamento" })).toHaveAttribute("href", "/hospedagens/3?abrir=pagamento");
    expect(screen.getByRole("link", { name: "Concluir check-out" })).toHaveAttribute("href", "/hospedagens/3?abrir=checkout");
  });

  it("não oferece pagamento nem check-out depois do check-out", async () => {
    vi.mocked(api.getStay).mockResolvedValue(stay("checked_out", 0));
    renderPage(<StayPanel id={3} onClose={noop} />);

    expect(await screen.findByText("Check-out feito")).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Registrar pagamento" })).toBeNull();
    expect(screen.getByRole("link", { name: "Abrir hospedagem completa" })).toHaveAttribute("href", "/hospedagens/3");
  });
});

describe("GuestPanel", () => {
  it("mostra o hóspede e leva ao cadastro completo", async () => {
    vi.mocked(api.getGuest).mockResolvedValue({ id: 7, code: "HOS-000007", full_name: "Ana Souza", phone: "", active: true } as api.Guest);
    renderPage(<GuestPanel id={7} onClose={noop} />);

    expect(await screen.findByText("Não informado")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Abrir cadastro completo" })).toHaveAttribute("href", "/hospedes/7");
  });
});

describe("RoomPanel", () => {
  const rooms = [
    { room_id: 1, code: "101", category_name: "Standard", state: "awaiting_cleaning", label_pt: "Aguardando limpeza", free_at: null, cleaning_task_id: 2 },
    { room_id: 2, code: "102", category_name: "Luxo", state: "available", label_pt: "Disponível", free_at: null, cleaning_task_id: null },
  ] as api.RoomStatus[];

  it("mostra a situação e leva à fila de limpeza quando precisa", async () => {
    vi.mocked(api.listRoomStatus).mockResolvedValue(rooms);
    renderPage(<RoomPanel id={1} onClose={noop} />);

    expect(await screen.findByText("Standard")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Ver na fila de limpeza" })).toHaveAttribute("href", "/limpeza");
  });

  it("quarto inexistente mostra o aviso", async () => {
    vi.mocked(api.listRoomStatus).mockResolvedValue(rooms);
    renderPage(<RoomPanel id={99} onClose={noop} />);

    expect(await screen.findByText("Não foi possível carregar este registro.")).toBeInTheDocument();
  });
});
