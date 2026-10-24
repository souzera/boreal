import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ReservationPage from "./ReservationPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    getReservation: vi.fn(),
    listRooms: vi.fn(),
    myPermissions: vi.fn(),
    previewReservationChange: vi.fn(),
    applyReservationChange: vi.fn(),
    previewReservationCancellation: vi.fn(),
    cancelReservation: vi.fn(),
    markReservationNoShow: vi.fn(),
    applyReservationDiscount: vi.fn(),
    confirmReservation: vi.fn(),
    recordReservationDeposit: vi.fn(),
    setReservationNotes: vi.fn(),
  };
});

const reservation: api.Reservation = {
  id: 5, code: "RES-000005", room_id: 1, room_code: "101", guest_id: 7, guest_name: "Ana Souza",
  check_in_date: "2099-10-25", check_out_date: "2099-10-28", expected_check_in_time: "14:00",
  expected_check_out_time: "12:00", nights: 3, occupants: 2, status: "pending", list_total_cents: 30000,
  discount_cents: 3000, discount_reason: "Cliente antigo", agreed_total_cents: 27000, deposit_expected_cents: 8100,
  deposit_paid_cents: 4000, deposit_due_date: "2099-10-24", deposit_overdue: false, notes: "Chegada tarde",
  cancelled_at: null, cancellation_fee_cents: null, version: 3, created_at: "2099-10-01T12:00:00Z", origin: "booking",
};
const detail: api.ReservationDetail = {
  reservation,
  stay_id: null,
  nights: [
    { date: "2099-10-25", amount_cents: 10000, rule: "weekday" },
    { date: "2099-10-26", amount_cents: 10000, rule: "weekday" },
    { date: "2099-10-27", amount_cents: 10000, rule: "season" },
  ],
  history: [
    {
      id: 1, changed_at: "2099-10-02T15:00:00Z", changed_by: 1, changed_by_name: "Administrador", kind: "discount",
      before_json: "{}", after_json: "{}", price_before_cents: 30000, price_after_cents: 27000,
      justification: "Cliente antigo",
    },
  ],
};
const ALL = ["change_reservation", "apply_discount", "cancellation_exception", "exceed_capacity"];
const routes = [{ path: "/reservas", element: <p>Lista de reservas</p> }];

function setDetail(patch: Partial<api.Reservation> = {}) {
  vi.mocked(api.getReservation).mockResolvedValue({ ...detail, reservation: { ...reservation, ...patch } });
}

beforeEach(() => {
  signIn();
  vi.mocked(api.getReservation).mockReset();
  setDetail();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue(ALL);
  vi.mocked(api.listRooms).mockResolvedValue({
    items: [
      { id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, version: 1 },
      { id: 2, code: "201", category_id: 2, category_name: "Luxo", max_capacity: 3, description: "", notes: "", active: true, version: 1 },
    ],
    total: 2, page: 1, page_size: 100,
  });
  for (const fn of [
    api.applyReservationChange, api.cancelReservation, api.markReservationNoShow, api.applyReservationDiscount,
    api.confirmReservation, api.recordReservationDeposit, api.setReservationNotes, api.previewReservationChange,
    api.previewReservationCancellation,
  ]) {
    vi.mocked(fn as (...a: never[]) => Promise<unknown>).mockReset();
  }
  vi.mocked(api.applyReservationChange).mockResolvedValue(null);
  vi.mocked(api.cancelReservation).mockResolvedValue({ refund_cents: 2000, retained_cents: 2000 });
  vi.mocked(api.markReservationNoShow).mockResolvedValue(null);
  vi.mocked(api.applyReservationDiscount).mockResolvedValue(null);
  vi.mocked(api.confirmReservation).mockResolvedValue(null);
  vi.mocked(api.recordReservationDeposit).mockResolvedValue(null);
});

async function open(state?: unknown) {
  const view = renderPage(<ReservationPage />, {
    path: "/reservas/:id",
    entry: { pathname: "/reservas/5", state },
    extraRoutes: routes,
  });
  await screen.findByRole("heading", { name: /RES-000005/ });
  return view;
}

const preview = (patch: Partial<api.ChangePreview> = {}): api.ChangePreview => ({
  feasible: true,
  fingerprint: "abc123",
  price_before_cents: 27000,
  price_after_cents: 37000,
  difference_cents: 10000,
  new_deposit_expected_cents: 11100,
  nights: [
    { date: "2099-10-25", amount_cents: 10000, kept: true, rule: "weekday" },
    { date: "2099-10-26", amount_cents: 10000, kept: true, rule: "weekday" },
    { date: "2099-10-27", amount_cents: 10000, kept: true, rule: "season" },
    { date: "2099-10-28", amount_cents: 12000, kept: false, rule: "weekday" },
  ],
  missing_dates: [],
  conflicts: [],
  problems: [],
  ...patch,
});

describe("ReservationPage — dados", () => {
  it("mostra dados, valores, sinal, noites e histórico", async () => {
    await open();

    expect(screen.getByText(/Pendente/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Ana Souza" })).toHaveAttribute("href", "/hospedes/7");
    expect(screen.getByText(/Quarto 101/)).toBeInTheDocument();
    expect(screen.getByText("25/10/2099 a 28/10/2099 (3 noites)")).toBeInTheDocument();
    expect(screen.getByText("R$ 300,00")).toBeInTheDocument(); // tabela
    expect(screen.getByText("− R$ 30,00")).toBeInTheDocument(); // desconto
    expect(screen.getByText("R$ 270,00")).toBeInTheDocument(); // combinado
    expect(screen.getByText(/Sinal esperado: R\$ 81,00/)).toBeInTheDocument();
    expect(screen.getByText(/Sinal recebido: R\$ 40,00/)).toBeInTheDocument();
    expect(screen.getByText(/Prazo do sinal: 24\/10\/2099/)).toBeInTheDocument();
    expect(screen.getByText("Temporada")).toBeInTheDocument();
    expect(screen.getByText("Chegada tarde")).toBeInTheDocument();
    expect(screen.getByText(/Administrador/)).toBeInTheDocument();
    expect(screen.getAllByText("Cliente antigo").length).toBeGreaterThan(0);
  });

  it("mostra a mensagem de resultado recebida da criação", async () => {
    await open({ notice: "Reserva RES-000005 criada." });

    expect(screen.getByText("Reserva RES-000005 criada.")).toBeInTheDocument();
  });

  it("sinal vencido aparece como alerta sem mudar a reserva", async () => {
    setDetail({ deposit_overdue: true });
    await open();

    expect(screen.getByText("Sinal vencido")).toBeInTheDocument();
    expect(screen.getByText(/Pendente/)).toBeInTheDocument();
  });

  it("ações dependem da situação e das permissões", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue([]);
    await open();

    expect(screen.getByRole("button", { name: "Cancelar reserva" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Alterar datas ou quarto" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Aplicar desconto" })).toBeNull();
  });

  it("reserva cancelada não oferece ações", async () => {
    setDetail({ status: "cancelled" });
    await open();

    for (const name of ["Cancelar reserva", "Alterar datas ou quarto", "Aplicar desconto", "Confirmar reserva", "Registrar sinal recebido"]) {
      expect(screen.queryByRole("button", { name })).toBeNull();
    }
  });

  it("não oferece No-show antes da data de chegada; oferece quando a chegada já passou", async () => {
    await open();
    expect(screen.queryByRole("button", { name: "Marcar no-show" })).toBeNull();
  });
});

describe("ReservationPage — alterar datas ou quarto", () => {
  async function openChange() {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: "Alterar datas ou quarto" }));
    return within(await screen.findByRole("dialog"));
  }

  it("mostra preço anterior, novo e diferença antes de qualquer gravação", async () => {
    vi.mocked(api.previewReservationChange).mockResolvedValue(preview());
    const dialog = await openChange();

    await userEvent.clear(dialog.getByLabelText("Nova saída"));
    await userEvent.type(dialog.getByLabelText("Nova saída"), "2099-10-29");
    await userEvent.click(dialog.getByRole("button", { name: "Ver prévia" }));

    await waitFor(() =>
      expect(api.previewReservationChange).toHaveBeenCalledWith("t", 5, { room_id: null, check_in_date: null, check_out_date: "2099-10-29" }),
    );
    expect(await dialog.findByText("Preço anterior: R$ 270,00")).toBeInTheDocument();
    expect(dialog.getByText("Preço novo: R$ 370,00")).toBeInTheDocument();
    expect(dialog.getByText("Diferença: + R$ 100,00")).toBeInTheDocument();
    expect(dialog.getAllByText("Mantém o preço combinado").length).toBe(3);
    expect(dialog.getByText("Preço da tabela atual")).toBeInTheDocument();
    expect(api.applyReservationChange).not.toHaveBeenCalled();
  });

  it("só confirma com justificativa e envia versão e impressão da prévia", async () => {
    vi.mocked(api.previewReservationChange).mockResolvedValue(preview());
    const dialog = await openChange();
    await userEvent.clear(dialog.getByLabelText("Nova saída"));
    await userEvent.type(dialog.getByLabelText("Nova saída"), "2099-10-29");
    await userEvent.click(dialog.getByRole("button", { name: "Ver prévia" }));
    await dialog.findByText("Preço novo: R$ 370,00");

    expect(dialog.getByRole("button", { name: "Confirmar alteração" })).toBeDisabled();
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Hóspede pediu mais uma noite");
    await userEvent.click(dialog.getByRole("button", { name: "Confirmar alteração" }));

    await waitFor(() =>
      expect(api.applyReservationChange).toHaveBeenCalledWith(
        "t", 5, { room_id: null, check_in_date: null, check_out_date: "2099-10-29" }, 3,
        "Hóspede pediu mais uma noite", "abc123",
      ),
    );
    expect(await screen.findByText("Reserva alterada.")).toBeInTheDocument();
  });

  it("alteração inviável mostra o motivo e não permite confirmar", async () => {
    vi.mocked(api.previewReservationChange).mockResolvedValue(
      preview({
        feasible: false,
        conflicts: [{ reservation_id: 8, code: "RES-000008", guest_name: "Bruno Lima", check_in_date: "2099-10-29", check_out_date: "2099-10-31", status: "confirmed" }],
        missing_dates: ["2099-10-28"],
        problems: ["O quarto escolhido está desativado."],
      }),
    );
    const dialog = await openChange();
    await userEvent.click(dialog.getByRole("button", { name: "Ver prévia" }));

    expect(await dialog.findByText(/RES-000008/)).toBeInTheDocument();
    expect(dialog.getByText(/Faltam tarifas para: 28\/10\/2099/)).toBeInTheDocument();
    expect(dialog.getByText("O quarto escolhido está desativado.")).toBeInTheDocument();
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "x");
    expect(dialog.getByRole("button", { name: "Confirmar alteração" })).toBeDisabled();
  });

  it("prévia desatualizada: explica e refaz a prévia sem gravar", async () => {
    vi.mocked(api.previewReservationChange).mockResolvedValue(preview());
    vi.mocked(api.applyReservationChange).mockRejectedValue(new api.ApiError("stale_preview", "Os preços ou a disponibilidade mudaram desde a prévia. Revise a nova prévia antes de confirmar."));
    const dialog = await openChange();
    await userEvent.click(dialog.getByRole("button", { name: "Ver prévia" }));
    await dialog.findByText("Preço novo: R$ 370,00");
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Mais uma noite");

    await userEvent.click(dialog.getByRole("button", { name: "Confirmar alteração" }));

    expect(await dialog.findByText(/mudaram desde a prévia/)).toBeInTheDocument();
    await waitFor(() => expect(api.previewReservationChange).toHaveBeenCalledTimes(2));
    expect(dialog.getByLabelText(/Justificativa/)).toHaveValue("Mais uma noite");
  });

  it("trocar de quarto envia o quarto escolhido", async () => {
    vi.mocked(api.previewReservationChange).mockResolvedValue(preview({ nights: preview().nights.map((n) => ({ ...n, kept: false })) }));
    const dialog = await openChange();

    await userEvent.selectOptions(await dialog.findByLabelText("Novo quarto"), "2");
    await userEvent.click(dialog.getByRole("button", { name: "Ver prévia" }));

    await waitFor(() =>
      expect(api.previewReservationChange).toHaveBeenCalledWith("t", 5, { room_id: 2, check_in_date: null, check_out_date: null }),
    );
  });
});

describe("ReservationPage — cancelar", () => {
  const cancelPreview: api.CancellationPreview = {
    days_before: 4, deposit_paid_cents: 4000, refund_cents: 2000, retained_cents: 2000, tier_min_days: 3, can_cancel: true,
  };

  async function openCancel() {
    vi.mocked(api.previewReservationCancellation).mockResolvedValue(cancelPreview);
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Cancelar reserva" }));
    return within(await screen.findByRole("dialog"));
  }

  it("mostra o cálculo pela política antes de cancelar", async () => {
    const dialog = await openCancel();

    expect(await dialog.findByText("Faltam 4 dias para a chegada.")).toBeInTheDocument();
    expect(dialog.getByText("Sinal já pago: R$ 40,00")).toBeInTheDocument();
    expect(dialog.getByText("Devolução: R$ 20,00")).toBeInTheDocument();
    expect(dialog.getByText("Retenção: R$ 20,00")).toBeInTheDocument();
    expect(api.cancelReservation).not.toHaveBeenCalled();
  });

  it("confirma o cancelamento com os valores calculados", async () => {
    const dialog = await openCancel();
    await dialog.findByText("Devolução: R$ 20,00");

    await userEvent.click(dialog.getByRole("button", { name: "Confirmar cancelamento" }));

    await waitFor(() => expect(api.cancelReservation).toHaveBeenCalledWith("t", 5, 3, null));
    expect(await screen.findByText("Reserva cancelada. Devolução: R$ 20,00; retenção: R$ 20,00.")).toBeInTheDocument();
  });

  it("exceção só para quem tem a permissão, com justificativa e valor válido", async () => {
    const dialog = await openCancel();
    await dialog.findByText("Devolução: R$ 20,00");

    await userEvent.click(await dialog.findByLabelText("Aplicar exceção à política"));
    await userEvent.type(dialog.getByLabelText(/Valor retido/), "0");
    await userEvent.click(dialog.getByRole("button", { name: "Confirmar cancelamento" }));
    expect(await dialog.findByText("Informe a justificativa da exceção.")).toBeInTheDocument();
    expect(api.cancelReservation).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Motivo de saúde");
    await userEvent.click(dialog.getByRole("button", { name: "Confirmar cancelamento" }));

    await waitFor(() =>
      expect(api.cancelReservation).toHaveBeenCalledWith("t", 5, 3, { retained_cents: 0, justification: "Motivo de saúde" }),
    );
  });

  it("sem a permissão não oferece a exceção", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue([]);
    const dialog = await openCancel();
    await dialog.findByText("Devolução: R$ 20,00");

    expect(dialog.queryByLabelText("Aplicar exceção à política")).toBeNull();
  });

  it("valor retido acima do sinal pago é recusado na tela", async () => {
    const dialog = await openCancel();
    await dialog.findByText("Devolução: R$ 20,00");
    await userEvent.click(await dialog.findByLabelText("Aplicar exceção à política"));
    await userEvent.type(dialog.getByLabelText(/Valor retido/), "50,00");
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "x");

    await userEvent.click(dialog.getByRole("button", { name: "Confirmar cancelamento" }));

    expect(await dialog.findByText("O valor retido deve estar entre R$ 0,00 e o sinal pago (R$ 40,00).")).toBeInTheDocument();
    expect(api.cancelReservation).not.toHaveBeenCalled();
  });
});

describe("ReservationPage — desconto, confirmação, sinal e no-show", () => {
  it("aplica desconto em percentual com justificativa", async () => {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: "Aplicar desconto" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Aplicar" }));
    expect(await dialog.findByText("Informe o valor do desconto.")).toBeInTheDocument();
    expect(dialog.getByText("Informe a justificativa do desconto.")).toBeInTheDocument();

    await userEvent.click(dialog.getByLabelText("Em percentual"));
    await userEvent.type(dialog.getByLabelText(/Percentual do desconto/), "10");
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Cliente recorrente");
    await userEvent.click(dialog.getByRole("button", { name: "Aplicar" }));

    await waitFor(() =>
      expect(api.applyReservationDiscount).toHaveBeenCalledWith(
        "t", 5, { kind: "percent", amount_cents: null, percent_bp: 1000, justification: "Cliente recorrente" }, 3,
      ),
    );
  });

  it("aplica desconto em reais", async () => {
    await open();
    await userEvent.click(await screen.findByRole("button", { name: "Aplicar desconto" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Valor do desconto/), "25,50");
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Cortesia");
    await userEvent.click(dialog.getByRole("button", { name: "Aplicar" }));

    await waitFor(() =>
      expect(api.applyReservationDiscount).toHaveBeenCalledWith(
        "t", 5, { kind: "amount", amount_cents: 2550, percent_bp: null, justification: "Cortesia" }, 3,
      ),
    );
  });

  it("confirma a reserva pendente", async () => {
    await open();

    await userEvent.click(screen.getByRole("button", { name: "Confirmar reserva" }));

    await waitFor(() => expect(api.confirmReservation).toHaveBeenCalledWith("t", 5, 3));
    expect(await screen.findByText("Reserva confirmada.")).toBeInTheDocument();
  });

  it("reserva já confirmada não oferece confirmar de novo", async () => {
    setDetail({ status: "confirmed" });
    await open();

    expect(screen.queryByRole("button", { name: "Confirmar reserva" })).toBeNull();
  });

  it("registra o sinal recebido em reais", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Registrar sinal recebido" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Valor recebido/), "41,00");
    await userEvent.click(dialog.getByRole("button", { name: "Registrar" }));
    expect(api.recordReservationDeposit).not.toHaveBeenCalled();
    expect(await dialog.findByText("Registrar sinal de R$ 41,00? Depois de salvo, o valor não pode ser corrigido nesta tela.")).toBeInTheDocument();
    await userEvent.click(dialog.getByRole("button", { name: "Confirmar registro" }));

    await waitFor(() => expect(api.recordReservationDeposit).toHaveBeenCalledWith("t", 5, 4100, 3));
  });

  it("oferece no-show já no dia da chegada (o sistema aplica a tolerância)", async () => {
    const d = new Date();
    const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    setDetail({ check_in_date: iso, check_out_date: "2099-12-31" });
    await open();

    expect(await screen.findByRole("button", { name: "Marcar no-show" })).toBeInTheDocument();
  });

  it("marca no-show depois de confirmar, quando a chegada já passou", async () => {
    setDetail({ check_in_date: "2000-01-01", check_out_date: "2000-01-03" });
    await open();

    await userEvent.click(screen.getByRole("button", { name: "Marcar no-show" }));
    expect(api.markReservationNoShow).not.toHaveBeenCalled();
    await userEvent.click(await screen.findByRole("button", { name: "Confirmar no-show" }));

    await waitFor(() => expect(api.markReservationNoShow).toHaveBeenCalledWith("t", 5, 3));
  });

  it("erro do backend aparece na tela e a reserva é recarregada em conflito de versão", async () => {
    vi.mocked(api.confirmReservation).mockRejectedValue(new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."));
    await open();
    const before = vi.mocked(api.getReservation).mock.calls.length;

    await userEvent.click(screen.getByRole("button", { name: "Confirmar reserva" }));

    expect(await screen.findByText(/foi alterado por outra operação/)).toBeInTheDocument();
    await waitFor(() => expect(vi.mocked(api.getReservation).mock.calls.length).toBeGreaterThan(before));
  });
});
