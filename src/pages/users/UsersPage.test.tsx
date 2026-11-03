import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import UsersPage from "./UsersPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listUsers: vi.fn(),
    createUser: vi.fn(),
    resetPassword: vi.fn(),
    setUserRole: vi.fn(),
    setUserActive: vi.fn(),
    grantablePermissions: vi.fn(),
    setExtraPermissions: vi.fn(),
  };
});

const maria: api.UserRow = {
  id: 2,
  username: "maria",
  display_name: "Maria Souza",
  role: "recepcao",
  active: true,
  must_change_password: false,
  version: 3,
  extra_permissions: [],
};
const admin: api.UserRow = {
  id: 1,
  username: "admin",
  display_name: "Administrador",
  role: "admin",
  active: true,
  must_change_password: false,
  version: 1,
  extra_permissions: [],
};

beforeEach(() => {
  signIn();
  vi.mocked(api.listUsers).mockReset().mockResolvedValue({ items: [admin, maria], total: 2, page: 1, page_size: 20 });
  vi.mocked(api.createUser).mockReset().mockResolvedValue(3);
  vi.mocked(api.resetPassword).mockReset().mockResolvedValue(null);
  vi.mocked(api.setUserRole).mockReset().mockResolvedValue(null);
  vi.mocked(api.setUserActive).mockReset().mockResolvedValue(null);
  vi.mocked(api.setExtraPermissions).mockReset().mockResolvedValue(null);
  vi.mocked(api.grantablePermissions).mockResolvedValue(["apply_discount", "release_room"]);
});

async function open() {
  const view = renderPage(<UsersPage />);
  await screen.findByText("Maria Souza");
  return view;
}

function rowOf(name: string) {
  return screen.getByText(name).closest("tr") as HTMLElement;
}

describe("UsersPage", () => {
  it("lista usuários com perfil e situação em texto", async () => {
    await open();

    const row = within(rowOf("Maria Souza"));
    expect(row.getByText("maria")).toBeInTheDocument();
    expect(row.getByText("Recepção")).toBeInTheDocument();
    expect(row.getByText("Ativo")).toBeInTheDocument();
    expect(screen.getByText("Mostrando 1–2 de 2")).toBeInTheDocument();
  });

  it("busca envia o texto ao backend", async () => {
    await open();

    await userEvent.type(screen.getByLabelText("Buscar usuário"), "mar");

    await waitFor(() =>
      expect(api.listUsers).toHaveBeenLastCalledWith("t", expect.objectContaining({ text: "mar", page: 1 })),
    );
  });

  it("novo usuário valida campos e cria com senha temporária", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo usuário" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.click(dialog.getByRole("button", { name: "Criar usuário" }));
    expect(await dialog.findByText("Informe o usuário.")).toBeInTheDocument();
    expect(dialog.getByText("Informe o nome completo.")).toBeInTheDocument();
    expect(api.createUser).not.toHaveBeenCalled();

    await userEvent.type(dialog.getByLabelText(/^Usuário/), "joao");
    await userEvent.type(dialog.getByLabelText(/Nome completo/), "João Lima");
    await userEvent.type(dialog.getByLabelText(/Senha temporária/), "12345");
    await userEvent.click(dialog.getByRole("button", { name: "Criar usuário" }));
    expect(await dialog.findByText("A senha deve ter pelo menos 6 caracteres.")).toBeInTheDocument();

    await userEvent.clear(dialog.getByLabelText(/Senha temporária/));
    await userEvent.type(dialog.getByLabelText(/Senha temporária/), "123456");
    await userEvent.click(dialog.getByRole("button", { name: "Criar usuário" }));

    await waitFor(() =>
      expect(api.createUser).toHaveBeenCalledWith("t", {
        username: "joao",
        display_name: "João Lima",
        role: "recepcao",
        initial_password: "123456",
      }),
    );
    expect(await screen.findByText(/Usuário joao criado/)).toBeInTheDocument();
  });

  it("erro do backend ao criar mantém o formulário preenchido", async () => {
    vi.mocked(api.createUser).mockRejectedValue(new api.ApiError("validation", "Já existe um usuário com este nome."));
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo usuário" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/^Usuário/), "maria");
    await userEvent.type(dialog.getByLabelText(/Nome completo/), "Outra Maria");
    await userEvent.type(dialog.getByLabelText(/Senha temporária/), "123456");

    await userEvent.click(dialog.getByRole("button", { name: "Criar usuário" }));

    expect(await dialog.findByText("Já existe um usuário com este nome.")).toBeInTheDocument();
    expect(dialog.getByLabelText(/^Usuário/)).toHaveValue("maria");
  });

  it("permissões: só oferece as concedíveis e salva a seleção", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Permissões" }));
    const dialog = within(await screen.findByRole("dialog"));

    expect(await dialog.findByRole("checkbox", { name: "Aplicar descontos" })).toBeInTheDocument();
    expect(dialog.getByRole("checkbox", { name: "Liberar quartos" })).toBeInTheDocument();
    expect(dialog.queryByRole("checkbox", { name: /Gerenciar usuários/ })).toBeNull();

    await userEvent.click(dialog.getByRole("checkbox", { name: "Aplicar descontos" }));
    await userEvent.click(dialog.getByRole("button", { name: "Salvar permissões" }));

    await waitFor(() => expect(api.setExtraPermissions).toHaveBeenCalledWith("t", 2, ["apply_discount"]));
  });

  it("administrador não tem botão de permissões", async () => {
    await open();

    expect(within(rowOf("admin")).queryByRole("button", { name: "Permissões" })).toBeNull();
  });

  it("redefinir senha envia a versão do registro", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Redefinir senha" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.type(dialog.getByLabelText(/Senha temporária/), "654321");
    await userEvent.click(dialog.getByRole("button", { name: "Redefinir senha" }));

    await waitFor(() => expect(api.resetPassword).toHaveBeenCalledWith("t", 2, "654321", 3));
  });

  it("desativar pede confirmação antes de chamar o backend", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Desativar" }));

    expect(api.setUserActive).not.toHaveBeenCalled();
    await userEvent.click(await screen.findByRole("button", { name: "Desativar usuário" }));

    await waitFor(() => expect(api.setUserActive).toHaveBeenCalledWith("t", 2, false));
  });

  it("alterar perfil pede confirmação e envia a versão", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Alterar perfil" }));
    expect(api.setUserRole).not.toHaveBeenCalled();

    await userEvent.click(await screen.findByRole("button", { name: "Tornar administrador" }));

    await waitFor(() => expect(api.setUserRole).toHaveBeenCalledWith("t", 2, "admin", 3));
  });

  it("não oferece alterar o próprio perfil nem desativar a si mesmo", async () => {
    await open();
    const row = within(rowOf("admin"));

    expect(row.queryByRole("button", { name: "Alterar perfil" })).toBeNull();
    expect(row.queryByRole("button", { name: "Desativar" })).toBeNull();
  });

  it("fechar o novo usuário com dados digitados pede confirmação", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo usuário" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/^Usuário/), "joao");

    await userEvent.keyboard("{Escape}");

    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));
    expect(screen.getByLabelText(/^Usuário/)).toHaveValue("joao");
  });

  it("fechar permissões com seleção alterada pede confirmação", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Permissões" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.click(await dialog.findByRole("checkbox", { name: "Liberar quartos" }));

    await userEvent.click(dialog.getByRole("button", { name: "Cancelar" }));

    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
  });

  it("fechar redefinição de senha com senha digitada pede confirmação", async () => {
    await open();
    await userEvent.click(within(rowOf("Maria Souza")).getByRole("button", { name: "Redefinir senha" }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByLabelText(/Senha temporária/), "abc");

    await userEvent.keyboard("{Escape}");

    expect(await screen.findByText("Descartar as alterações?")).toBeInTheDocument();
  });

  it("descartar o novo usuário limpa o formulário para a próxima vez", async () => {
    await open();
    await userEvent.click(screen.getByRole("button", { name: "Novo usuário" }));
    await userEvent.type(within(await screen.findByRole("dialog")).getByLabelText(/^Usuário/), "joao");
    await userEvent.keyboard("{Escape}");
    await userEvent.click(await screen.findByRole("button", { name: "Descartar alterações" }));
    await waitFor(() => expect(screen.queryByLabelText(/^Usuário/)).toBeNull());

    await userEvent.click(screen.getByRole("button", { name: "Novo usuário" }));

    expect(within(await screen.findByRole("dialog")).getByLabelText(/^Usuário/)).toHaveValue("");
  });
});
