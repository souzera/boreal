import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";

import { useSessionStore } from "@/stores/session";
import AppShell from "./AppShell";

const user = {
  id: 1,
  username: "admin",
  display_name: "Administrador",
  role: "admin" as const,
  active: true,
  must_change_password: false,
  failed_attempts: 0,
  locked_until: null,
  version: 1,
};

function renderShell() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<p>Conteúdo do dashboard</p>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  useSessionStore.getState().clear();
  useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
});

describe("AppShell", () => {
  it("com sessão ativa mostra o menu lateral e o conteúdo", () => {
    renderShell();

    expect(screen.getByRole("navigation", { name: "Menu principal" })).toBeInTheDocument();
    expect(screen.getByText("Conteúdo do dashboard")).toBeInTheDocument();
    expect(screen.getByText("Administrador")).toBeInTheDocument();
  });

  it("com sessão bloqueada oculta o conteúdo e pede a senha", () => {
    useSessionStore.getState().lock();
    renderShell();

    expect(screen.queryByText("Conteúdo do dashboard")).not.toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Menu principal" })).not.toBeInTheDocument();
    expect(screen.getByLabelText(/Senha/)).toBeInTheDocument();
  });
});
