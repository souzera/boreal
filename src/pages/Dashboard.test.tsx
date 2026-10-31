import { screen } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import Dashboard from "./Dashboard";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, cleaningQueue: vi.fn(), closingPendingDates: vi.fn() };
});

function task(id: number): api.CleaningItem {
  return {
    task: {
      id, room_id: id, room_code: `10${id}`, stay_id: id, status: "awaiting", ready_since: "2026-10-23T15:00:00Z",
      priority_override: null, priority_justification: null, started_at: null, started_by: null, completed_at: null,
      completed_by: null, version: 1,
    },
    priority: "low",
    suggested_priority: "low",
    next_arrival: null,
  };
}

beforeEach(() => {
  signIn();
  vi.mocked(api.closingPendingDates).mockReset().mockResolvedValue([]);
});

describe("Dashboard", () => {
  it("mostra quantos quartos aguardam limpeza, com atalho para a fila", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([task(1), task(2)]);
    renderPage(<Dashboard />);

    const link = await screen.findByRole("link", { name: "Quartos aguardando limpeza: 2" });
    expect(link).toHaveAttribute("href", "/limpeza");
  });

  it("sem quartos para limpar, diz que não há pendência", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    renderPage(<Dashboard />);

    expect(await screen.findByText("Nenhum quarto aguardando limpeza.")).toBeInTheDocument();
  });

  it("avisa dos dias com movimento e sem fechamento, com atalho para a conferência", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    vi.mocked(api.closingPendingDates).mockResolvedValue(["2026-10-19", "2026-10-20"]);
    renderPage(<Dashboard />);

    const first = await screen.findByRole("link", { name: "Fechamento pendente: 19/10/2026" });
    expect(first).toHaveAttribute("href", "/fechamento/2026-10-19");
    expect(screen.getByRole("link", { name: "Fechamento pendente: 20/10/2026" })).toHaveAttribute("href", "/fechamento/2026-10-20");
  });

  it("sem dias pendentes não mostra aviso de fechamento", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    renderPage(<Dashboard />);
    await screen.findByText("Nenhum quarto aguardando limpeza.");

    expect(screen.queryByText(/Fechamento pendente/)).toBeNull();
  });
});
