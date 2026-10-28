import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import CleaningQueuePage from "./CleaningQueuePage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    cleaningQueue: vi.fn(),
    startCleaning: vi.fn(),
    completeCleaning: vi.fn(),
    setCleaningPriority: vi.fn(),
    listRoomStatus: vi.fn(),
  };
});

function item(patch: Partial<api.CleaningTask> = {}, rest: Partial<api.CleaningItem> = {}): api.CleaningItem {
  return {
    task: {
      id: 7, room_id: 1, room_code: "101", stay_id: 3, status: "awaiting", ready_since: "2026-10-23T15:00:00Z",
      priority_override: null, priority_justification: null, started_at: null, started_by: null, completed_at: null,
      completed_by: null, version: 1, ...patch,
    },
    priority: "urgent",
    suggested_priority: "urgent",
    next_arrival: "2026-10-23T14:00",
    ...rest,
  };
}

beforeEach(() => {
  signIn();
  vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([item()]);
  vi.mocked(api.startCleaning).mockReset().mockResolvedValue(null);
  vi.mocked(api.completeCleaning).mockReset().mockResolvedValue(null);
  vi.mocked(api.setCleaningPriority).mockReset().mockResolvedValue(null);
  vi.mocked(api.listRoomStatus).mockReset().mockResolvedValue([]);
});

describe("CleaningQueuePage", () => {
  it("mostra a prioridade por rótulo e símbolo, não só por cor", async () => {
    renderPage(<CleaningQueuePage />);

    const row = await screen.findByRole("row", { name: /101/ });
    expect(within(row).getByText(/Urgente/)).toBeInTheDocument();
    expect(within(row).getByText(/Aguardando limpeza/)).toBeInTheDocument();
    expect(within(row).getByText("23/10/2026 14:00")).toBeInTheDocument();
  });

  it("tarefa aguardando oferece Iniciar limpeza e em andamento oferece Concluir limpeza", async () => {
    renderPage(<CleaningQueuePage />);
    expect(await screen.findByRole("button", { name: "Iniciar limpeza" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Concluir limpeza" })).not.toBeInTheDocument();

    vi.mocked(api.cleaningQueue).mockResolvedValue([item({ status: "in_progress", version: 2 })]);
    await userEvent.click(screen.getByRole("button", { name: "Iniciar limpeza" }));

    expect(api.startCleaning).toHaveBeenCalledWith("t", 7, 1);
    expect(await screen.findByRole("button", { name: "Concluir limpeza" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Iniciar limpeza" })).not.toBeInTheDocument();
  });

  it("concluir usa a versão da tarefa e mostra o resultado com a hora em que o quarto fica livre", async () => {
    vi.mocked(api.cleaningQueue).mockResolvedValueOnce([item({ status: "in_progress", version: 2 })]).mockResolvedValue([]);
    vi.mocked(api.listRoomStatus).mockResolvedValue([
      { room_id: 1, code: "101", category_name: "Standard", state: "clean_waiting_interval", label_pt: "Limpo — aguardando intervalo", free_at: "2026-10-23T14:00", cleaning_task_id: null },
    ]);
    renderPage(<CleaningQueuePage />);

    await userEvent.click(await screen.findByRole("button", { name: "Concluir limpeza" }));

    expect(api.completeCleaning).toHaveBeenCalledWith("t", 7, 2);
    expect(await screen.findByText("Limpeza do quarto 101 concluída. O quarto fica livre às 14:00.")).toBeInTheDocument();
  });

  it("conflito recarrega a fila e explica", async () => {
    vi.mocked(api.startCleaning).mockRejectedValue(new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."));
    renderPage(<CleaningQueuePage />);

    await userEvent.click(await screen.findByRole("button", { name: "Iniciar limpeza" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("alterado por outra operação");
    await waitFor(() => expect(api.cleaningQueue).toHaveBeenCalledTimes(2));
  });

  it("fila vazia diz que não há o que limpar", async () => {
    vi.mocked(api.cleaningQueue).mockResolvedValue([]);
    renderPage(<CleaningQueuePage />);

    expect(await screen.findByText("Nenhum quarto aguardando limpeza.")).toBeInTheDocument();
  });

  it("ajuste de prioridade exige justificativa e mantém o texto quando a gravação falha", async () => {
    vi.mocked(api.setCleaningPriority).mockRejectedValueOnce(new api.ApiError("validation", "Não foi possível ajustar."));
    renderPage(<CleaningQueuePage />);

    await userEvent.click(await screen.findByRole("button", { name: "Ajustar prioridade" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.selectOptions(dialog.getByLabelText("Nova prioridade"), "low");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar prioridade" }));
    expect(await dialog.findByText("Informe a justificativa do ajuste.")).toBeInTheDocument();
    expect(api.setCleaningPriority).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Justificativa do ajuste/), "Hóspede só chega amanhã");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar prioridade" }));

    expect(await dialog.findByRole("alert")).toHaveTextContent("Não foi possível ajustar.");
    expect(dialog.getByLabelText(/Justificativa do ajuste/)).toHaveValue("Hóspede só chega amanhã");
    expect(api.setCleaningPriority).toHaveBeenCalledWith("t", 7, "low", "Hóspede só chega amanhã", 1);
  });
});
