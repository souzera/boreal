import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import GuestFormPage from "./GuestFormPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, getGuest: vi.fn(), createGuest: vi.fn(), updateGuest: vi.fn(), setGuestActive: vi.fn() };
});

const ana: api.Guest = {
  id: 7,
  code: "HOS-000007",
  full_name: "Ana Souza",
  phone: "11999990000",
  cpf: "52998224725",
  email: "ana@example.com",
  address: "Rua das Flores, 100",
  active: true,
  version: 3,
};

const routes = [
  { path: "/hospedes", element: <p>Lista de hóspedes</p> },
  { path: "/hospedes/7", element: <p>Cadastro da Ana</p> },
];

function openNew() {
  return renderPage(<GuestFormPage />, { path: "/hospedes/novo", extraRoutes: routes });
}

async function openEdit() {
  const view = renderPage(<GuestFormPage />, { path: "/hospedes/:id", entry: "/hospedes/7", extraRoutes: [routes[0]] });
  await screen.findByDisplayValue("Ana Souza");
  return view;
}

async function fillRequired(name = "Ana Souza", phone = "11999990000") {
  await userEvent.type(screen.getByLabelText(/Nome completo/), name);
  await userEvent.type(screen.getByLabelText(/Telefone/), phone);
}

beforeEach(() => {
  signIn();
  vi.mocked(api.getGuest).mockReset().mockResolvedValue(ana);
  vi.mocked(api.createGuest).mockReset().mockResolvedValue({ status: "created", id: 7, code: "HOS-000007" });
  vi.mocked(api.updateGuest).mockReset().mockResolvedValue(null);
  vi.mocked(api.setGuestActive).mockReset().mockResolvedValue(null);
});

describe("GuestFormPage", () => {
  it("nome e telefone são obrigatórios, com * e legenda", async () => {
    openNew();

    expect(screen.getByText("* campo obrigatório")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));

    expect(await screen.findByText("Informe o nome completo.")).toBeInTheDocument();
    expect(screen.getByText("Informe o telefone.")).toBeInTheDocument();
    expect(api.createGuest).not.toHaveBeenCalled();
  });

  it("valida CPF, telefone e e-mail antes de enviar", async () => {
    openNew();
    await fillRequired("Ana Souza", "123");
    await userEvent.type(screen.getByLabelText(/CPF/), "11111111111");
    await userEvent.type(screen.getByLabelText(/E-mail/), "sem-arroba");

    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));

    expect(await screen.findByText("Telefone inválido. Informe o DDD e o número.")).toBeInTheDocument();
    expect(screen.getByText("CPF inválido. Confira os números.")).toBeInTheDocument();
    expect(screen.getByText("E-mail inválido.")).toBeInTheDocument();
    expect(api.createGuest).not.toHaveBeenCalled();
  });

  it("aplica máscara de CPF e telefone enquanto digita", async () => {
    openNew();

    await userEvent.type(screen.getByLabelText(/CPF/), "52998224725");
    await userEvent.type(screen.getByLabelText(/Telefone/), "11999990000");

    expect(screen.getByLabelText(/CPF/)).toHaveValue("529.982.247-25");
    expect(screen.getByLabelText(/Telefone/)).toHaveValue("(11) 99999-0000");
  });

  it("cadastra e volta à lista", async () => {
    openNew();
    await fillRequired();

    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));

    await waitFor(() =>
      expect(api.createGuest).toHaveBeenCalledWith(
        "t",
        { full_name: "Ana Souza", phone: "(11) 99999-0000", cpf: null, email: "", address: "" },
        false,
      ),
    );
    expect(await screen.findByText("Lista de hóspedes")).toBeInTheDocument();
  });

  it("possível duplicidade: mostra os motivos, mantém o formulário e permite criar outro cadastro", async () => {
    vi.mocked(api.createGuest)
      .mockResolvedValueOnce({ status: "duplicates", matches: [{ guest: ana, reasons: ["phone", "name"] }] })
      .mockResolvedValueOnce({ status: "created", id: 8, code: "HOS-000008" });
    openNew();
    await fillRequired();

    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));
    const dialog = within(await screen.findByRole("dialog"));

    expect(dialog.getByText("Ana Souza")).toBeInTheDocument();
    expect(dialog.getByText("HOS-000007")).toBeInTheDocument();
    expect(dialog.getByText(/mesmo telefone/)).toBeInTheDocument();
    expect(dialog.getByText(/nome parecido/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Nome completo/)).toHaveValue("Ana Souza");

    await userEvent.click(dialog.getByRole("button", { name: "Criar outro cadastro" }));

    await waitFor(() => expect(api.createGuest).toHaveBeenLastCalledWith("t", expect.anything(), true));
    expect(await screen.findByText("Lista de hóspedes")).toBeInTheDocument();
  });

  it('"Usar este cadastro" abre o cadastro existente sem criar outro', async () => {
    vi.mocked(api.createGuest).mockResolvedValue({ status: "duplicates", matches: [{ guest: ana, reasons: ["cpf"] }] });
    openNew();
    await fillRequired();

    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));
    const dialog = within(await screen.findByRole("dialog"));
    expect(dialog.getByText(/mesmo CPF/)).toBeInTheDocument();
    await userEvent.click(dialog.getByRole("button", { name: "Usar este cadastro" }));

    expect(await screen.findByText("Cadastro da Ana")).toBeInTheDocument();
    expect(api.createGuest).toHaveBeenCalledTimes(1);
  });

  it("edição carrega os dados formatados e envia a versão", async () => {
    await openEdit();

    expect(screen.getByLabelText(/Telefone/)).toHaveValue("(11) 99999-0000");
    expect(screen.getByLabelText(/CPF/)).toHaveValue("529.982.247-25");
    await userEvent.clear(screen.getByLabelText(/Nome completo/));
    await userEvent.type(screen.getByLabelText(/Nome completo/), "Ana Souza Lima");
    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));

    await waitFor(() =>
      expect(api.updateGuest).toHaveBeenCalledWith(
        "t",
        7,
        expect.objectContaining({ full_name: "Ana Souza Lima", cpf: "529.982.247-25" }),
        3,
      ),
    );
  });

  it("conflito de versão explica o problema e mantém o que foi digitado", async () => {
    vi.mocked(api.updateGuest).mockRejectedValue(
      new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."),
    );
    await openEdit();
    await userEvent.clear(screen.getByLabelText(/Nome completo/));
    await userEvent.type(screen.getByLabelText(/Nome completo/), "Ana Lima");

    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));

    expect(await screen.findByText(/foi alterado por outra operação/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Nome completo/)).toHaveValue("Ana Lima");
  });

  it("sair com alterações pede decisão", async () => {
    const { router } = openNew();
    await userEvent.type(screen.getByLabelText(/Nome completo/), "Ana");

    act(() => void router.navigate("/outra"));

    expect(await screen.findByText("Alterações não salvas")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));
    expect(screen.getByLabelText(/Nome completo/)).toHaveValue("Ana");
  });

  it("recepção não vê Desativar; administrador desativa com confirmação", async () => {
    signIn(recepcaoUser);
    await openEdit();
    expect(screen.queryByRole("button", { name: "Desativar hóspede" })).toBeNull();
  });

  it("administrador desativa com confirmação", async () => {
    await openEdit();

    await userEvent.click(screen.getByRole("button", { name: "Desativar hóspede" }));
    expect(api.setGuestActive).not.toHaveBeenCalled();
    await userEvent.click(await screen.findByRole("button", { name: "Confirmar desativação" }));

    await waitFor(() => expect(api.setGuestActive).toHaveBeenCalledWith("t", 7, false));
  });

  it("clique duplo em Criar outro cadastro cria só um hóspede", async () => {
    let release: (value: api.CreateGuestOutcome) => void = () => undefined;
    vi.mocked(api.createGuest)
      .mockResolvedValueOnce({ status: "duplicates", matches: [{ guest: ana, reasons: ["phone"] }] })
      .mockReturnValueOnce(new Promise<api.CreateGuestOutcome>((resolve) => (release = resolve)));
    openNew();
    await fillRequired();
    await userEvent.click(screen.getByRole("button", { name: "Salvar hóspede" }));
    const dialog = within(await screen.findByRole("dialog"));

    await userEvent.dblClick(dialog.getByRole("button", { name: "Criar outro cadastro" }));
    release({ status: "created", id: 8, code: "HOS-000008" });

    await waitFor(() => expect(api.createGuest).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Lista de hóspedes")).toBeInTheDocument();
  });
});
