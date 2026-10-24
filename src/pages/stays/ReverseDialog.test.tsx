import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import ReverseDialog from "./ReverseDialog";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, reverseStayCharge: vi.fn() };
});

const charge: api.ChargeLine = {
  id: 8, kind: "consumption", description: "Água", amount_cents: 3500, night_date: null, justification: null,
  reverses_charge_id: null, reversed: false, created_by_name: "Administrador", created_at: "2026-10-21T17:00:00Z",
};

const onDone = vi.fn();

function open(patch: Partial<api.ChargeLine> = {}) {
  renderPage(<ReverseDialog open stayId={3} charge={{ ...charge, ...patch }} onClose={vi.fn()} onDone={onDone} />);
}

async function dialog() {
  return within(await screen.findByRole("dialog"));
}

beforeEach(() => {
  signIn();
  onDone.mockReset();
  vi.mocked(api.reverseStayCharge).mockReset().mockResolvedValue(null);
});

describe("ReverseDialog", () => {
  it("mostra o lançamento e explica que o original fica no histórico", async () => {
    open();
    const d = await dialog();

    expect(d.getByText(/Água/)).toBeInTheDocument();
    expect(d.getByText(/R\$ 35,00/)).toBeInTheDocument();
    expect(d.getByText(/continua no histórico/)).toBeInTheDocument();
  });

  it("sem justificativa não envia", async () => {
    open();
    const d = await dialog();

    await userEvent.click(d.getByRole("button", { name: "Estornar lançamento" }));

    expect(await d.findByText("Informe a justificativa do estorno.")).toBeInTheDocument();
    expect(api.reverseStayCharge).not.toHaveBeenCalled();
  });

  it("estorna sem relançar", async () => {
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Justificativa/), "Lançado por engano");
    await userEvent.click(d.getByRole("button", { name: "Estornar lançamento" }));

    await waitFor(() => expect(api.reverseStayCharge).toHaveBeenCalledWith("t", 3, 8, "Lançado por engano", null));
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Lançamento estornado."));
  });

  it("estorna e relança com o valor corrigido", async () => {
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Justificativa/), "Valor errado");
    await userEvent.click(d.getByLabelText("Relançar com novo valor ou descrição"));
    const value = d.getByLabelText(/Novo valor/);
    expect(value).toHaveValue("R$ 35,00");
    await userEvent.clear(value);
    await userEvent.type(value, "30,00");
    await userEvent.click(d.getByRole("button", { name: "Estornar lançamento" }));

    await waitFor(() =>
      expect(api.reverseStayCharge).toHaveBeenCalledWith("t", 3, 8, "Valor errado", {
        kind: "consumption",
        description: "Água",
        amount_cents: 3000,
        item_id: null,
      }),
    );
    await waitFor(() => expect(onDone).toHaveBeenCalledWith("Lançamento estornado e lançado novamente."));
  });

  it("diária não pode ser relançada por aqui", async () => {
    open({ kind: "night", description: "Diária de 21/10/2026", amount_cents: 20000 });
    const d = await dialog();

    expect(d.queryByLabelText("Relançar com novo valor ou descrição")).toBeNull();
  });

  it("erro do backend mantém a justificativa", async () => {
    vi.mocked(api.reverseStayCharge).mockRejectedValue({ code: "validation", message: "Este lançamento já foi estornado." });
    open();
    const d = await dialog();

    await userEvent.type(d.getByLabelText(/Justificativa/), "Engano");
    await userEvent.click(d.getByRole("button", { name: "Estornar lançamento" }));

    expect(await d.findByText("Este lançamento já foi estornado.")).toBeInTheDocument();
    expect(d.getByLabelText(/Justificativa/)).toHaveValue("Engano");
  });
});
