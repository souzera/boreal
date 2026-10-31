import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import RefundDialog from "./RefundDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, recordRefund: vi.fn() };
});

const onDone = vi.fn();
const payment = { id: 4, code: "PAG-000004", amount_cents: 10000, refundable_cents: 7000 };

function open() {
  renderPage(<RefundDialog open payment={payment} onClose={vi.fn()} onDone={onDone} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  vi.mocked(api.recordRefund).mockReset().mockResolvedValue({ id: 2, code: "DEV-000001", total_cents: 3000 });
});

describe("RefundDialog", () => {
  it("mostra o recebimento original e quanto ainda pode ser devolvido", async () => {
    open();
    const d = await dialog();

    expect(d.getByText(/PAG-000004/)).toBeInTheDocument();
    expect(d.getByText(/pode devolver até R\$ 70,00/)).toBeInTheDocument();
  });

  it("exige o motivo e não passa do valor devolvível, sem chamar o backend", async () => {
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor a devolver/), "80,00");
    await userEvent.selectOptions(d.getByLabelText("Forma de pagamento"), "pix");

    await userEvent.click(d.getByRole("button", { name: "Registrar devolução" }));
    expect(await d.findByText("Só é possível devolver até R$ 70,00 deste recebimento.")).toBeInTheDocument();
    expect(d.getByText("Informe o motivo da devolução.")).toBeInTheDocument();
    expect(api.recordRefund).not.toHaveBeenCalled();
  });

  it("confirma e registra a devolução com o motivo", async () => {
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor a devolver/), "30,00");
    await userEvent.selectOptions(d.getByLabelText("Forma de pagamento"), "pix");
    await userEvent.type(d.getByLabelText(/Motivo da devolução/), "Cobrança em dobro");
    await userEvent.click(d.getByRole("button", { name: "Registrar devolução" }));

    expect(await d.findByText(/Registrar devolução de R\$ 30,00 em Pix/)).toBeInTheDocument();
    await userEvent.click(d.getByRole("button", { name: "Confirmar devolução" }));

    await waitFor(() =>
      expect(api.recordRefund).toHaveBeenCalledWith("t", {
        payment_id: 4,
        components: [{ method: "pix", amount_cents: 3000 }],
        reason: "Cobrança em dobro",
      }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Devolução DEV-000001 de R$ 30,00 registrada e salva neste computador."));
  });

  it("erro do backend mantém os campos", async () => {
    vi.mocked(api.recordRefund).mockRejectedValue(new api.ApiError("validation", "As devoluções passam do valor recebido."));
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor a devolver/), "30,00");
    await userEvent.selectOptions(d.getByLabelText("Forma de pagamento"), "cash");
    await userEvent.type(d.getByLabelText(/Motivo da devolução/), "Engano");
    await userEvent.click(d.getByRole("button", { name: "Registrar devolução" }));
    await userEvent.click(await d.findByRole("button", { name: "Confirmar devolução" }));

    expect(await d.findByText("As devoluções passam do valor recebido.")).toBeInTheDocument();
    expect(d.getByLabelText(/Valor a devolver/)).toHaveValue("30,00");
    expect(d.getByLabelText(/Motivo da devolução/)).toHaveValue("Engano");
  });
});
