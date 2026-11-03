import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import MaintenancePage from "./MaintenancePage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listMaintenance: vi.fn(), openMaintenance: vi.fn(), listRooms: vi.fn() };
});

const order: api.MaintenanceOrder = {
  id: 2, code: "MAN-000002", room_id: 1, room_code: "101", description: "Trocar o chuveiro", priority: "high", status: "open",
  planned_start: "2026-11-10", planned_end: "2026-11-12", actual_start: null, actual_end: null, responsible: "Seu João", notes: "", version: 1,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.listMaintenance).mockReset().mockResolvedValue([order]);
  vi.mocked(api.openMaintenance).mockReset().mockResolvedValue({ id: 9, code: "MAN-000009", conflicts: [] });
  vi.mocked(api.listRooms).mockReset().mockResolvedValue({
    items: [{ id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, cleaning_interval_minutes: null, version: 1 }],
    total: 1, page: 1, page_size: 100,
  });
});

const routes = [{ path: "/manutencao/:id", element: <p>Tela da ordem</p> }];

describe("MaintenancePage", () => {
  it("lista as ordens com situação em texto e período em DD/MM/AAAA", async () => {
    renderPage(<MaintenancePage />, { extraRoutes: routes });

    const row = await screen.findByRole("row", { name: /MAN-000002/ }, { timeout: 5000 });
    expect(within(row).getByText(/Aberta/)).toBeInTheDocument();
    expect(within(row).getByText("10/11/2026 a 12/11/2026")).toBeInTheDocument();
    expect(within(row).getByText("Trocar o chuveiro")).toBeInTheDocument();
  });

  it("o diálogo valida descrição, quarto e datas antes de chamar o backend", async () => {
    renderPage(<MaintenancePage />, { extraRoutes: routes });

    await userEvent.click(await screen.findByRole("button", { name: "Abrir ordem de manutenção" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Abrir ordem de manutenção" }));
    expect(await dialog.findByText("Escolha o quarto.")).toBeInTheDocument();
    expect(dialog.getByText("Descreva o que precisa ser feito.")).toBeInTheDocument();
    expect(dialog.getByText("Informe o início previsto.")).toBeInTheDocument();

    await userEvent.selectOptions(await dialog.findByLabelText(/Quarto/), "1");
    await userEvent.type(dialog.getByLabelText(/Descrição/), "Trocar o chuveiro");
    await userEvent.type(dialog.getByLabelText(/Início previsto/), "2026-11-10");
    await userEvent.type(dialog.getByLabelText(/Fim previsto/), "2026-11-09");
    await userEvent.click(dialog.getByRole("button", { name: "Abrir ordem de manutenção" }));
    expect(await dialog.findByText("O fim deve ser depois do início.")).toBeInTheDocument();
    expect(api.openMaintenance).not.toHaveBeenCalled();
  });

  it("erro do backend mantém o que foi digitado; depois de abrir, leva para a tela da ordem", async () => {
    vi.mocked(api.openMaintenance).mockRejectedValueOnce(new api.ApiError("validation", "O quarto 101 já tem um bloqueio nesse período."));
    renderPage(<MaintenancePage />, { extraRoutes: routes });

    await userEvent.click(await screen.findByRole("button", { name: "Abrir ordem de manutenção" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.selectOptions(await dialog.findByLabelText(/Quarto/), "1");
    await userEvent.type(dialog.getByLabelText(/Descrição/), "Trocar o chuveiro");
    await userEvent.type(dialog.getByLabelText(/Início previsto/), "2026-11-10");
    await userEvent.click(dialog.getByRole("button", { name: "Abrir ordem de manutenção" }));

    expect(await dialog.findByRole("alert")).toHaveTextContent("já tem um bloqueio");
    expect(dialog.getByLabelText(/Descrição/)).toHaveValue("Trocar o chuveiro");

    await userEvent.click(dialog.getByRole("button", { name: "Abrir ordem de manutenção" }));

    expect(await screen.findByText("Tela da ordem")).toBeInTheDocument();
    expect(api.openMaintenance).toHaveBeenLastCalledWith("t", {
      room_id: 1, description: "Trocar o chuveiro", priority: "normal", planned_start: "2026-11-10", planned_end: null, responsible: "", notes: "",
    });
  });
});
