import { screen, waitFor, within } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { adminUser, recepcaoUser, renderPage, signIn } from "@/test/render";
import Sidebar from "./Sidebar";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, myPermissions: vi.fn() };
});

function renderSidebar(entry = "/") {
  renderPage(<Sidebar />, { path: "*", entry });
}

beforeEach(() => {
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
});

describe("Sidebar", () => {
  it("administrador vê Configurações como link", () => {
    signIn(adminUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Configurações" })).toHaveAttribute("href", "/configuracoes");
    expect(screen.getByRole("link", { name: "Usuários" })).toHaveAttribute("href", "/usuarios");
  });

  it("agrupa os itens com rótulo de grupo (que não é link)", () => {
    signIn(recepcaoUser);
    renderSidebar();

    for (const group of ["Reservas", "Quartos", "Financeiro"]) {
      expect(screen.getByText(group, { selector: "p" })).toBeInTheDocument();
    }
    const quartos = screen.getByRole("group", { name: "Quartos" });
    for (const [label, href] of [
      ["Mapa de quartos", "/mapa"],
      ["Limpeza", "/limpeza"],
      ["Bloqueios", "/bloqueios"],
      ["Manutenção", "/manutencao"],
      ["Cadastro de quartos", "/quartos"],
    ]) {
      expect(within(quartos).getByRole("link", { name: label })).toHaveAttribute("href", href);
    }
  });

  it("a tela inicial se chama Recepção", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Recepção" })).toHaveAttribute("href", "/");
    expect(screen.queryByRole("link", { name: "Dashboard" })).toBeNull();
  });

  it("mostra Fechamento de caixa para toda a equipe", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Fechamento de caixa" })).toHaveAttribute("href", "/fechamento");
  });

  it("o item da tela atual é marcado com aria-current", () => {
    signIn(recepcaoUser);
    renderSidebar("/limpeza");

    expect(screen.getByRole("link", { name: "Limpeza" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Recepção" })).not.toHaveAttribute("aria-current");
  });

  it("o item continua destacado nas sub-rotas, e a Recepção só na raiz", () => {
    signIn(recepcaoUser);
    renderSidebar("/reservas/nova");

    expect(screen.getByRole("link", { name: "Reservas" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Recepção" })).not.toHaveAttribute("aria-current");
  });

  it("Cadastro de quartos fica destacado em /quartos/categorias", () => {
    signIn(recepcaoUser);
    renderSidebar("/quartos/categorias");

    expect(screen.getByRole("link", { name: "Cadastro de quartos" })).toHaveAttribute("aria-current", "page");
  });

  it("todo item tem ícone", () => {
    signIn(adminUser);
    renderSidebar();

    for (const link of screen.getAllByRole("link")) expect(link.querySelector("[data-icon]")).not.toBeNull();
  });

  it("Relatórios e Alertas aparecem desabilitados, com 'Em breve'", () => {
    signIn(recepcaoUser);
    renderSidebar();

    for (const label of ["Relatórios", "Alertas"]) {
      const item = screen.getByText(label, { selector: "span" }).closest("[aria-disabled]") as HTMLElement;
      expect(item).toHaveAttribute("aria-disabled", "true");
      expect(item).toHaveAttribute("title", "Em breve");
      expect(screen.queryByRole("link", { name: label })).toBeNull();
    }
  });

  it("Contas a pagar aparece para quem tem a permissão", async () => {
    vi.mocked(api.myPermissions).mockResolvedValue(["manage_payables"]);
    signIn(recepcaoUser);
    renderSidebar();

    expect(await screen.findByRole("link", { name: "Contas a pagar" })).toHaveAttribute("href", "/contas-a-pagar");
  });

  it("Contas a pagar não aparece sem a permissão", async () => {
    signIn(recepcaoUser);
    renderSidebar();
    await waitFor(() => expect(api.myPermissions).toHaveBeenCalled());

    expect(screen.queryByRole("link", { name: "Contas a pagar" })).toBeNull();
  });

  it("recepção não vê Configurações nem Usuários, mas continua vendo Tarifas e Itens e serviços", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.queryByRole("link", { name: "Configurações" })).toBeNull();
    expect(screen.queryByRole("link", { name: "Usuários" })).toBeNull();
    expect(screen.getByRole("link", { name: "Tarifas" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Itens e serviços" })).toBeInTheDocument();
  });
});
