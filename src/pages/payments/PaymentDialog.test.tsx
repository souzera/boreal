import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import PaymentDialog from "./PaymentDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, recordStayPayment: vi.fn(), recordReservationPayment: vi.fn(), myPermissions: vi.fn() };
});

const onDone = vi.fn();
const onConflict = vi.fn();

type Target = React.ComponentProps<typeof PaymentDialog>["target"];

function open(target: Target = { kind: "stay", stayId: 3 }) {
  renderPage(<PaymentDialog open target={target} hint="Saldo a receber R$ 240,00" onClose={vi.fn()} onDone={onDone} onConflict={onConflict} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

const receipt = { id: 5, code: "PAG-000001", total_cents: 10000 };
const duplicate = new api.ApiError("duplicate_payment", "Possível recebimento duplicado", undefined, {
  code: "PAG-000009",
  received_at: "2026-10-21T15:00:00Z",
  total_cents: 10000,
  methods: ["pix"],
});

async function fillSinglePix(d: Awaited<ReturnType<typeof dialog>>) {
  await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
  await userEvent.selectOptions(d.getByLabelText("Forma de pagamento"), "pix");
  await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  onConflict.mockReset();
  vi.mocked(api.recordStayPayment).mockReset().mockResolvedValue(receipt);
  vi.mocked(api.recordReservationPayment).mockReset().mockResolvedValue(receipt);
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
});

describe("PaymentDialog", () => {
  it("mostra a situação da conta recebida do chamador", async () => {
    open();
    const d = await dialog();

    expect(d.getByText(/Saldo a receber R\$ 240,00/)).toBeInTheDocument();
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

  it("pede confirmação com o valor antes de enviar e avisa que foi salvo neste computador", async () => {
    open();
    const d = await dialog();

    await fillSinglePix(d);

    expect(await d.findByText(/Registrar pagamento de R\$ 100,00 em Pix\?/)).toBeInTheDocument();
    expect(api.recordStayPayment).not.toHaveBeenCalled();
    await userEvent.click(d.getByRole("button", { name: "Confirmar pagamento" }));

    await waitFor(() =>
      expect(api.recordStayPayment).toHaveBeenCalledWith("t", 3, { components: [{ method: "pix", amount_cents: 10000 }], note: "" }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Pagamento PAG-000001 de R$ 100,00 registrado e salvo neste computador."));
  });

  it("dividido: só habilita quando a soma confere e envia todos os componentes", async () => {
    open();
    const d = await dialog();
    await userEvent.type(d.getByLabelText(/Valor recebido/), "100,00");
    await userEvent.click(d.getByRole("button", { name: "Dividir em outra forma" }));
    await userEvent.selectOptions(d.getByLabelText("Forma de pagamento 1"), "pix");
    await userEvent.type(d.getByLabelText("Valor da forma 1"), "60,00");
    await userEvent.selectOptions(d.getByLabelText("Forma de pagamento 2"), "cash");
    await userEvent.type(d.getByLabelText("Valor da forma 2"), "30,00");

    expect(d.getByRole("status")).toHaveTextContent("Falta R$ 10,00");
    expect(d.getByRole("button", { name: "Registrar pagamento" })).toBeDisabled();

    await userEvent.clear(d.getByLabelText("Valor da forma 2"));
    await userEvent.type(d.getByLabelText("Valor da forma 2"), "40,00");
    await userEvent.click(d.getByRole("button", { name: "Registrar pagamento" }));
    expect(await d.findByText(/dividido em Pix R\$ 60,00 \+ Dinheiro R\$ 40,00/)).toBeInTheDocument();
    await userEvent.click(d.getByRole("button", { name: "Confirmar pagamento" }));

    await waitFor(() =>
      expect(api.recordStayPayment).toHaveBeenCalledWith("t", 3, {
        components: [
          { method: "pix", amount_cents: 6000 },
          { method: "cash", amount_cents: 4000 },
        ],
        note: "",
      }),
    );
  });

  it("corrigir volta aos campos sem perder o que foi digitado", async () => {
    open();
    const d = await dialog();
    await fillSinglePix(d);

    await userEvent.click(await d.findByRole("button", { name: "Corrigir dados" }));

    expect(d.getByLabelText(/Valor recebido/)).toHaveValue("100,00");
    expect(d.getByLabelText("Forma de pagamento")).toHaveValue("pix");
  });

  it("erro do backend mantém os campos e mostra a mensagem", async () => {
    vi.mocked(api.recordStayPayment).mockRejectedValue(new api.ApiError("validation", "A hospedagem já foi encerrada. Não é possível mudar a conta."));
    open();
    const d = await dialog();
    await fillSinglePix(d);
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento" }));

    expect(await d.findByText("A hospedagem já foi encerrada. Não é possível mudar a conta.")).toBeInTheDocument();
    expect(d.getByLabelText(/Valor recebido/)).toHaveValue("100,00");
    expect(onDone).not.toHaveBeenCalled();
  });

  it("possível duplicado: mostra o recebimento parecido e sem permissão não deixa liberar", async () => {
    vi.mocked(api.recordStayPayment).mockRejectedValue(duplicate);
    open();
    const d = await dialog();
    await fillSinglePix(d);
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento" }));

    expect(await d.findByText(/PAG-000009/)).toBeInTheDocument();
    expect(d.getByText(/Somente quem tem a permissão Liberar recebimento duplicado/)).toBeInTheDocument();
    expect(d.queryByRole("button", { name: "Liberar recebimento duplicado" })).toBeNull();

    await userEvent.click(d.getByRole("button", { name: "Voltar e corrigir" }));
    expect(d.getByLabelText(/Valor recebido/)).toHaveValue("100,00");
  });

  it("com a permissão, libera o duplicado com justificativa", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["release_duplicate_payment"]);
    vi.mocked(api.recordStayPayment).mockRejectedValueOnce(duplicate);
    open();
    const d = await dialog();
    await fillSinglePix(d);
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento" }));
    await d.findByText(/PAG-000009/);

    await userEvent.click(await d.findByRole("button", { name: "Liberar recebimento duplicado" }));
    expect(await d.findByText("Informe a justificativa para liberar o recebimento.")).toBeInTheDocument();
    await userEvent.type(d.getByLabelText(/Justificativa/), "Dois hóspedes pagaram o mesmo valor");
    await userEvent.click(d.getByRole("button", { name: "Liberar recebimento duplicado" }));

    await waitFor(() =>
      expect(api.recordStayPayment).toHaveBeenLastCalledWith("t", 3, {
        components: [{ method: "pix", amount_cents: 10000 }],
        note: "",
        duplicate_release: { justification: "Dois hóspedes pagaram o mesmo valor" },
      }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalled());
  });

  it("sinal da reserva usa a versão vista na tela e avisa conflito", async () => {
    vi.mocked(api.recordReservationPayment).mockRejectedValue(new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."));
    open({ kind: "reservation", reservationId: 5, version: 3 });
    const d = await dialog();
    await fillSinglePix(d);
    await userEvent.click(await d.findByRole("button", { name: "Confirmar pagamento" }));

    await waitFor(() =>
      expect(api.recordReservationPayment).toHaveBeenCalledWith("t", 5, 3, { components: [{ method: "pix", amount_cents: 10000 }], note: "" }),
    );
    expect(await d.findByText(/O registro foi alterado por outra operação/)).toBeInTheDocument();
    expect(onConflict).toHaveBeenCalled();
  });
});
