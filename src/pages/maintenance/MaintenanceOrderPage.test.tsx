import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import MaintenanceOrderPage from "./MaintenanceOrderPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    getMaintenance: vi.fn(),
    applyMaintenanceUnavailability: vi.fn(),
    resolveMaintenanceConflict: vi.fn(),
    startMaintenance: vi.fn(),
    requestMaintenanceResolution: vi.fn(),
    completeMaintenance: vi.fn(),
  };
});

const order: api.MaintenanceOrder = {
  id: 2, code: "MAN-000002", room_id: 1, room_code: "101", description: "Trocar o chuveiro", priority: "high", status: "open",
  planned_start: "2026-11-10", planned_end: "2026-11-12", actual_start: null, actual_end: null, responsible: "Seu João", notes: "", version: 1,
};
const conflict: api.MaintenanceConflict = {
  id: 5, order_id: 2, reservation_id: 8, reservation_code: "RES-000008", guest_name: "Ana Souza", check_in_date: "2026-11-09",
  check_out_date: "2026-11-11", reservation_status: "confirmed", status: "open", resolution_note: null,
};
const block: api.RoomBlock = {
  id: 4, room_id: 1, room_code: "101", kind: "maintenance", status: "preventive", start_date: "2026-11-10", end_date: "2026-11-12",
  reason: "Manutenção MAN-000002: Trocar o chuveiro", maintenance_order_id: 2, released_at: null, release_justification: null, version: 1,
};

function detail(patch: Partial<api.OrderDetail> = {}): api.OrderDetail {
  return { order, conflicts: [conflict], block, ...patch };
}

const path = "/manutencao/:id";
const entry = "/manutencao/2";

beforeEach(() => {
  signIn();
  vi.mocked(api.getMaintenance).mockReset().mockResolvedValue(detail());
  for (const fn of [api.resolveMaintenanceConflict, api.startMaintenance, api.requestMaintenanceResolution, api.completeMaintenance]) {
    vi.mocked(fn).mockReset().mockResolvedValue(null);
  }
  vi.mocked(api.applyMaintenanceUnavailability).mockReset().mockResolvedValue({ still_open: [conflict], effective: false });
});

describe("MaintenanceOrderPage", () => {
  it("lista as reservas afetadas e não deixa iniciar enquanto houver conflito aberto", async () => {
    renderPage(<MaintenanceOrderPage />, { path, entry });

    const row = await screen.findByRole("row", { name: /RES-000008/ });
    expect(within(row).getByText("Ana Souza")).toBeInTheDocument();
    expect(within(row).getByText("09/11/2026 a 11/11/2026")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Iniciar manutenção" })).toBeDisabled();
    expect(screen.getByText(/Resolva as reservas afetadas antes de iniciar/)).toBeInTheDocument();
    expect(screen.getByText(/Bloqueio preventivo/)).toBeInTheDocument();
  });

  it("Aplicar indisponibilidade chama o backend com a versão da ordem e mostra o que falta", async () => {
    renderPage(<MaintenanceOrderPage />, { path, entry });

    await userEvent.click(await screen.findByRole("button", { name: "Aplicar indisponibilidade" }));

    expect(api.applyMaintenanceUnavailability).toHaveBeenCalledWith("t", 2, 1);
    expect(await screen.findByText(/Ainda há 1 reserva afetada/)).toBeInTheDocument();
  });

  it("resolução manual exige a nota", async () => {
    renderPage(<MaintenanceOrderPage />, { path, entry });

    await userEvent.click(await screen.findByRole("button", { name: "Registrar resolução" }));
    await userEvent.click(await screen.findByRole("button", { name: "Registrar resolução" }));
    expect(await screen.findByText("Informe como a reserva foi resolvida.")).toBeInTheDocument();
    expect(api.resolveMaintenanceConflict).not.toHaveBeenCalled();

    await userEvent.type(screen.getByLabelText(/Como foi resolvido/), "Hóspede foi para o 102");
    await userEvent.click(screen.getByRole("button", { name: "Registrar resolução" }));

    await waitFor(() => expect(api.resolveMaintenanceConflict).toHaveBeenCalledWith("t", 5, "Hóspede foi para o 102"));
  });

  it("sem conflito aberto, Iniciar manutenção usa a versão da ordem", async () => {
    vi.mocked(api.getMaintenance).mockResolvedValue(
      detail({ conflicts: [{ ...conflict, status: "resolved", resolution_note: "ok" }], block: { ...block, status: "active" } }),
    );
    renderPage(<MaintenanceOrderPage />, { path, entry });

    await userEvent.click(await screen.findByRole("button", { name: "Iniciar manutenção" }));

    expect(api.startMaintenance).toHaveBeenCalledWith("t", 2, 1);
  });

  it("depois de concluir, avisa que o quarto continua bloqueado até ser liberado", async () => {
    vi.mocked(api.getMaintenance).mockResolvedValue(
      detail({
        order: { ...order, status: "completed", actual_start: "2026-11-10T12:00:00Z", actual_end: "2026-11-11T12:00:00Z", version: 4 },
        conflicts: [],
        block: { ...block, status: "awaiting_release" },
      }),
    );
    renderPage(<MaintenanceOrderPage />, { path, entry });

    expect(await screen.findByText(/continua bloqueado até ser liberado/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Ir para os bloqueios" })).toHaveAttribute("href", "/bloqueios");
    expect(screen.queryByRole("button", { name: "Concluir manutenção" })).not.toBeInTheDocument();
  });

  it("em resolução, oferece Concluir manutenção e a recusa do backend aparece na tela", async () => {
    vi.mocked(api.getMaintenance).mockResolvedValue(
      detail({ order: { ...order, status: "awaiting_resolution", version: 3 }, conflicts: [], block: { ...block, status: "active" } }),
    );
    vi.mocked(api.completeMaintenance).mockRejectedValue(
      new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."),
    );
    renderPage(<MaintenanceOrderPage />, { path, entry });

    await userEvent.click(await screen.findByRole("button", { name: "Concluir manutenção" }));

    expect(api.completeMaintenance).toHaveBeenCalledWith("t", 2, 3);
    expect(await screen.findByRole("alert")).toHaveTextContent("alterado por outra operação");
  });
});
