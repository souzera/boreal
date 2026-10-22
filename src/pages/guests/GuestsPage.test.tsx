import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import GuestsPage from "./GuestsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listGuests: vi.fn() };
});

const ana: api.Guest = {
  id: 7,
  code: "HOS-000007",
  full_name: "Ana Souza",
  phone: "11999990000",
  cpf: "52998224725",
  email: "",
  address: "",
  active: true,
  version: 3,
};
const inactive: api.Guest = { ...ana, id: 8, code: "HOS-000008", full_name: "Bia Lima", cpf: null, active: false };

beforeEach(() => {
  signIn();
  vi.mocked(api.listGuests)
    .mockReset()
    .mockResolvedValue({ items: [ana, inactive], total: 2, page: 1, page_size: 20 });
});

const routes = [
  { path: "/hospedes/novo", element: <p>Novo cadastro</p> },
  { path: "/hospedes/:id", element: <p>Cadastro aberto</p> },
];

describe("GuestsPage", () => {
  it("lista hóspedes com código, telefone e CPF formatados e situação em texto", async () => {
    renderPage(<GuestsPage />, { path: "/hospedes", extraRoutes: routes });

    const row = within((await screen.findByText("Ana Souza")).closest("tr") as HTMLElement);
    expect(row.getByText("HOS-000007")).toBeInTheDocument();
    expect(row.getByText("(11) 99999-0000")).toBeInTheDocument();
    expect(row.getByText("529.982.247-25")).toBeInTheDocument();
    expect(row.getByText("Ativo")).toBeInTheDocument();
    expect(within(screen.getByText("Bia Lima").closest("tr") as HTMLElement).getByText("Inativo")).toBeInTheDocument();
  });

  it("busca envia o texto ao backend", async () => {
    renderPage(<GuestsPage />, { path: "/hospedes", extraRoutes: routes });
    await screen.findByText("Ana Souza");

    await userEvent.type(screen.getByLabelText("Buscar hóspede"), "jose");

    await waitFor(() =>
      expect(api.listGuests).toHaveBeenLastCalledWith("t", expect.objectContaining({ text: "jose", page: 1 })),
    );
  });

  it("Novo hóspede e clicar numa linha navegam", async () => {
    renderPage(<GuestsPage />, { path: "/hospedes", extraRoutes: routes });
    await screen.findByText("Ana Souza");

    await userEvent.click(screen.getByRole("link", { name: "Novo hóspede" }));
    expect(await screen.findByText("Novo cadastro")).toBeInTheDocument();
  });

  it("clicar na linha abre o cadastro", async () => {
    renderPage(<GuestsPage />, { path: "/hospedes", extraRoutes: routes });

    await userEvent.click(await screen.findByText("Ana Souza"));

    expect(await screen.findByText("Cadastro aberto")).toBeInTheDocument();
  });

  it("sem resultados orienta a pessoa", async () => {
    vi.mocked(api.listGuests).mockResolvedValue({ items: [], total: 0, page: 1, page_size: 20 });
    renderPage(<GuestsPage />, { path: "/hospedes", extraRoutes: routes });

    expect(await screen.findByText("Nenhum hóspede encontrado.")).toBeInTheDocument();
  });
});
