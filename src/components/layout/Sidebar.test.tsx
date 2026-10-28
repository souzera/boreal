import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { adminUser, recepcaoUser, signIn } from "@/test/render";
import Sidebar from "./Sidebar";

function renderSidebar() {
  render(
    <MemoryRouter>
      <Sidebar />
    </MemoryRouter>,
  );
}

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

  it("recepção não vê Configurações", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.queryByText("Configurações")).toBeNull();
    expect(screen.getByRole("link", { name: "Dashboard" })).toBeInTheDocument();
  });
});
