import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import StayPage from "./StayPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, getStay: vi.fn(), myPermissions: vi.fn(), setStayLateTolerance: vi.fn() };
});

const stay: api.Stay = {
  id: 3, code: "RES-000003", reservation_id: 5, status: "in_house", room_id: 1, room_code: "101", guest_id: 7,
  guest_name: "Ana Souza", check_in_date: "2026-10-21", check_out_date: "2026-10-23", expected_check_in_time: "14:00",
  expected_check_out_time: "12:00", checked_in_at: "2026-10-21T17:00:00Z", checked_out_at: null, early_check_in: false,
  late_tolerance_minutes: null, version: 4,
};

function line(patch: Partial<api.ChargeLine> & Pick<api.ChargeLine, "id" | "kind" | "description" | "amount_cents">): api.ChargeLine {
  return {
    night_date: null, justification: null, reverses_charge_id: null, reversed: false, created_by_name: "Administrador",
    created_at: "2026-10-21T17:00:00Z", ...patch,
  };
}

const account: api.StayAccount = {
  charges: [
    line({ id: 1, kind: "night", description: "Diária de 21/10/2026", amount_cents: 20000, night_date: "2026-10-21" }),
    line({ id: 2, kind: "consumption", description: "Água", amount_cents: 3500, reversed: true }),
    line({ id: 3, kind: "reversal", description: "Estorno de: Água", amount_cents: -3500, reverses_charge_id: 2, justification: "Engano" }),
  ],
  payments: [{ id: 1, code: "PAG-000001", amount_cents: 12000, method: "prior_deposit", received_at: "2026-10-21T17:00:00Z", received_by_name: "Administrador" }],
  refunds: [],
  charges_total_cents: 20000,
  received_cents: 12000,
  refunded_cents: 0,
  balance_cents: 8000,
};

function detail(patch: Partial<api.StayDetail> = {}, stayPatch: Partial<api.Stay> = {}): api.StayDetail {
  return {
    stay: { ...stay, ...stayPatch },
    occupants: [{ id: 1, full_name: "Ana Souza", guest_id: 7, document: "", is_holder: true }],
    account,
    ...patch,
  };
}

const open = () => renderPage(<StayPage />, { path: "/hospedagens/:id", entry: "/hospedagens/3" });

beforeEach(() => {
  signIn();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue(["reverse_entry"]);
  vi.mocked(api.getStay).mockReset().mockResolvedValue(detail());
});

describe("StayPage", () => {
  it("mostra a conta, os recebimentos e o saldo a receber", async () => {
    open();

    expect(await screen.findByText("Hospedagem RES-000003")).toBeInTheDocument();
    expect(screen.getByText("Diária de 21/10/2026")).toBeInTheDocument();
    expect(screen.getByText("PAG-000001")).toBeInTheDocument();
    expect(screen.getByText("Saldo a receber R$ 80,00")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Lançar consumo" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Registrar pagamento" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Concluir check-out" })).toBeInTheDocument();
  });

  it("marca o lançamento estornado com o rótulo, sem depender de cor", async () => {
    open();

    expect(await screen.findByText("Estornado")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Estornar lançamento" })).toHaveLength(1);
  });

  it("não oferece estorno sem a permissão", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue([]);
    open();
    await screen.findByText("Hospedagem RES-000003");

    expect(screen.queryByRole("button", { name: "Estornar lançamento" })).toBeNull();
  });

  it("hospedagem encerrada só consulta", async () => {
    vi.mocked(api.getStay).mockResolvedValue(
      detail({ account: { ...account, balance_cents: 0 } }, { status: "checked_out", checked_out_at: "2026-10-23T15:00:00Z" }),
    );
    open();

    expect(await screen.findByText("Quitado")).toBeInTheDocument();
    expect(screen.getByText(/Encerrada/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Lançar consumo" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Registrar pagamento" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Concluir check-out" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Estornar lançamento" })).toBeNull();
  });

  it("mostra o crédito do hóspede quando recebeu a mais", async () => {
    vi.mocked(api.getStay).mockResolvedValue(detail({ account: { ...account, balance_cents: -6000 } }));
    open();

    expect(await screen.findByText("Crédito do hóspede R$ 60,00")).toBeInTheDocument();
  });

  it("abre os diálogos de lançar, receber e estornar", async () => {
    open();
    await screen.findByText("Hospedagem RES-000003");

    await userEvent.click(screen.getByRole("button", { name: "Lançar consumo" }));
    expect(within(await screen.findByRole("dialog")).getByText("Lançar consumo", { selector: "[data-slot='dialog-title'], h2" })).toBeInTheDocument();
  });

  it("quem pode alterar reservas ajusta a tolerância de saída desta hospedagem", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["change_reservation"]);
    vi.mocked(api.setStayLateTolerance).mockReset().mockResolvedValue(null);
    open();
    const field = await screen.findByLabelText("Tolerância em minutos");

    await userEvent.type(field, "120");
    await userEvent.click(screen.getByRole("button", { name: "Salvar tolerância" }));

    await waitFor(() => expect(api.setStayLateTolerance).toHaveBeenCalledWith("t", 3, 120, 4));
    expect(await screen.findByText("Tolerância de saída tardia salva.")).toBeInTheDocument();
  });

  it("tolerância inválida não chama o backend", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["change_reservation"]);
    vi.mocked(api.setStayLateTolerance).mockReset();
    open();

    await userEvent.type(await screen.findByLabelText("Tolerância em minutos"), "700");
    await userEvent.click(screen.getByRole("button", { name: "Salvar tolerância" }));

    expect(await screen.findByText(/Informe de 0 a 600 minutos/)).toBeInTheDocument();
    expect(api.setStayLateTolerance).not.toHaveBeenCalled();
  });

  it("sem a permissão não mostra a tolerância", async () => {
    open();
    await screen.findByText("Hospedagem RES-000003");

    expect(screen.queryByLabelText("Tolerância em minutos")).toBeNull();
  });
});
