import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import AlertsPage from "./AlertsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, refreshAlerts: vi.fn(), listAlerts: vi.fn(), dismissAlert: vi.fn() };
});

function alert(patch: Partial<api.AlertView> & Pick<api.AlertView, "id" | "kind" | "severity">): api.AlertView {
  return {
    kind_label: "Tipo", severity_label: "Alto", title: `Alerta ${patch.id}`, detail: "Detalhe do alerta", link: `/reservas/${patch.id}`,
    status: "active", first_seen_at: "2026-10-21T15:00:00Z", last_seen_at: "2026-10-21T15:00:00Z", resolved_at: null,
    resolved_by_name: null, resolution_note: null, version: 1, ...patch,
  };
}

const active = [
  alert({ id: 1, kind: "departure_risk", severity: "critical", severity_label: "Crítico", kind_label: "Risco para a próxima reserva", title: "Risco para a próxima reserva: quarto 101", link: "/hospedagens/7" }),
  alert({ id: 2, kind: "deposit_overdue", severity: "high", severity_label: "Alto", kind_label: "Sinal vencido", title: "Sinal vencido: Ana Souza, quarto 102", version: 3 }),
  alert({ id: 3, kind: "arrival_today", severity: "normal", severity_label: "Normal", kind_label: "Chegada hoje", title: "Chegada hoje: Bruno Lima, quarto 103" }),
];
const history = [
  alert({ id: 9, kind: "deposit_overdue", severity: "high", status: "resolved", resolved_at: "2026-10-20T18:00:00Z", title: "Sinal vencido: Carla Menezes, quarto 104" }),
  alert({ id: 8, kind: "closing_pending", severity: "normal", status: "dismissed", resolved_at: "2026-10-20T19:00:00Z", resolved_by_name: "Administrador", resolution_note: "Vou fechar amanhã", title: "Fechamento de caixa pendente: 19/10/2026" }),
];

const page = (items: api.AlertView[]) => ({ items, total: items.length, page: 1, page_size: 20 });

function open() {
  renderPage(<AlertsPage />);
}

beforeEach(() => {
  signIn();
  vi.mocked(api.refreshAlerts).mockReset().mockResolvedValue(null);
  vi.mocked(api.dismissAlert).mockReset().mockResolvedValue(null);
  vi.mocked(api.listAlerts).mockReset().mockImplementation(async (_t, q) => page(q.status === "history" ? history : active));
});

describe("AlertsPage", () => {
  it("lista os alertas ativos com prioridade em rótulo e ícone, o crítico primeiro", async () => {
    open();

    const items = await screen.findAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Risco para a próxima reserva: quarto 101");
    expect(items[0].querySelector('[data-icon="error"]')).not.toBeNull();
    expect(within(items[0]).getByText("Crítico")).toBeInTheDocument();
    expect(items[1].querySelector('[data-icon="warning"]')).not.toBeNull();
    expect(items[2].querySelector('[data-icon="info"]')).not.toBeNull();
    expect(screen.getAllByText("Detalhe do alerta", { selector: "p" })).toHaveLength(3);
  });

  it("Abrir registro leva à origem do alerta", async () => {
    open();

    const links = await screen.findAllByRole("link", { name: "Abrir registro" });

    expect(links[0]).toHaveAttribute("href", "/hospedagens/7");
    expect(links[1]).toHaveAttribute("href", "/reservas/2");
  });

  it("alerta crítico não oferece Dispensar alerta e explica por quê", async () => {
    open();
    const critical = (await screen.findAllByRole("listitem"))[0];

    expect(within(critical).queryByRole("button", { name: "Dispensar alerta" })).toBeNull();
    expect(within(critical).getByText(/só se resolve quando o problema for resolvido/i)).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Dispensar alerta" })).toHaveLength(2);
  });

  it("dispensar exige justificativa e envia a versão vista", async () => {
    open();
    const second = (await screen.findAllByRole("listitem"))[1];
    await userEvent.click(within(second).getByRole("button", { name: "Dispensar alerta" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Dispensar alerta" }));
    expect(await dialog.findByText("Explique por que o alerta está sendo dispensado.")).toBeInTheDocument();
    expect(api.dismissAlert).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Hóspede vai pagar na chegada");
    await userEvent.click(dialog.getByRole("button", { name: "Dispensar alerta" }));

    await waitFor(() => expect(api.dismissAlert).toHaveBeenCalledWith("t", 2, "Hóspede vai pagar na chegada", 3));
    expect(await screen.findByText("Alerta dispensado e salvo neste computador.")).toBeInTheDocument();
  });

  it("erro ao dispensar mantém o texto digitado", async () => {
    vi.mocked(api.dismissAlert).mockRejectedValue(new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."));
    open();
    const second = (await screen.findAllByRole("listitem"))[1];
    await userEvent.click(within(second).getByRole("button", { name: "Dispensar alerta" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Justificativa/), "Depois eu vejo");
    await userEvent.click(dialog.getByRole("button", { name: "Dispensar alerta" }));

    expect(await dialog.findByText(/O registro foi alterado por outra operação/)).toBeInTheDocument();
    expect(dialog.getByLabelText(/Justificativa/)).toHaveValue("Depois eu vejo");
  });

  it("o histórico mostra quem resolveu, quando e por quê", async () => {
    open();
    await screen.findAllByRole("listitem");

    await userEvent.click(screen.getByRole("tab", { name: "Histórico" }));

    expect(await screen.findByText(/Resolvido automaticamente/)).toBeInTheDocument();
    const dismissed = screen.getByText("Fechamento de caixa pendente: 19/10/2026").closest("li") as HTMLElement;
    expect(dismissed).toHaveTextContent("Dispensado por Administrador");
    expect(dismissed).toHaveTextContent("Vou fechar amanhã");
    expect(within(dismissed).queryByRole("button", { name: "Dispensar alerta" })).toBeNull();
    await waitFor(() => expect(api.listAlerts).toHaveBeenLastCalledWith("t", expect.objectContaining({ status: "history" })));
  });

  it("filtra por tipo e por prioridade consultando o backend", async () => {
    open();
    await screen.findAllByRole("listitem");

    await userEvent.selectOptions(screen.getByLabelText("Tipo"), "deposit_overdue");
    await userEvent.selectOptions(screen.getByLabelText("Prioridade"), "high");

    await waitFor(() =>
      expect(api.listAlerts).toHaveBeenLastCalledWith("t", expect.objectContaining({ status: "active", kind: "deposit_overdue", severity: "high", page: 1 })),
    );
  });

  it("sem alertas, explica e diz que está tudo em ordem", async () => {
    vi.mocked(api.listAlerts).mockResolvedValue(page([]));
    open();

    expect(await screen.findByText("Nenhum alerta ativo. Está tudo em ordem por enquanto.")).toBeInTheDocument();
  });

  it("atualiza os alertas ao abrir a tela", async () => {
    open();
    await screen.findAllByRole("listitem");

    expect(api.refreshAlerts).toHaveBeenCalledWith("t");
  });
});
