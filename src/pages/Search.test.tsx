import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLocation } from "react-router-dom";
import { vi } from "vitest";

import GlobalSearch from "@/components/layout/GlobalSearch";
import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import Search from "./Search";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listGuests: vi.fn() };
});

const ana: api.Guest = {
  id: 7,
  code: "HOS-000007",
  full_name: "Ana Souza",
  phone: "11999990000",
  cpf: null,
  email: "",
  address: "",
  active: true,
  version: 1,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.listGuests).mockReset().mockResolvedValue({ items: [ana], total: 1, page: 1, page_size: 10 });
});

function Where() {
  const location = useLocation();
  return <p>{`${location.pathname}${location.search}`}</p>;
}

describe("GlobalSearch", () => {
  it("enviar a busca leva a /busca com o texto", async () => {
    renderPage(<GlobalSearch />, { extraRoutes: [{ path: "/busca", element: <Where /> }] });

    await userEvent.type(screen.getByRole("searchbox", { name: "Busca geral" }), "  ana souza ");
    await userEvent.keyboard("{Enter}");

    expect(await screen.findByText("/busca?q=ana+souza")).toBeInTheDocument();
  });

  it("busca vazia não navega", async () => {
    renderPage(<GlobalSearch />, { extraRoutes: [{ path: "/busca", element: <Where /> }] });

    await userEvent.type(screen.getByRole("searchbox", { name: "Busca geral" }), "   {Enter}");

    expect(screen.queryByText(/\/busca/)).toBeNull();
  });
});

describe("Search page", () => {
  it("mostra hóspedes encontrados com link para o cadastro", async () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ana" });

    const link = await screen.findByRole("link", { name: /Ana Souza/ });

    expect(link).toHaveAttribute("href", "/hospedes/7");
    expect(screen.getByText("HOS-000007")).toBeInTheDocument();
    await waitFor(() =>
      expect(api.listGuests).toHaveBeenCalledWith("t", expect.objectContaining({ text: "ana" })),
    );
  });

  it("sem resultados orienta a pessoa", async () => {
    vi.mocked(api.listGuests).mockResolvedValue({ items: [], total: 0, page: 1, page_size: 10 });
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=zzz" });

    expect(await screen.findByText("Nenhum resultado para “zzz”.")).toBeInTheDocument();
  });

  it("sem texto explica o que dá para buscar", () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca" });

    expect(screen.getByText(/Digite um nome, telefone, CPF ou código/)).toBeInTheDocument();
    expect(api.listGuests).not.toHaveBeenCalled();
  });
});
