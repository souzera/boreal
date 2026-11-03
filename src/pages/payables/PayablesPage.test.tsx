import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import PayablesPage from "./PayablesPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    myPermissions: vi.fn(),
    listPayables: vi.fn(),
    listExpenseCategories: vi.fn(),
    createPayable: vi.fn(),
    payPayable: vi.fn(),
    payablesSummary: vi.fn(),
    createExpenseCategory: vi.fn(),
    renameExpenseCategory: vi.fn(),
    setExpenseCategoryActive: vi.fn(),
  };
});

const categories: api.ExpenseCategory[] = [
  { id: 1, name: "Energia", active: true, version: 2 },
  { id: 2, name: "Água", active: true, version: 1 },
];

function payable(patch: Partial<api.PayableView>): api.PayableView {
  return {
    id: 1, code: "CAP-000001", supplier: "Companhia de Energia", description: "Conta de outubro", amount_cents: 50000,
    paid_cents: 0, balance_cents: 50000, status: "pending", status_label: "Pendente", competence_date: "2026-10-15",
    due_date: "2026-11-10", category_id: 1, category_name: "Energia", notes: "", version: 1, created_at: "2026-10-15T12:00:00Z",
    ...patch,
  };
}

const rows = [
  payable({}),
  payable({ id: 2, code: "CAP-000002", supplier: "Padaria Sol", status: "overdue", status_label: "Vencida", due_date: "2026-10-10", version: 3 }),
  payable({ id: 3, code: "CAP-000003", supplier: "Gás Ltda", status: "partial", status_label: "Parcial", paid_cents: 20000, balance_cents: 30000 }),
  payable({ id: 4, code: "CAP-000004", supplier: "Internet SA", status: "paid", status_label: "Paga", paid_cents: 50000, balance_cents: 0 }),
];

function open() {
  renderPage(<PayablesPage />);
}

beforeEach(() => {
  signIn();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue(["manage_payables"]);
  vi.mocked(api.listPayables).mockReset().mockResolvedValue({ items: rows, total: rows.length, page: 1, page_size: 20 });
  vi.mocked(api.listExpenseCategories).mockReset().mockResolvedValue(categories);
  vi.mocked(api.payPayable).mockReset().mockResolvedValue(9);
  vi.mocked(api.createPayable).mockReset().mockResolvedValue(5);
  vi.mocked(api.payablesSummary).mockReset().mockResolvedValue([{ category_id: 1, category_name: "Energia", total_cents: 50000 }]);
  vi.mocked(api.createExpenseCategory).mockReset().mockResolvedValue(3);
  vi.mocked(api.setExpenseCategoryActive).mockReset().mockResolvedValue(null);
  vi.mocked(api.renameExpenseCategory).mockReset().mockResolvedValue(null);
});

describe("PayablesPage", () => {
  it("sem a permissão explica que o acesso depende dela e não consulta as contas", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue([]);
    open();

    expect(await screen.findByText(/Você não tem permissão para ver as contas a pagar/)).toBeInTheDocument();
    expect(api.listPayables).not.toHaveBeenCalled();
  });

  it("mostra cada situação com rótulo e ícone, não só cor", async () => {
    open();

    expect(await screen.findByText("CAP-000001")).toBeInTheDocument();
    const cell = (label: string) => screen.getAllByRole("cell").find((c) => c.textContent?.endsWith(label)) as HTMLElement;
    expect(cell("Vencida").querySelector('[data-icon="warning"]')).not.toBeNull();
    expect(cell("Pendente").querySelector('[data-icon="radio_button_unchecked"]')).not.toBeNull();
    expect(cell("Parcial").querySelector('[data-icon="timelapse"]')).not.toBeNull();
    expect(cell("Paga").querySelector('[data-icon="check_circle"]')).not.toBeNull();
  });

  it("conta paga não oferece pagamento; as demais oferecem", async () => {
    open();
    await screen.findByText("CAP-000001");

    expect(screen.getAllByRole("button", { name: "Registrar pagamento da conta" })).toHaveLength(3);
  });

  it("registra o pagamento da conta mostrando o saldo e usando a versão vista", async () => {
    open();
    await screen.findByText("CAP-000002");
    const row = screen.getByText("CAP-000002").closest("tr") as HTMLElement;
    await userEvent.click(within(row).getByRole("button", { name: "Registrar pagamento da conta" }));
    const d = within(await screen.findByRole("dialog"));

    expect(d.getByText(/Saldo da conta: R\$ 500,00/)).toBeInTheDocument();
    await userEvent.type(d.getByLabelText(/Valor pago/), "200,00");
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "pix");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento da conta" }));
    expect(await d.findByText(/Registrar pagamento de R\$ 200,00 em Pix para Padaria Sol\?/)).toBeInTheDocument();
    expect(api.payPayable).not.toHaveBeenCalled();
    await userEvent.click(d.getByRole("button", { name: "Confirmar pagamento da conta" }));

    await waitFor(() =>
      expect(api.payPayable).toHaveBeenCalledWith("t", 2, { amount_cents: 20000, method: "pix", note: "" }, 3),
    );
  });

  it("valor acima do saldo é recusado no backend e os campos continuam preenchidos", async () => {
    vi.mocked(api.payPayable).mockRejectedValue(new api.ApiError("validation", "Informe um valor maior que zero e que não passe do saldo da conta."));
    open();
    const row = (await screen.findByText("CAP-000001")).closest("tr") as HTMLElement;
    await userEvent.click(within(row).getByRole("button", { name: "Registrar pagamento da conta" }));
    const d = within(await screen.findByRole("dialog"));
    await userEvent.type(d.getByLabelText(/Valor pago/), "900,00");
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "cash");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento da conta" }));
    expect(await d.findByText("O valor não pode passar do saldo da conta (R$ 500,00).")).toBeInTheDocument();
    expect(api.payPayable).not.toHaveBeenCalled();
    await userEvent.clear(d.getByLabelText(/Valor pago/));
    await userEvent.type(d.getByLabelText(/Valor pago/), "400,00");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento da conta" }));
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento da conta" }));

    expect(await d.findByText(/não passe do saldo da conta/)).toBeInTheDocument();
    expect(d.getByLabelText(/Valor pago/)).toHaveValue("400,00");
  });

  it("registra uma conta já paga à vista", async () => {
    open();
    await userEvent.click(await screen.findByRole("button", { name: "Registrar conta a pagar" }));
    const d = within(await screen.findByRole("dialog"));

    await userEvent.type(d.getByLabelText(/Fornecedor/), "Mercado Central");
    await userEvent.type(d.getByLabelText(/Descrição/), "Compras do café");
    await userEvent.type(d.getByLabelText(/Valor da conta/), "120,00");
    fireEvent.change(d.getByLabelText(/Data da despesa/), { target: { value: "2026-10-20" } });
    fireEvent.change(d.getByLabelText(/Vencimento/), { target: { value: "2026-10-20" } });
    await userEvent.selectOptions(d.getByLabelText(/Categoria/), "1");
    await userEvent.click(d.getByLabelText("Já paga à vista"));
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "cash");
    await userEvent.click(d.getByRole("button", { name: "Registrar conta a pagar" }));

    await waitFor(() =>
      expect(api.createPayable).toHaveBeenCalledWith("t", {
        supplier: "Mercado Central",
        description: "Compras do café",
        amount_cents: 12000,
        competence_date: "2026-10-20",
        due_date: "2026-10-20",
        category_id: 1,
        notes: "",
        paid_now: { method: "cash" },
      }),
    );
  });

  it("campos obrigatórios vazios mostram o que corrigir sem chamar o backend", async () => {
    open();
    await userEvent.click(await screen.findByRole("button", { name: "Registrar conta a pagar" }));
    const d = within(await screen.findByRole("dialog"));

    await userEvent.click(d.getByRole("button", { name: "Registrar conta a pagar" }));

    expect(await d.findByText("Informe o fornecedor.")).toBeInTheDocument();
    expect(d.getByText("Informe um valor maior que zero, por exemplo R$ 500,00.")).toBeInTheDocument();
    expect(api.createPayable).not.toHaveBeenCalled();
  });

  it("filtra por situação consultando o backend", async () => {
    open();
    await screen.findByText("CAP-000001");

    await userEvent.selectOptions(screen.getByLabelText("Situação"), "overdue");

    await waitFor(() =>
      expect(api.listPayables).toHaveBeenLastCalledWith("t", expect.objectContaining({ status: "overdue", page: 1 })),
    );
  });

  it("alterna entre competência e data de pagamento nos totais por categoria", async () => {
    open();
    await screen.findByText("CAP-000001");

    await userEvent.click(screen.getByRole("radio", { name: "Pagamento" }));

    await waitFor(() => expect(api.payablesSummary).toHaveBeenLastCalledWith("t", expect.objectContaining({ basis: "payment" })));
    expect(await screen.findByText("Energia R$ 500,00")).toBeInTheDocument();
  });

  it("administrador cria e desativa categorias", async () => {
    open();
    await userEvent.click(await screen.findByRole("button", { name: "Nova categoria" }));
    const d = within(await screen.findByRole("dialog"));
    await userEvent.type(d.getByLabelText(/Nome da categoria/), "Seguros");
    await userEvent.click(d.getByRole("button", { name: "Salvar categoria" }));
    await waitFor(() => expect(api.createExpenseCategory).toHaveBeenCalledWith("t", "Seguros"));

    const item = (await screen.findByText("Energia", { selector: "li span" })).closest("li") as HTMLElement;
    await userEvent.click(within(item).getByRole("button", { name: "Desativar" }));
    await waitFor(() => expect(api.setExpenseCategoryActive).toHaveBeenCalledWith("t", 1, false, 2));
  });

  it("recepção com a permissão de contas não vê o painel de categorias", async () => {
    signIn(recepcaoUser);
    open();
    await screen.findByText("CAP-000001");

    expect(screen.queryByRole("button", { name: "Nova categoria" })).toBeNull();
  });
});
