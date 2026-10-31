import { screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { adminUser, recepcaoUser, renderPage, signIn } from "@/test/render";
import Sidebar from "./Sidebar";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, myPermissions: vi.fn() };
});

function renderSidebar() {
  renderPage(<Sidebar />);
}

beforeEach(() => {
  vi.mocked(api.myPermissions).mockReset().mockResolvedValue([]);
});

describe("Sidebar", () => {
  it("administrador vê Configurações como link", () => {
    signIn(adminUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Configurações" })).toHaveAttribute("href", "/configuracoes");
  });

  it("mostra Limpeza como link para a fila de limpeza", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Limpeza" })).toHaveAttribute("href", "/limpeza");
  });

  it("mostra Bloqueios e Manutenção como links", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Bloqueios" })).toHaveAttribute("href", "/bloqueios");
    expect(screen.getByRole("link", { name: "Manutenção" })).toHaveAttribute("href", "/manutencao");
  });

  it("mostra Fechamento de caixa para toda a equipe", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.getByRole("link", { name: "Fechamento de caixa" })).toHaveAttribute("href", "/fechamento");
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

  it("recepção não vê Configurações", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.queryByText("Configurações")).toBeNull();
    expect(screen.getByRole("link", { name: "Dashboard" })).toBeInTheDocument();
  });
});
