import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import RatesPage from "./RatesPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listCategories: vi.fn(),
    listRateRules: vi.fn(),
    createRateRule: vi.fn(),
    updateRateRule: vi.fn(),
    setRateRuleActive: vi.fn(),
    deleteRateRule: vi.fn(),
    listHolidays: vi.fn(),
    createHoliday: vi.fn(),
    deleteHoliday: vi.fn(),
    quoteStay: vi.fn(),
  };
});

const standard: api.Category = { id: 1, name: "Standard", description: "", active: true, version: 1 };
const luxo: api.Category = { id: 2, name: "Luxo", description: "", active: true, version: 1 };

const monday: api.RateRule = {
  id: 10, category_id: 1, rule_type: "weekday", weekday: 1, start_date: null, end_date: null, date: null,
  amount_cents: 18000, label: "", active: true, version: 3,
};
const summer: api.RateRule = {
  id: 11, category_id: 1, rule_type: "season", weekday: null, start_date: "2026-12-15", end_date: "2027-02-28",
  date: null, amount_cents: 27000, label: "Alta temporada", active: true, version: 1,
};
const holidayRate: api.RateRule = {
  id: 12, category_id: 1, rule_type: "holiday", weekday: null, start_date: null, end_date: null, date: null,
  amount_cents: 25000, label: "Feriados", active: true, version: 1,
};
const festival: api.RateRule = {
  id: 13, category_id: 1, rule_type: "date_override", weekday: null, start_date: null, end_date: null,
  date: "2026-11-07", amount_cents: 32000, label: "Festival", active: true, version: 1,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.listCategories).mockResolvedValue([standard, luxo]);
  vi.mocked(api.listRateRules).mockReset().mockResolvedValue([monday, summer, holidayRate, festival]);
  vi.mocked(api.createRateRule).mockReset().mockResolvedValue(20);
  vi.mocked(api.updateRateRule).mockReset().mockResolvedValue(null);
  vi.mocked(api.setRateRuleActive).mockReset().mockResolvedValue(null);
  vi.mocked(api.deleteRateRule).mockReset().mockResolvedValue(null);
  vi.mocked(api.listHolidays).mockReset().mockResolvedValue([{ id: 1, date: "2026-11-02", name: "Finados" }]);
  vi.mocked(api.createHoliday).mockReset().mockResolvedValue(2);
  vi.mocked(api.deleteHoliday).mockReset().mockResolvedValue(null);
  vi.mocked(api.quoteStay).mockReset();
});

async function open() {
  const view = renderPage(<RatesPage />);
  await screen.findByText("Alta temporada");
  return view;
}

function row(text: string) {
  return within(screen.getByText(text).closest("tr") as HTMLElement);
}

describe("RatesPage", () => {
  it("mostra os sete dias da semana com valor ou 'Sem tarifa'", async () => {
    await open();

    expect(row("Segunda-feira").getByText("R$ 180,00")).toBeInTheDocument();
    expect(row("Terça-feira").getByText("Sem tarifa")).toBeInTheDocument();
    expect(screen.getByText("Domingo")).toBeInTheDocument();
    expect(screen.getByText("Sábado")).toBeInTheDocument();
  });

  it("mostra temporadas, preço de feriado e exceções de data", async () => {
    await open();

    expect(row("Alta temporada").getByText("15/12/2026 a 28/02/2027")).toBeInTheDocument();
    expect(row("Alta temporada").getByText("R$ 270,00")).toBeInTheDocument();
    expect(screen.getByText("Preço nos feriados cadastrados")).toBeInTheDocument();
    expect(screen.getByText("R$ 250,00")).toBeInTheDocument();
    expect(row("Festival").getByText("07/11/2026")).toBeInTheDocument();
  });

  it("trocar a categoria busca as tarifas dela", async () => {
    await open();

    await userEvent.selectOptions(screen.getByLabelText("Categoria"), "2");

    await waitFor(() => expect(api.listRateRules).toHaveBeenLastCalledWith("t", 2));
  });

  it("define a tarifa de um dia da semana sem tarifa", async () => {
    await open();
    await userEvent.click(within(screen.getByText("Terça-feira").closest("tr") as HTMLElement).getByRole("button", { name: "Definir" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Valor da diária/), "190,50");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar tarifa" }));

    await waitFor(() =>
      expect(api.createRateRule).toHaveBeenCalledWith("t", {
        category_id: 1, rule_type: "weekday", weekday: 2, start_date: null, end_date: null, date: null,
        amount_cents: 19050, label: "",
      }),
    );
  });

  it("edita uma tarifa existente enviando a versão", async () => {
    await open();
    await userEvent.click(row("Segunda-feira").getByRole("button", { name: "Editar" }));
    const dialog = within(await screen.findByRole("dialog"));
    expect(dialog.getByLabelText(/Valor da diária/)).toHaveValue("180,00");

    await userEvent.clear(dialog.getByLabelText(/Valor da diária/));
    await userEvent.type(dialog.getByLabelText(/Valor da diária/), "200,00");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar tarifa" }));

    await waitFor(() =>
      expect(api.updateRateRule).toHaveBeenCalledWith("t", 10, expect.objectContaining({ amount_cents: 20000, weekday: 1 }), 3),
    );
  });

  it("cria uma temporada validando datas e valor", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Nova temporada" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Salvar tarifa" }));
    expect(await dialog.findByText("Informe o início da temporada.")).toBeInTheDocument();
    expect(dialog.getByText("Informe um valor, por exemplo 180,00.")).toBeInTheDocument();
    expect(api.createRateRule).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Início/), "2027-03-01");
    await userEvent.type(dialog.getByLabelText(/Fim/), "2027-03-31");
    await userEvent.type(dialog.getByLabelText(/Descrição/), "Semana Santa");
    await userEvent.type(dialog.getByLabelText(/Valor da diária/), "300,00");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar tarifa" }));

    await waitFor(() =>
      expect(api.createRateRule).toHaveBeenCalledWith("t", {
        category_id: 1, rule_type: "season", weekday: null, start_date: "2027-03-01", end_date: "2027-03-31",
        date: null, amount_cents: 30000, label: "Semana Santa",
      }),
    );
  });

  it("sobreposição de temporada: mostra o erro do backend e mantém o formulário", async () => {
    vi.mocked(api.createRateRule).mockRejectedValue(
      new api.ApiError("validation", "Esta temporada se sobrepõe a outra temporada ativa da mesma categoria."),
    );
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Nova temporada" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Início/), "2027-02-01");
    await userEvent.type(dialog.getByLabelText(/Fim/), "2027-03-31");
    await userEvent.type(dialog.getByLabelText(/Valor da diária/), "300,00");

    await userEvent.click(dialog.getByRole("button", { name: "Salvar tarifa" }));

    expect(await dialog.findByText(/se sobrepõe a outra temporada/)).toBeInTheDocument();
    expect(dialog.getByLabelText(/Valor da diária/)).toHaveValue("300,00");
  });

  it("fechar o formulário com dados digitados pede confirmação", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Nova exceção de data" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Valor da diária/), "400,00");

    await userEvent.click(dialog.getByRole("button", { name: "Cancelar" }));

    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
  });

  it("recepção só consulta", async () => {
    signIn(recepcaoUser);
    await open();

    expect(screen.queryByRole("button", { name: "Nova temporada" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Editar" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Definir" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Adicionar feriado" })).toBeNull();
  });

  it("cadastra e remove feriados", async () => {
    await open();
    expect(screen.getByText("Finados")).toBeInTheDocument();

    await userEvent.type(screen.getByLabelText("Data do feriado"), "2026-11-15");
    await userEvent.type(screen.getByLabelText("Nome do feriado"), "Proclamação da República");
    await userEvent.click(screen.getByRole("button", { name: "Adicionar feriado" }));
    await waitFor(() => expect(api.createHoliday).toHaveBeenCalledWith("t", "2026-11-15", "Proclamação da República"));

    await userEvent.click(within(screen.getByText("Finados").closest("tr") as HTMLElement).getByRole("button", { name: "Remover" }));
    await userEvent.click(await screen.findByRole("button", { name: "Remover feriado" }));
    await waitFor(() => expect(api.deleteHoliday).toHaveBeenCalledWith("t", 1));
  });

  it("simula o preço noite a noite e destaca datas sem tarifa", async () => {
    vi.mocked(api.quoteStay).mockResolvedValue({
      nights: [
        { date: "2026-11-06", amount_cents: 22500, rule: "weekday" },
        { date: "2026-11-07", amount_cents: 32000, rule: "date_override" },
      ],
      total_cents: 54500,
      missing_dates: ["2026-11-08"],
    });
    await open();

    await userEvent.type(screen.getByLabelText("Entrada"), "2026-11-06");
    await userEvent.type(screen.getByLabelText("Saída"), "2026-11-09");
    await userEvent.click(screen.getByRole("button", { name: "Simular preço" }));

    await waitFor(() => expect(api.quoteStay).toHaveBeenCalledWith("t", 1, "2026-11-06", "2026-11-09"));
    expect(await screen.findByText("Exceção de data")).toBeInTheDocument();
    expect(screen.getByText("R$ 545,00")).toBeInTheDocument();
    expect(screen.getByText(/Faltam tarifas para: 08\/11\/2026/)).toBeInTheDocument();
  });
});
