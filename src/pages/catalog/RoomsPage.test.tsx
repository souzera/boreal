import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import RoomsPage from "./RoomsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listRooms: vi.fn(),
    listCategories: vi.fn(),
    createRoom: vi.fn(),
    updateRoom: vi.fn(),
    setRoomActive: vi.fn(),
  };
});

const standard: api.Category = { id: 1, name: "Standard", description: "", active: true, version: 1 };
const luxo: api.Category = { id: 2, name: "Luxo", description: "", active: true, version: 1 };
const room101: api.Room = {
  id: 10,
  code: "101",
  category_id: 1,
  category_name: "Standard",
  max_capacity: 2,
  description: "Vista para o jardim",
  notes: "",
  active: true,
  version: 4,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.listCategories).mockResolvedValue([standard, luxo]);
  vi.mocked(api.listRooms).mockReset().mockResolvedValue({ items: [room101], total: 1, page: 1, page_size: 20 });
  vi.mocked(api.createRoom).mockReset().mockResolvedValue(11);
  vi.mocked(api.updateRoom).mockReset().mockResolvedValue(null);
  vi.mocked(api.setRoomActive).mockReset().mockResolvedValue(null);
});

async function open() {
  const view = renderPage(<RoomsPage />);
  await screen.findByText("Vista para o jardim");
  return view;
}

async function fillNewRoom(dialog: ReturnType<typeof within>, code: string, capacity: string) {
  await userEvent.type(dialog.getByLabelText(/Código do quarto/), code);
  await userEvent.selectOptions(dialog.getByLabelText(/Categoria/), "1");
  await userEvent.clear(dialog.getByLabelText(/Capacidade máxima/));
  if (capacity) await userEvent.type(dialog.getByLabelText(/Capacidade máxima/), capacity);
}

describe("RoomsPage", () => {
  it("lista quartos com categoria, capacidade e situação em texto", async () => {
    await open();

    const row = within(screen.getByText("101").closest("tr") as HTMLElement);
    expect(row.getByText("Standard")).toBeInTheDocument();
    expect(row.getByText("2 pessoas")).toBeInTheDocument();
    expect(row.getByText("Ativo")).toBeInTheDocument();
  });

  it("filtra por categoria e por texto no backend", async () => {
    await open();

    await userEvent.selectOptions(screen.getByLabelText("Categoria do filtro"), "2");
    await waitFor(() =>
      expect(api.listRooms).toHaveBeenLastCalledWith("t", expect.objectContaining({ category_id: 2, page: 1 })),
    );
    await userEvent.type(screen.getByLabelText("Buscar quarto"), "10");
    await waitFor(() =>
      expect(api.listRooms).toHaveBeenLastCalledWith("t", expect.objectContaining({ text: "10" })),
    );
  });

  it("recepção não vê as ações de escrita", async () => {
    signIn(recepcaoUser);
    await open();

    expect(screen.queryByRole("button", { name: "Novo quarto" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Desativar" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Editar" })).toBeNull();
  });

  it("valida código e capacidade sem chamar o backend", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo quarto" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));
    expect(await dialog.findByText("Informe o código do quarto.")).toBeInTheDocument();
    expect(dialog.getByText("Escolha a categoria.")).toBeInTheDocument();

    await fillNewRoom(dialog, "301", "0");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));
    expect(await dialog.findByText("Capacidade máxima: informe de 1 a 50 pessoas.")).toBeInTheDocument();
    expect(api.createRoom).not.toHaveBeenCalled();
  });

  it("cria o quarto e confirma o resultado", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo quarto" }));
    const dialog = within(await screen.findByRole("dialog"));

    await fillNewRoom(dialog, "301", "3");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));

    await waitFor(() =>
      expect(api.createRoom).toHaveBeenCalledWith("t", {
        code: "301",
        category_id: 1,
        max_capacity: 3,
        description: "",
        notes: "",
      }),
    );
    expect(await screen.findByText("Quarto 301 criado.")).toBeInTheDocument();
  });

  it("código duplicado: mostra o erro do backend e mantém o que foi digitado", async () => {
    vi.mocked(api.createRoom).mockRejectedValue(new api.ApiError("validation", "Já existe um quarto com este código."));
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo quarto" }));
    const dialog = within(await screen.findByRole("dialog"));
    await fillNewRoom(dialog, "101", "2");

    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));

    expect(await dialog.findByText("Já existe um quarto com este código.")).toBeInTheDocument();
    expect(dialog.getByLabelText(/Código do quarto/)).toHaveValue("101");
  });

  it("editar abre com os valores atuais e envia a versão", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Editar" }));
    const dialog = within(await screen.findByRole("dialog"));
    expect(dialog.getByLabelText(/Código do quarto/)).toHaveValue("101");

    await userEvent.clear(dialog.getByLabelText(/Capacidade máxima/));
    await userEvent.type(dialog.getByLabelText(/Capacidade máxima/), "4");
    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));

    await waitFor(() =>
      expect(api.updateRoom).toHaveBeenCalledWith(
        "t",
        10,
        expect.objectContaining({ code: "101", category_id: 1, max_capacity: 4 }),
        4,
      ),
    );
  });

  it("conflito de versão explica o problema e mantém a edição", async () => {
    vi.mocked(api.updateRoom).mockRejectedValue(
      new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."),
    );
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Editar" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.clear(dialog.getByLabelText(/Capacidade máxima/));
    await userEvent.type(dialog.getByLabelText(/Capacidade máxima/), "4");

    await userEvent.click(dialog.getByRole("button", { name: "Salvar quarto" }));

    expect(await dialog.findByText(/foi alterado por outra operação/)).toBeInTheDocument();
    expect(dialog.getByLabelText(/Capacidade máxima/)).toHaveValue(4);
  });

  it("desativar pede confirmação e informa o resultado", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Desativar" }));
    expect(api.setRoomActive).not.toHaveBeenCalled();

    await userEvent.click(await screen.findByRole("button", { name: "Desativar quarto" }));

    await waitFor(() => expect(api.setRoomActive).toHaveBeenCalledWith("t", 10, false));
    expect(await screen.findByText("Quarto 101 desativado.")).toBeInTheDocument();
  });

  it("fechar o formulário com dados digitados pede confirmação; Continuar editando mantém tudo", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo quarto" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Código do quarto/), "777");

    await userEvent.keyboard("{Escape}");
    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));

    expect(screen.queryByText("Descartar as alterações?")).toBeNull();
    expect(screen.getByLabelText(/Código do quarto/)).toHaveValue("777");

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    await userEvent.click(await screen.findByRole("button", { name: "Descartar alterações" }));
    await waitFor(() => expect(screen.queryByLabelText(/Código do quarto/)).toBeNull());
  });

  it("fechar o formulário sem digitar nada não pergunta", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo quarto" }));
    await screen.findByRole("dialog");

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    await waitFor(() => expect(screen.queryByLabelText(/Código do quarto/)).toBeNull());
    expect(screen.queryByText("Descartar as alterações?")).toBeNull();
  });
});
