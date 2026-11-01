import { screen } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import Dashboard from "./Dashboard";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, cleaningQueue: vi.fn(), refreshAlerts: vi.fn(), alertSummary: vi.fn() };
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
  vi.mocked(api.refreshAlerts).mockReset().mockResolvedValue(null);
  vi.mocked(api.alertSummary).mockReset().mockResolvedValue({ critical: 0, high: 0, normal: 0, total: 0, top: [] });
});

describe("Dashboard", () => {
  it("a tela inicial se chama Recepção", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    renderPage(<Dashboard />);

    expect(await screen.findByRole("heading", { name: "Recepção" })).toBeInTheDocument();
  });

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

  it("mostra os alertas mais urgentes com link para a origem", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    const alert = (id: number, severity: api.AlertSeverity, title: string, link: string): api.AlertView => ({
      id, kind: "deposit_overdue", kind_label: "Sinal vencido", severity, severity_label: "Alto", title, detail: "Detalhe", link, status: "active",
      first_seen_at: "2026-10-21T15:00:00Z", last_seen_at: "2026-10-21T15:00:00Z", resolved_at: null, resolved_by_name: null, resolution_note: null, version: 1,
    });
    vi.mocked(api.alertSummary).mockResolvedValue({
      critical: 1, high: 1, normal: 0, total: 2,
      top: [alert(1, "critical", "Risco para a próxima reserva: quarto 101", "/hospedagens/7"), alert(2, "high", "Sinal vencido: Ana Souza", "/reservas/5")],
    });
    renderPage(<Dashboard />);

    const first = await screen.findByRole("link", { name: /Risco para a próxima reserva: quarto 101/ });
    expect(first).toHaveAttribute("href", "/hospedagens/7");
    expect(screen.getByRole("link", { name: /Sinal vencido: Ana Souza/ })).toHaveAttribute("href", "/reservas/5");
    expect(screen.getByRole("link", { name: "Ver todos os alertas" })).toHaveAttribute("href", "/alertas");
  });

  it("sem alertas, diz que está tudo em ordem", async () => {
    vi.mocked(api.cleaningQueue).mockReset().mockResolvedValue([]);
    renderPage(<Dashboard />);

    expect(await screen.findByText("Nenhum alerta ativo. Está tudo em ordem.")).toBeInTheDocument();
  });
});
