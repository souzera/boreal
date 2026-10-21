import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

    expect(screen.getByText("Conteúdo do dashboard")).not.toBeVisible();
    expect(screen.getByRole("navigation", { name: "Menu principal", hidden: true })).not.toBeVisible();
    expect(screen.getByLabelText(/Senha/)).toBeVisible();
  });

  it("bloquear e desbloquear não descarta o que foi digitado na página", async () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <Routes>
          <Route element={<AppShell />}>
            <Route path="/" element={<input aria-label="Observação" />} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    await userEvent.type(screen.getByLabelText("Observação"), "rascunho");

    act(() => useSessionStore.getState().lock());
    expect(screen.getByLabelText("Observação", { selector: "input" })).not.toBeVisible();
    act(() => useSessionStore.getState().unlock());

    expect(screen.getByLabelText("Observação")).toHaveValue("rascunho");
  });
});
