import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ChargeDialog from "./ChargeDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, addStayCharge: vi.fn(), addStayAdjustment: vi.fn(), listItems: vi.fn(), myPermissions: vi.fn() };
});

const onDone = vi.fn();
const onClose = vi.fn();

function open() {
  renderPage(<ChargeDialog open stayId={3} onClose={onClose} onDone={onDone} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  onClose.mockReset();
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
  vi.mocked(api.listItems).mockReset().mockResolvedValue([{ id: 1, name: "Água mineral", default_price_cents: 600, active: true, version: 1 }]);
  vi.mocked(api.addStayCharge).mockReset().mockResolvedValue(21);
  vi.mocked(api.addStayAdjustment).mockReset().mockResolvedValue(22);
});

describe("ChargeDialog", () => {
  it("escolher um item preenche a descrição e o valor padrão", async () => {
    open();
    const d = await dialog();

    await userEvent.selectOptions(await d.findByLabelText("Item do catálogo"), "1");

    expect(d.getByLabelText(/Descrição/)).toHaveValue("Água mineral");
    expect(d.getByLabelText(/Valor/)).toHaveValue("R$ 6,00");
  });

  it("valor zero ou inválido não chama o backend e preserva a descrição", async () => {
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Descrição/), "Jantar");
    await userEvent.type(d.getByLabelText(/Valor/), "0");
    await userEvent.click(d.getByRole("button", { name: "Lançar consumo" }));

    expect(await d.findByText("Informe um valor maior que zero, por exemplo R$ 35,00.")).toBeInTheDocument();
    expect(api.addStayCharge).not.toHaveBeenCalled();
    expect(d.getByLabelText(/Descrição/)).toHaveValue("Jantar");
  });

  it("envia o consumo em centavos e avisa o resultado", async () => {
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Descrição/), "Jantar");
    await userEvent.type(d.getByLabelText(/Valor/), "35,00");
    await userEvent.click(d.getByRole("button", { name: "Lançar consumo" }));

    await waitFor(() =>
      expect(api.addStayCharge).toHaveBeenCalledWith("t", 3, { kind: "consumption", description: "Jantar", amount_cents: 3500, item_id: null }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Consumo de R$ 35,00 lançado na conta."));
  });

  it("com item escolhido envia o item e o valor mostrado", async () => {
    open();
    const d = await dialog();

    await userEvent.selectOptions(await d.findByLabelText("Item do catálogo"), "1");
    await userEvent.click(d.getByRole("button", { name: "Lançar consumo" }));

    await waitFor(() =>
      expect(api.addStayCharge).toHaveBeenCalledWith("t", 3, { kind: "consumption", description: "Água mineral", amount_cents: 600, item_id: 1 }),
    );
  });

  it("sem permissão não oferece ajuste", async () => {
    open();
    const d = await dialog();

    expect(d.queryByRole("option", { name: "Ajuste da conta" })).toBeNull();
  });

  it("ajuste abatendo valor envia centavos negativos com a justificativa", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["apply_discount"]);
    open();
    const d = await dialog();

    await userEvent.selectOptions(await d.findByLabelText("Tipo de lançamento"), "adjustment");
    await userEvent.selectOptions(d.getByLabelText("Efeito no saldo"), "reduce");
    await userEvent.type(d.getByLabelText(/Descrição/), "Cortesia");
    await userEvent.type(d.getByLabelText(/Valor/), "10,00");
    await userEvent.type(d.getByLabelText(/Justificativa/), "Chuveiro com defeito");
    await userEvent.click(d.getByRole("button", { name: "Lançar ajuste" }));

    await waitFor(() =>
      expect(api.addStayAdjustment).toHaveBeenCalledWith("t", 3, {
        description: "Cortesia",
        amount_cents: -1000,
        justification: "Chuveiro com defeito",
      }),
    );
  });

  it("erro do backend aparece sem apagar o que foi digitado", async () => {
    vi.mocked(api.addStayCharge).mockRejectedValue({ code: "validation", message: "A hospedagem já foi encerrada. Não é possível mudar a conta." });
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Descrição/), "Jantar");
    await userEvent.type(d.getByLabelText(/Valor/), "35,00");
    await userEvent.click(d.getByRole("button", { name: "Lançar consumo" }));

    expect(await d.findByText("A hospedagem já foi encerrada. Não é possível mudar a conta.")).toBeInTheDocument();
    expect(d.getByLabelText(/Descrição/)).toHaveValue("Jantar");
    expect(onDone).not.toHaveBeenCalled();
  });
});
