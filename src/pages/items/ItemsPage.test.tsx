import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import ItemsPage from "./ItemsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listItems: vi.fn(),
    createItem: vi.fn(),
    updateItem: vi.fn(),
    setItemActive: vi.fn(),
  };
});

const agua: api.Item = { id: 1, name: "Água mineral", default_price_cents: 600, active: true, version: 3 };

beforeEach(() => {
  signIn();
  vi.mocked(api.listItems).mockReset().mockResolvedValue([agua]);
  vi.mocked(api.createItem).mockReset().mockResolvedValue(2);
  vi.mocked(api.updateItem).mockReset().mockResolvedValue(null);
  vi.mocked(api.setItemActive).mockReset().mockResolvedValue(null);
});

async function open() {
  renderPage(<ItemsPage />);
  await screen.findByText("Água mineral");
}

describe("ItemsPage", () => {
  it("lista os itens com o preço em reais e a situação", async () => {
    await open();

    expect(screen.getByText("R$ 6,00")).toBeInTheDocument();
    expect(screen.getByText("Ativo")).toBeInTheDocument();
  });

  it("recepção só consulta", async () => {
    signIn(recepcaoUser);
    await open();

    expect(screen.queryByRole("button", { name: "Novo item" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Desativar" })).toBeNull();
  });

  it("cria um item enviando o preço em centavos", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo item" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Nome do item/), "Café da manhã");
    await userEvent.type(dialog.getByLabelText(/Preço padrão/), "R$ 30,00");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar item" }));

    await waitFor(() =>
      expect(api.createItem).toHaveBeenCalledWith("t", { name: "Café da manhã", default_price_cents: 3000 }),
    );
  });

  it("preço inválido não chama o backend e mantém o que foi digitado", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo item" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Nome do item/), "Lavanderia");
    await userEvent.type(dialog.getByLabelText(/Preço padrão/), "0");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar item" }));

    expect(await dialog.findByText("Informe um preço maior que zero, por exemplo R$ 6,00.")).toBeInTheDocument();
    expect(api.createItem).not.toHaveBeenCalled();
    expect(dialog.getByLabelText(/Nome do item/)).toHaveValue("Lavanderia");
  });

  it("mostra o erro do backend sem fechar o formulário", async () => {
    vi.mocked(api.createItem).mockRejectedValue({ code: "validation", message: "Já existe um item com este nome." });
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo item" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Nome do item/), "agua mineral");
    await userEvent.type(dialog.getByLabelText(/Preço padrão/), "7,00");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar item" }));

    expect(await dialog.findByText("Já existe um item com este nome.")).toBeInTheDocument();
    expect(dialog.getByLabelText(/Nome do item/)).toHaveValue("agua mineral");
  });

  it("desativa o item depois de confirmar", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Desativar" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Desativar item" }));

    await waitFor(() => expect(api.setItemActive).toHaveBeenCalledWith("t", 1, false, 3));
  });
});
