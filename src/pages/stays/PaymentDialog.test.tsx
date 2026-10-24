import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import PaymentDialog from "./PaymentDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, recordStayPayment: vi.fn() };
});

const onDone = vi.fn();

function open() {
  renderPage(<PaymentDialog open stayId={3} balanceCents={24000} onClose={vi.fn()} onDone={onDone} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  vi.mocked(api.recordStayPayment).mockReset().mockResolvedValue(5);
});

describe("PaymentDialog", () => {
  it("mostra o saldo e marca o registro como provisório", async () => {
    open();
    const d = await dialog();

    expect(d.getByText(/Saldo a receber R\$ 240,00/)).toBeInTheDocument();
    expect(d.getByText(/provisório/i)).toBeInTheDocument();
  });

  it("valor inválido e forma de pagamento são exigidos sem chamar o backend", async () => {
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));
    expect(await d.findByText("Informe um valor maior que zero, por exemplo R$ 100,00.")).toBeInTheDocument();
    await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));
    expect(await d.findByText("Escolha a forma de pagamento.")).toBeInTheDocument();
    expect(api.recordStayPayment).not.toHaveBeenCalled();
  });

  it("pede confirmação com o valor antes de enviar", async () => {
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "pix");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));

    expect(await d.findByText(/Registrar pagamento de R\$ 100,00 em Pix\?/)).toBeInTheDocument();
    expect(api.recordStayPayment).not.toHaveBeenCalled();
    await userEvent.click(d.getByRole("button", { name: "Confirmar pagamento" }));

    await waitFor(() => expect(api.recordStayPayment).toHaveBeenCalledWith("t", 3, { amount_cents: 10000, method: "pix", note: "" }));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Pagamento de R$ 100,00 registrado."));
  });

  it("corrigir volta aos campos sem perder o que foi digitado", async () => {
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "cash");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));

    await userEvent.click(await d.findByRole("button", { name: "Corrigir dados" }));

    expect(d.getByLabelText(/Valor recebido/)).toHaveValue("100,00");
  });

  it("erro do backend mantém os campos e mostra a mensagem", async () => {
    vi.mocked(api.recordStayPayment).mockRejectedValue({ code: "validation", message: "A hospedagem já foi encerrada. Não é possível mudar a conta." });
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
    await userEvent.selectOptions(d.getByLabelText(/Forma de pagamento/), "cash");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento" }));

    expect(await d.findByText("A hospedagem já foi encerrada. Não é possível mudar a conta.")).toBeInTheDocument();
    expect(onDone).not.toHaveBeenCalled();
  });
});
