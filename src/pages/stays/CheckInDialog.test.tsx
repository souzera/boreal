import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import CheckInDialog from "./CheckInDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, checkInReservation: vi.fn(), listRooms: vi.fn(), myPermissions: vi.fn() };
});

const reservation: api.Reservation = {
  id: 5, code: "RES-000005", room_id: 1, room_code: "101", guest_id: 7, guest_name: "Ana Souza",
  check_in_date: "2000-01-01", check_out_date: "2000-01-03", expected_check_in_time: "14:00",
  expected_check_out_time: "12:00", nights: 2, occupants: 2, status: "confirmed", list_total_cents: 30000,
  discount_cents: 3000, discount_reason: "Cliente antigo", agreed_total_cents: 27000, deposit_expected_cents: 8100,
  deposit_paid_cents: 4000, deposit_due_date: "1999-12-30", deposit_overdue: false, notes: "",
  cancelled_at: null, cancellation_fee_cents: null, version: 3, created_at: "1999-12-01T12:00:00Z", origin: "booking",
};

const onDone = vi.fn();
const onClose = vi.fn();

function open(patch: Partial<api.Reservation> = {}) {
  return renderPage(<CheckInDialog open reservation={{ ...reservation, ...patch }} onClose={onClose} onDone={onDone} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  onClose.mockReset();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
  vi.mocked(api.checkInReservation).mockReset().mockResolvedValue(11);
  vi.mocked(api.listRooms).mockResolvedValue({
    items: [{ id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, cleaning_interval_minutes: null, version: 1 }],
    total: 1, page: 1, page_size: 100,
  });
});

describe("CheckInDialog", () => {
  it("pré-preenche o titular e mostra o saldo inicial", async () => {
    open();
    const d = await dialog();

    expect(d.getByText("Ana Souza (titular)")).toBeInTheDocument();
    expect(d.getByText("Saldo inicial: R$ 230,00")).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Confirmar check-in" })).toBeEnabled();
  });

  it("ocupante sem nome não chama o backend", async () => {
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.click(d.getByRole("button", { name: "Confirmar check-in" }));

    expect(await d.findByText("Informe o nome de todos os ocupantes.")).toBeInTheDocument();
    expect(api.checkInReservation).not.toHaveBeenCalled();
  });

  it("envia o titular e os acompanhantes e avisa o resultado", async () => {
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 2"), "Bia Souza");
    await userEvent.click(d.getByRole("button", { name: "Confirmar check-in" }));

    await waitFor(() =>
      expect(api.checkInReservation).toHaveBeenCalledWith("t", {
        reservation_id: 5,
        expected_version: 3,
        occupants: [
          { full_name: "Ana Souza", guest_id: 7, document: "" },
          { full_name: "Bia Souza", guest_id: null, document: "" },
        ],
        exceed_capacity: false,
      }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith(11));
  });

  it("check-in antecipado avisa e fica bloqueado sem a permissão", async () => {
    open({ check_in_date: "2099-01-01", check_out_date: "2099-01-03" });
    const d = await dialog();

    expect(await d.findByText(/Check-in antes do horário previsto/)).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Confirmar check-in" })).toBeDisabled();
  });

  it("check-in antecipado é liberado com a permissão", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["early_check_in"]);
    open({ check_in_date: "2099-01-01", check_out_date: "2099-01-03" });
    const d = await dialog();

    await waitFor(() => expect(d.getByRole("button", { name: "Confirmar check-in" })).toBeEnabled());
  });

  it("excesso de capacidade exige a permissão e a confirmação", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["exceed_capacity"]);
    open();
    const d = await dialog();
    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 2"), "Bia");
    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 3"), "Caio");

    expect(await d.findByText(/O quarto 101 comporta 2 pessoa\(s\)/)).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Confirmar check-in" })).toBeDisabled();
    await userEvent.click(d.getByLabelText("Autorizar exceder a capacidade"));
    await userEvent.click(d.getByRole("button", { name: "Confirmar check-in" }));

    await waitFor(() => expect(api.checkInReservation).toHaveBeenCalledWith("t", expect.objectContaining({ exceed_capacity: true })));
  });

  it("sem permissão, excesso de capacidade fica bloqueado com explicação", async () => {
    open();
    const d = await dialog();
    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 2"), "Bia");
    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 3"), "Caio");

    expect(await d.findByText(/peça a quem pode autorizar/)).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Confirmar check-in" })).toBeDisabled();
  });

  it("erro do backend mantém o que foi preenchido", async () => {
    vi.mocked(api.checkInReservation).mockRejectedValue({ code: "validation", message: "O quarto já está reservado nesse período: RES-000001." });
    open();
    const d = await dialog();
    await userEvent.click(d.getByRole("button", { name: "Adicionar ocupante" }));
    await userEvent.type(d.getByLabelText("Nome do ocupante 2"), "Bia");

    await userEvent.click(d.getByRole("button", { name: "Confirmar check-in" }));

    expect(await d.findByText("O quarto já está reservado nesse período: RES-000001.")).toBeInTheDocument();
    expect(d.getByLabelText("Nome do ocupante 2")).toHaveValue("Bia");
    expect(onDone).not.toHaveBeenCalled();
  });
});
