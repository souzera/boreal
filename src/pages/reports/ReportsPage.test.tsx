import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import ReportsPage from "./ReportsPage";

const save = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: (...args: unknown[]) => save(...args) }));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    runReport: vi.fn(),
    exportReportCsv: vi.fn(),
    exportReportPdf: vi.fn(),
    openExportedFile: vi.fn(),
    listRooms: vi.fn(),
    listExpenseCategories: vi.fn(),
  };
});

const FIXED_TODAY = new Date(2099, 9, 24, 12, 0, 0); // 24/10/2099

function table(patch: Partial<api.ReportTable> = {}): api.ReportTable {
  return {
    kind: "receipts",
    title: "Recebimentos e devoluções",
    filters: ["Período: 25/09/2099 a 24/10/2099"],
    columns: [
      { key: "date", label: "Data", kind: "date" },
      { key: "guest", label: "Hóspede", kind: "text" },
      { key: "amount", label: "Valor", kind: "money" },
      { key: "rate", label: "Taxa", kind: "percent" },
    ],
    rows: [
      [{ kind: "text", value: "24/10/2099" }, { kind: "text", value: "Ana Souza" }, { kind: "money", value: 15000 }, { kind: "percent", value: 7500 }],
      [{ kind: "text", value: "24/10/2099" }, { kind: "text", value: "Bruno Lima" }, { kind: "money", value: -3000 }, { kind: "empty" }],
    ],
    totals: [{ kind: "text", value: "Total" }, { kind: "empty" }, { kind: "money", value: 12000 }, { kind: "empty" }],
    truncated: false,
    ...patch,
  };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: FIXED_TODAY });
  signIn();
  save.mockReset();
  vi.mocked(api.runReport).mockReset().mockResolvedValue(table());
  vi.mocked(api.exportReportCsv).mockReset().mockResolvedValue({ path: "C:/x/r.csv", bytes: 10 });
  vi.mocked(api.exportReportPdf).mockReset().mockResolvedValue({ path: "C:/x/r.pdf", bytes: 10 });
  vi.mocked(api.openExportedFile).mockReset().mockResolvedValue(null);
  vi.mocked(api.listRooms).mockReset().mockResolvedValue({ items: [{ id: 1, code: "101" }, { id: 2, code: "102" }] as api.Room[], total: 2, page: 1, page_size: 100 });
  vi.mocked(api.listExpenseCategories).mockReset().mockResolvedValue([{ id: 7, name: "Energia", active: true, version: 1 }]);
});
afterEach(() => vi.useRealTimers());

function open(search = "") {
  return renderPage(<ReportsPage />, { path: "/relatorios", entry: { pathname: "/relatorios", search } });
}

const reportSelect = () => screen.getByRole("combobox", { name: /^Relatório/ });
const optionNames = () => within(reportSelect()).getAllByRole("option").map((o) => o.textContent);

describe("ReportsPage", () => {
  it("o administrador vê os dez relatórios e a recepção só os quatro operacionais", () => {
    open();
    expect(optionNames()).toHaveLength(10);
    expect(optionNames()).toContain("Recebimentos e devoluções");
  });

  it("a recepção não vê os relatórios financeiros", () => {
    signIn(recepcaoUser);
    open();

    expect(optionNames()).toEqual(["Ocupação", "Reservas", "Hospedagens", "Cancelamentos e no-shows"]);
  });

  it("a recepção que abre um relatório financeiro pela URL não consulta nada", () => {
    signIn(recepcaoUser);
    open("?tipo=receipts");

    expect(api.runReport).not.toHaveBeenCalled();
    expect(screen.queryByRole("region", { name: "Resultado" })).toBeNull();
  });

  it("trocar o relatório mostra só os filtros dele", async () => {
    open();

    expect(screen.getByLabelText("Quarto")).toBeInTheDocument();
    expect(screen.queryByLabelText("Forma de pagamento")).toBeNull();
    await userEvent.selectOptions(reportSelect(), "receipts");
    expect(screen.getByLabelText("Forma de pagamento")).toBeInTheDocument();
    expect(screen.queryByLabelText("Situação")).toBeNull();
    await userEvent.selectOptions(reportSelect(), "balances");
    expect(screen.getByText("Mostra a situação de agora; não usa período.")).toBeInTheDocument();
    expect(screen.queryByLabelText("De")).toBeNull();
  });

  it("período invertido ou acima de 366 dias mostra o erro e não consulta", async () => {
    open();

    fireEvent.change(screen.getByLabelText("Até"), { target: { value: "2099-01-01" } });
    await userEvent.click(screen.getByRole("button", { name: "Gerar relatório" }));
    expect(await screen.findByText("A data final não pode ser antes da inicial.")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Até"), { target: { value: "2099-10-24" } });
    fireEvent.change(screen.getByLabelText("De"), { target: { value: "2097-01-01" } });
    await userEvent.click(screen.getByRole("button", { name: "Gerar relatório" }));
    expect(await screen.findByText("O período pode ter no máximo 366 dias.")).toBeInTheDocument();
    expect(api.runReport).not.toHaveBeenCalled();
  });

  it("Gerar relatório consulta com os filtros combinados e mostra colunas, linhas e totais", async () => {
    open();
    await userEvent.selectOptions(reportSelect(), "receipts");
    await userEvent.selectOptions(screen.getByLabelText("Forma de pagamento"), "pix");
    await userEvent.type(screen.getByLabelText("Texto"), "ana");
    await waitFor(() => expect(screen.getByRole("option", { name: "101" })).toBeInTheDocument());
    await userEvent.selectOptions(screen.getByLabelText("Quarto"), "2");

    await userEvent.click(screen.getByRole("button", { name: "Gerar relatório" }));

    await screen.findByRole("region", { name: "Resultado" });
    expect(api.runReport).toHaveBeenCalledWith("t", {
      kind: "receipts", from: "2099-09-25", to: "2099-10-24", status: undefined, method: "pix", text: "ana", room_id: 2,
    });
    const grid = screen.getByRole("table");
    expect(within(grid).getByText("R$ 150,00")).toBeInTheDocument();
    expect(within(grid).getByText("-R$ 30,00")).toBeInTheDocument();
    expect(within(grid).getByText("75,0%")).toBeInTheDocument();
    expect(within(grid).getByText("R$ 120,00")).toBeInTheDocument();
    expect(screen.getByText("Período: 25/09/2099 a 24/10/2099")).toBeInTheDocument();
  });

  it("vazio explica e truncado avisa", async () => {
    vi.mocked(api.runReport).mockResolvedValue(table({ rows: [], totals: null }));
    open("?tipo=receipts");
    expect(await screen.findByText("Nenhum registro para estes filtros.")).toBeInTheDocument();
  });

  it("linhas cortadas pelo limite mostram o aviso", async () => {
    vi.mocked(api.runReport).mockResolvedValue(table({ truncated: true, totals: null }));
    open("?tipo=receipts");

    expect(await screen.findByText(/Mostrando só as primeiras linhas/)).toBeInTheDocument();
  });

  it("o relatório e os filtros vêm da URL ao recarregar", async () => {
    open("?tipo=payables&de=2099-10-01&ate=2099-10-31&situacao=overdue&categoria=7&texto=luz");

    await screen.findByRole("region", { name: "Resultado" });
    expect(api.runReport).toHaveBeenCalledWith("t", { kind: "payables", from: "2099-10-01", to: "2099-10-31", status: "overdue", method: undefined, text: "luz", category_id: 7 });
    expect(reportSelect()).toHaveValue("payables");
  });

  it("Exportar CSV abre Salvar como com o nome sugerido, usa os mesmos filtros e oferece abrir o arquivo", async () => {
    save.mockResolvedValue("C:/x/r.csv");
    open("?tipo=receipts&forma=pix");
    await screen.findByRole("region", { name: "Resultado" });
    await userEvent.selectOptions(screen.getByLabelText("Separador do CSV"), ",");

    await userEvent.click(screen.getByRole("button", { name: "Exportar CSV" }));

    expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "relatorio-receipts-2099-10-24.csv" }));
    await waitFor(() =>
      expect(api.exportReportCsv).toHaveBeenCalledWith("t", expect.objectContaining({ kind: "receipts", method: "pix" }), ",", "C:/x/r.csv"),
    );
    expect(await screen.findByText("Arquivo salvo em C:/x/r.csv")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Abrir arquivo" }));
    expect(api.openExportedFile).toHaveBeenCalledWith("t", "C:/x/r.csv");
  });

  it("Exportar PDF usa o diálogo e a exportação em PDF", async () => {
    save.mockResolvedValue("C:/x/r.pdf");
    open("?tipo=receipts");
    await screen.findByRole("region", { name: "Resultado" });

    await userEvent.click(screen.getByRole("button", { name: "Exportar PDF" }));

    await waitFor(() => expect(api.exportReportPdf).toHaveBeenCalledWith("t", expect.objectContaining({ kind: "receipts" }), "C:/x/r.pdf"));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "relatorio-receipts-2099-10-24.pdf" }));
  });

  it("cancelar o diálogo não exporta nem mostra erro", async () => {
    save.mockResolvedValue(null);
    open("?tipo=receipts");
    await screen.findByRole("region", { name: "Resultado" });

    await userEvent.click(screen.getByRole("button", { name: "Exportar CSV" }));

    expect(api.exportReportCsv).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("falha ao gravar mostra a mensagem e Tentar de novo repete só a gravação", async () => {
    save.mockResolvedValue("C:/x/r.csv");
    vi.mocked(api.exportReportCsv)
      .mockRejectedValueOnce({ code: "validation", message: "Não foi possível gravar neste local. Escolha outra pasta e tente de novo." })
      .mockResolvedValueOnce({ path: "C:/x/r.csv", bytes: 10 });
    open("?tipo=receipts");
    await screen.findByRole("region", { name: "Resultado" });

    await userEvent.click(screen.getByRole("button", { name: "Exportar CSV" }));
    expect(await screen.findByText(/Escolha outra pasta/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

    expect(await screen.findByText("Arquivo salvo em C:/x/r.csv")).toBeInTheDocument();
    expect(save).toHaveBeenCalledTimes(1);
    expect(api.exportReportCsv).toHaveBeenCalledTimes(2);
    expect(api.runReport).toHaveBeenCalledTimes(1);
  });

  it("mostra a recusa do backend", async () => {
    vi.mocked(api.runReport).mockRejectedValue({ code: "forbidden", message: "Você não tem permissão para esta ação." });
    open("?tipo=receipts");

    expect(await screen.findByText("Você não tem permissão para esta ação.")).toBeInTheDocument();
  });
});
