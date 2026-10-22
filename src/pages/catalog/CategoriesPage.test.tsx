import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import CategoriesPage from "./CategoriesPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listCategories: vi.fn(),
    createCategory: vi.fn(),
    updateCategory: vi.fn(),
    setCategoryActive: vi.fn(),
  };
});

const standard: api.Category = { id: 1, name: "Standard", description: "Quarto simples", active: true, version: 2 };

beforeEach(() => {
  signIn();
  vi.mocked(api.listCategories).mockReset().mockResolvedValue([standard]);
  vi.mocked(api.createCategory).mockReset().mockResolvedValue(2);
  vi.mocked(api.updateCategory).mockReset().mockResolvedValue(null);
  vi.mocked(api.setCategoryActive).mockReset().mockResolvedValue(null);
});

async function open() {
  renderPage(<CategoriesPage />);
  await screen.findByText("Quarto simples");
}

describe("CategoriesPage", () => {
  it("lista categorias", async () => {
    await open();

    expect(screen.getByText("Standard")).toBeInTheDocument();
    expect(screen.getByText("Ativa")).toBeInTheDocument();
  });

  it("recepção só consulta", async () => {
    signIn(recepcaoUser);
    await open();

    expect(screen.queryByRole("button", { name: "Nova categoria" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Desativar" })).toBeNull();
  });

  it("nome é obrigatório; cria a categoria", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Nova categoria" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Salvar categoria" }));
    expect(await dialog.findByText("Informe o nome da categoria.")).toBeInTheDocument();
    expect(api.createCategory).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/Nome da categoria/), "Luxo");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar categoria" }));

    await waitFor(() => expect(api.createCategory).toHaveBeenCalledWith("t", { name: "Luxo", description: "" }));
    expect(await screen.findByText("Categoria Luxo criada.")).toBeInTheDocument();
  });

  it("editar envia a versão da categoria", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Editar" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Nome da categoria/), " Plus");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar categoria" }));

    await waitFor(() =>
      expect(api.updateCategory).toHaveBeenCalledWith("t", 1, { name: "Standard Plus", description: "Quarto simples" }, 2),
    );
  });

  it("desativar com quartos ativos mostra a orientação do backend", async () => {
    vi.mocked(api.setCategoryActive).mockRejectedValue(
      new api.ApiError("validation", "Há 3 quarto(s) ativo(s) nesta categoria. Desative-os ou mude a categoria antes."),
    );
    await open();

    await userEvent.click(screen.getByRole("button", { name: "Desativar" }));
    await userEvent.click(await screen.findByRole("button", { name: "Desativar categoria" }));

    expect(await screen.findByText(/Há 3 quarto\(s\) ativo\(s\)/)).toBeInTheDocument();
  });

  it("fechar o formulário com dados digitados pede confirmação", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Nova categoria" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Nome da categoria/), "Suíte");

    await userEvent.click(dialog.getByRole("button", { name: "Cancelar" }));

    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Descartar alterações" }));
    await waitFor(() => expect(screen.queryByLabelText(/Nome da categoria/)).toBeNull());
  });
});
