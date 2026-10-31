import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { toLocalIsoDate } from "@/lib/format";
import { renderPage, signIn } from "@/test/render";
import ClosingPage from "./ClosingPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, closingPreview: vi.fn(), closeCash: vi.fn(), closingPendingDates: vi.fn() };
});

const TODAY = toLocalIsoDate(new Date());

function expectedLine(method: api.PaymentMethod, label: string, received: number, refunded = 0): api.ExpectedLineView {
  return { method, label, received_cents: received, refunded_cents: refunded, expected_cents: received - refunded };
}

const expected: api.ExpectedLineView[] = [
  expectedLine("cash", "Dinheiro", 10000),
  expectedLine("pix", "Pix", 8000, 2000),
  expectedLine("debit", "Cartão de débito", 5000),
  expectedLine("credit", "Cartão de crédito", 3000),
  expectedLine("transfer", "Transferência bancária", 0),
];

function preview(patch: Partial<api.ClosingPreview> = {}): api.ClosingPreview {
  return { date: TODAY, closed: null, expected, supplier_payments_cents: 0, after_closing_cents: null, ...patch };
}

const closed: api.ClosingView = {
  id: 1,
  closing_date: TODAY,
  closed_at: "2026-10-21T22:00:00Z",
  closed_by_name: "Administrador",
  lines: [
    { method: "cash", label: "Dinheiro", expected_cents: 10000, counted_cents: 9500, difference_cents: -500, justification: "Troco devolvido" },
    { method: "pix", label: "Pix", expected_cents: 6000, counted_cents: 6000, difference_cents: 0, justification: null },
  ],
};

function open(path = "/fechamento") {
  renderPage(<ClosingPage />, { path: "/fechamento/:data?", entry: path });
}

async function fillCounts(counts: Record<string, string>) {
  for (const [label, value] of Object.entries(counts)) {
    const input = await screen.findByLabelText(`Conferido em ${label}`);
    await userEvent.clear(input);
    await userEvent.type(input, value);
  }
}

const exact = { Dinheiro: "100,00", Pix: "60,00", "Cartão de débito": "50,00", "Cartão de crédito": "30,00", "Transferência bancária": "0,00" };

beforeEach(() => {
  signIn();
  vi.mocked(api.closingPreview).mockReset().mockResolvedValue(preview());
  vi.mocked(api.closingPendingDates).mockReset().mockResolvedValue([]);
  vi.mocked(api.closeCash).mockReset().mockResolvedValue(closed);
});

describe("ClosingPage", () => {
  it("mostra recebido, devolvido e esperado de cada forma, com débito e crédito separados", async () => {
    open();

    const pix = (await screen.findByText("Pix", { selector: "td" })).closest("tr") as HTMLElement;
    expect(within(pix).getByText("R$ 80,00")).toBeInTheDocument();
    expect(within(pix).getByText("R$ 20,00")).toBeInTheDocument();
    expect(within(pix).getByText("R$ 60,00")).toBeInTheDocument();
    expect(screen.getByText("Cartão de débito", { selector: "td" })).toBeInTheDocument();
    expect(screen.getByText("Cartão de crédito", { selector: "td" })).toBeInTheDocument();
  });

  it("calcula a diferença ao digitar o valor conferido, com texto e ícone", async () => {
    open();
    await fillCounts({ Dinheiro: "95,00" });

    const cash = screen.getByText("Dinheiro", { selector: "td" }).closest("tr") as HTMLElement;
    expect(within(cash).getByText(/Falta R\$ 5,00/)).toHaveTextContent("⚠");
    await fillCounts({ Dinheiro: "105,00" });
    expect(within(cash).getByText(/Sobra R\$ 5,00/)).toBeInTheDocument();
    await fillCounts({ Dinheiro: "100,00" });
    expect(within(cash).getByText(/Sem diferença/)).toBeInTheDocument();
  });

  it("diferença exige justificativa naquela linha, mas não impede concluir", async () => {
    open();
    await fillCounts({ ...exact, Dinheiro: "95,00" });

    await userEvent.click(screen.getByRole("button", { name: "Concluir fechamento" }));
    expect(await screen.findByText("Explique a diferença de Dinheiro.")).toBeInTheDocument();
    expect(api.closeCash).not.toHaveBeenCalled();

    await userEvent.type(screen.getByLabelText("Justificativa de Dinheiro"), "Troco devolvido");
    await userEvent.click(screen.getByRole("button", { name: "Concluir fechamento" }));
    const confirm = within(await screen.findByRole("dialog"));
    expect(confirm.getByText(/Dinheiro: falta R\$ 5,00/)).toBeInTheDocument();
    await userEvent.click(confirm.getByRole("button", { name: "Concluir fechamento" }));

    await waitFor(() =>
      expect(api.closeCash).toHaveBeenCalledWith("t", {
        date: TODAY,
        counted: [
          { method: "cash", counted_cents: 9500, justification: "Troco devolvido" },
          { method: "pix", counted_cents: 6000, justification: null },
          { method: "debit", counted_cents: 5000, justification: null },
          { method: "credit", counted_cents: 3000, justification: null },
          { method: "transfer", counted_cents: 0, justification: null },
        ],
      }),
    );
    expect(await screen.findByText(/concluído e salvo neste computador/)).toBeInTheDocument();
  });

  it("falta de valor conferido em alguma forma é apontada", async () => {
    open();
    await fillCounts({ Dinheiro: "100,00" });

    await userEvent.click(screen.getByRole("button", { name: "Concluir fechamento" }));

    expect(await screen.findAllByText("Informe o valor conferido, por exemplo R$ 0,00.")).not.toHaveLength(0);
    expect(api.closeCash).not.toHaveBeenCalled();
  });

  it("erro ao gravar mantém todos os campos preenchidos", async () => {
    vi.mocked(api.closeCash).mockRejectedValue(new api.ApiError("validation", "Esse dia já foi fechado."));
    open();
    await fillCounts(exact);
    await userEvent.click(screen.getByRole("button", { name: "Concluir fechamento" }));
    await userEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Concluir fechamento" }));

    expect(await screen.findByText("Esse dia já foi fechado.")).toBeInTheDocument();
    expect(screen.getByLabelText("Conferido em Dinheiro")).toHaveValue("100,00");
    expect(screen.getByLabelText("Conferido em Pix")).toHaveValue("60,00");
  });

  it("dia fechado vira somente leitura e mostra o que mudou depois", async () => {
    vi.mocked(api.closingPreview).mockResolvedValue(preview({ closed, after_closing_cents: 2500 }));
    open();

    expect(await screen.findByText(/Fechado por Administrador em/)).toBeInTheDocument();
    expect(screen.getByText("Troco devolvido")).toBeInTheDocument();
    expect(screen.getByText("Movimentos depois do fechamento: R$ 25,00")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Concluir fechamento" })).toBeNull();
    expect(screen.queryByLabelText("Conferido em Dinheiro")).toBeNull();
  });

  it("pagamentos a fornecedores aparecem à parte, sem entrar no esperado", async () => {
    vi.mocked(api.closingPreview).mockResolvedValue(preview({ supplier_payments_cents: 3000 }));
    open();

    expect(await screen.findByText(/Pagamentos a fornecedores no dia: R\$ 30,00/)).toBeInTheDocument();
  });

  it("linha Sem modalidade só aparece quando o backend a envia", async () => {
    vi.mocked(api.closingPreview).mockResolvedValue(preview({ expected: [...expected, expectedLine("unspecified", "Sem modalidade", 12000)] }));
    open();

    expect(await screen.findByLabelText("Conferido em Sem modalidade")).toBeInTheDocument();
  });

  it("não oferece dia futuro e lista os dias pendentes com atalho", async () => {
    vi.mocked(api.closingPendingDates).mockResolvedValue(["2026-10-20"]);
    open();

    const date = await screen.findByLabelText("Dia a fechar");
    expect(date).toHaveAttribute("max", TODAY);
    expect(await screen.findByRole("link", { name: "20/10/2026" })).toHaveAttribute("href", "/fechamento/2026-10-20");
  });

  it("escolher outro dia consulta o backend com a data", async () => {
    open();
    const date = await screen.findByLabelText("Dia a fechar");

    fireEvent.change(date, { target: { value: "2026-10-20" } });

    await waitFor(() => expect(api.closingPreview).toHaveBeenLastCalledWith("t", "2026-10-20"));
  });
});
