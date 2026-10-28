import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import BlocksPage from "./BlocksPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listBlocks: vi.fn(), createBlock: vi.fn(), releaseBlock: vi.fn(), listRooms: vi.fn(), myPermissions: vi.fn() };
});

const block: api.RoomBlock = {
  id: 4, room_id: 1, room_code: "101", kind: "manual", status: "active", start_date: "2026-11-10", end_date: "2026-11-13",
  reason: "Pintura", maintenance_order_id: null, released_at: null, release_justification: null, version: 1,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue(["release_room"]);
  vi.mocked(api.listBlocks).mockReset().mockResolvedValue([block]);
  vi.mocked(api.createBlock).mockReset().mockResolvedValue(9);
  vi.mocked(api.releaseBlock).mockReset().mockResolvedValue(null);
  vi.mocked(api.listRooms).mockReset().mockResolvedValue({
    items: [{ id: 1, code: "101", category_id: 1, category_name: "Standard", max_capacity: 2, description: "", notes: "", active: true, cleaning_interval_minutes: null, version: 1 }],
    total: 1, page: 1, page_size: 100,
  });
});

describe("BlocksPage", () => {
  it("lista o bloqueio com período em DD/MM/AAAA e situação em texto", async () => {
    renderPage(<BlocksPage />);

    const row = await screen.findByRole("row", { name: /101/ });
    expect(within(row).getByText("10/11/2026 a 13/11/2026")).toBeInTheDocument();
    expect(within(row).getByText(/Bloqueado/)).toBeInTheDocument();
    expect(within(row).getByText("Pintura")).toBeInTheDocument();
  });

  it("sem permissão, não oferece Criar bloqueio nem Liberar quarto e explica", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue([]);
    renderPage(<BlocksPage />);

    await screen.findByRole("row", { name: /101/ });
    expect(screen.queryByRole("button", { name: "Criar bloqueio" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Liberar quarto" })).not.toBeInTheDocument();
    expect(screen.getByText(/precisa da permissão de liberar quartos/i)).toBeInTheDocument();
  });

  it("recusa do backend mostra o motivo e mantém os valores digitados", async () => {
    vi.mocked(api.createBlock).mockRejectedValue(new api.ApiError("validation", "O quarto já está reservado nesse período: RES-000001 (Ana, 12/11/2026 a 14/11/2026)."));
    renderPage(<BlocksPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Criar bloqueio" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.selectOptions(await dialog.findByLabelText(/Quarto/), "1");
    await userEvent.type(dialog.getByLabelText(/Início/), "2026-11-12");
    await userEvent.type(dialog.getByLabelText(/Motivo/), "Pintura do teto");
    await userEvent.click(dialog.getByRole("button", { name: "Criar bloqueio" }));

    expect(await dialog.findByRole("alert")).toHaveTextContent("RES-000001");
    expect(dialog.getByLabelText(/Motivo/)).toHaveValue("Pintura do teto");
    expect(api.createBlock).toHaveBeenCalledWith("t", { room_id: 1, start_date: "2026-11-12", end_date: null, reason: "Pintura do teto" });
  });

  it("Criar bloqueio valida motivo e início antes de chamar o backend", async () => {
    renderPage(<BlocksPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Criar bloqueio" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Criar bloqueio" }));

    expect(await dialog.findByText("Escolha o quarto.")).toBeInTheDocument();
    expect(dialog.getByText("Informe a data de início.")).toBeInTheDocument();
    expect(dialog.getByText("Informe o motivo do bloqueio.")).toBeInTheDocument();
    expect(api.createBlock).not.toHaveBeenCalled();
  });

  it("Liberar quarto exige justificativa e usa a versão do bloqueio", async () => {
    renderPage(<BlocksPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Liberar quarto" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Liberar quarto" }));
    expect(await dialog.findByText("Informe a justificativa da liberação.")).toBeInTheDocument();
    expect(api.releaseBlock).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Obra terminou");
    await userEvent.click(dialog.getByRole("button", { name: "Liberar quarto" }));

    await waitFor(() => expect(api.releaseBlock).toHaveBeenCalledWith("t", 4, "Obra terminou", 1));
    expect(await screen.findByText("Quarto 101 liberado.")).toBeInTheDocument();
  });
});
