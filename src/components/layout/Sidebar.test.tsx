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

  it("recepção não vê Configurações", () => {
    signIn(recepcaoUser);
    renderSidebar();

    expect(screen.queryByText("Configurações")).toBeNull();
    expect(screen.getByRole("link", { name: "Dashboard" })).toBeInTheDocument();
  });
});
