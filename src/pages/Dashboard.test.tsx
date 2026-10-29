import { screen } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import Dashboard from "./Dashboard";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, cleaningQueue: vi.fn() };
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

beforeEach(() => signIn());

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
});
