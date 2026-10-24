import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import CheckoutDialog from "./CheckoutDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, previewStayCheckout: vi.fn(), checkoutStay: vi.fn(), myPermissions: vi.fn() };
});

const stay: api.Stay = {
  id: 3, code: "RES-000003", reservation_id: 5, status: "in_house", room_id: 1, room_code: "101", guest_id: 7,
  guest_name: "Ana Souza", check_in_date: "2026-10-21", check_out_date: "2026-10-23", expected_check_in_time: "14:00",
  expected_check_out_time: "12:00", checked_in_at: "2026-10-21T17:00:00Z", checked_out_at: null, early_check_in: false,
  late_tolerance_minutes: null, version: 6,
};

function line(id: number, kind: api.ChargeKind, description: string, amount: number, extra: Partial<api.ChargeLine> = {}): api.ChargeLine {
  return {
    id, kind, description, amount_cents: amount, night_date: null, justification: null, reverses_charge_id: null,
    reversed: false, created_by_name: "Administrador", created_at: "2026-10-21T17:00:00Z", ...extra,
  };
}

const account = (balance: number): api.StayAccount => ({
  charges: [
    line(1, "night", "Diária de 21/10/2026", 20000),
    line(2, "night", "Diária de 22/10/2026", 20000),
    line(3, "discount", "Desconto concedido na reserva", -4000),
    line(4, "consumption", "Água", 3500, { reversed: true }),
    line(5, "reversal", "Estorno de: Água", -3500, { reverses_charge_id: 4 }),
  ],
  payments: [{ id: 1, code: "PAG-000001", amount_cents: 12000, method: "prior_deposit", received_at: "2026-10-21T17:00:00Z", received_by_name: "Administrador" }],
  refunds: [],
  charges_total_cents: 36000,
  received_cents: 12000,
  refunded_cents: 0,
  balance_cents: balance,
});

function preview(patch: Partial<api.CheckoutPreview> = {}, balance = 0): api.CheckoutPreview {
  return {
    stay,
    account: account(balance),
    late: { minutes_late: 0, tolerance_minutes: 30, applies: false, fee_cents: 0 },
    balance_cents: balance,
    balance_with_late_fee_cents: balance,
    risk: null,
    ...patch,
  };
}

const lateInfo: api.LateInfo = { minutes_late: 45, tolerance_minutes: 30, applies: true, fee_cents: 10000 };

const onDone = vi.fn();

function open() {
  renderPage(<CheckoutDialog open stayId={3} onClose={vi.fn()} onDone={onDone} />);
}

async function dialog() {
  const d = within(await screen.findByRole("dialog"));
  await d.findByText("Demonstrativo");
  return d;
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
  vi.mocked(api.previewStayCheckout).mockReset().mockResolvedValue(preview());
  vi.mocked(api.checkoutStay).mockReset().mockResolvedValue(null);
});

describe("CheckoutDialog", () => {
  it("mostra o demonstrativo sem contar o lançamento estornado", async () => {
    open();
    const d = await dialog();

    expect(d.getByText("Diárias")).toBeInTheDocument();
    expect(d.getByText("R$ 400,00")).toBeInTheDocument();
    expect(d.getByText("Descontos e ajustes")).toBeInTheDocument();
    expect(d.getByText("− R$ 40,00")).toBeInTheDocument();
    expect(d.getByText("Recebimentos")).toBeInTheDocument();
    expect(d.getByText("R$ 120,00")).toBeInTheDocument();
    expect(d.getByText("Conta quitada.")).toBeInTheDocument();
  });

  it("conta quitada conclui o check-out com a versão da prévia", async () => {
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Concluir check-out" }));

    await waitFor(() =>
      expect(api.checkoutStay).toHaveBeenCalledWith("t", { stay_id: 3, expected_version: 6, late_fee: "not_applicable", balance_exception: null }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Check-out concluído e salvo neste computador."));
  });

  it("saída tardia exige escolher cobrar ou dispensar e avisa que nenhuma diária é criada", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({ late: lateInfo, balance_with_late_fee_cents: 10000 }));
    vi.mocked(api.myPermissions).mockResolvedValue(["waive_late_fee"]);
    open();
    const d = await dialog();

    expect(d.getByText(/Saída 45 min depois do previsto; cobrança prevista R\$ 100,00/)).toBeInTheDocument();
    expect(d.getByText(/Nenhuma diária é criada automaticamente/)).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Concluir check-out" })).toBeDisabled();
  });

  it("dispensar a cobrança pede a justificativa e envia a decisão", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({ late: lateInfo, balance_with_late_fee_cents: 10000 }));
    vi.mocked(api.myPermissions).mockResolvedValue(["waive_late_fee"]);
    open();
    const d = await dialog();

    await userEvent.click(await d.findByLabelText("Dispensar cobrança"));
    await userEvent.click(d.getByRole("button", { name: "Concluir check-out" }));
    expect(await d.findByText("Informe a justificativa para dispensar a cobrança.")).toBeInTheDocument();
    expect(api.checkoutStay).not.toHaveBeenCalled();
    await userEvent.type(d.getByLabelText(/Justificativa para dispensar/), "Problema no chuveiro");
    await userEvent.click(d.getByRole("button", { name: "Concluir check-out" }));

    await waitFor(() =>
      expect(api.checkoutStay).toHaveBeenCalledWith("t", {
        stay_id: 3,
        expected_version: 6,
        late_fee: { waive: { justification: "Problema no chuveiro" } },
        balance_exception: null,
      }),
    );
  });

  it("sem permissão não oferece dispensar a cobrança", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({ late: lateInfo, balance_with_late_fee_cents: 10000 }));
    open();
    const d = await dialog();

    expect(d.getByLabelText("Cobrar saída tardia")).toBeInTheDocument();
    expect(d.queryByLabelText("Dispensar cobrança")).toBeNull();
  });

  it("cobrar a saída tardia soma ao saldo e exige a exceção de saldo", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({ late: lateInfo, balance_with_late_fee_cents: 10000 }));
    vi.mocked(api.myPermissions).mockResolvedValue(["checkout_with_balance"]);
    open();
    const d = await dialog();

    await userEvent.click(await d.findByLabelText("Cobrar saída tardia"));

    expect(d.getByText("Saldo em aberto: R$ 100,00")).toBeInTheDocument();
    expect(d.getByRole("button", { name: "Concluir check-out" })).toBeDisabled();
    await userEvent.click(d.getByLabelText("Concluir com saldo em aberto"));
    await userEvent.type(d.getByLabelText(/Justificativa do saldo/), "Paga pelo Pix amanhã");
    await userEvent.click(d.getByRole("button", { name: "Concluir check-out" }));

    await waitFor(() =>
      expect(api.checkoutStay).toHaveBeenCalledWith("t", {
        stay_id: 3,
        expected_version: 6,
        late_fee: "charge",
        balance_exception: { justification: "Paga pelo Pix amanhã" },
      }),
    );
  });

  it("saldo em aberto sem permissão fica bloqueado e explica", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({}, 24000));
    open();
    const d = await dialog();

    expect(d.getByText("Saldo em aberto: R$ 240,00")).toBeInTheDocument();
    expect(d.getByText(/Peça a quem pode autorizar/)).toBeInTheDocument();
    expect(d.queryByLabelText("Concluir com saldo em aberto")).toBeNull();
    expect(d.getByRole("button", { name: "Concluir check-out" })).toBeDisabled();
  });

  it("crédito do hóspede mostra o valor a devolver", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(preview({}, -6000));
    open();
    const d = await dialog();

    expect(d.getByText("Valor a devolver ao hóspede R$ 60,00")).toBeInTheDocument();
  });

  it("avisa o risco para a próxima reserva", async () => {
    vi.mocked(api.previewStayCheckout).mockResolvedValue(
      preview({ risk: { next_reservation_id: 9, next_code: "RES-000009", next_start: "2026-10-23T14:00", ready_at: "2026-10-23T14:30", overrun_minutes: 30 } }),
    );
    open();
    const d = await dialog();

    expect(d.getByText(/Esta saída pode atrasar a chegada da reserva RES-000009 \(≈ 30 min\)/)).toBeInTheDocument();
  });

  it("erro do backend mantém o diálogo aberto com a mensagem", async () => {
    vi.mocked(api.checkoutStay).mockRejectedValue({ code: "conflict", message: "O registro foi alterado por outra operação. Revise e tente novamente." });
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Concluir check-out" }));

    expect(await d.findByText("O registro foi alterado por outra operação. Revise e tente novamente.")).toBeInTheDocument();
    expect(onDone).not.toHaveBeenCalled();
    await waitFor(() => expect(api.previewStayCheckout).toHaveBeenCalledTimes(2));
  });
});
