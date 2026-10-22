import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, MemoryRouter, Route, RouterProvider, Routes } from "react-router-dom";
import { vi } from "vitest";

import { UnsavedChangesGuard } from "@/components/UnsavedChangesGuard";
import * as api from "@/lib/api";
import Login from "@/pages/Login";
import { useSessionStore } from "@/stores/session";
import AppShell from "./AppShell";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, logout: vi.fn().mockResolvedValue(null), sessionStatus: vi.fn().mockResolvedValue({ state: "active", user: null }), getSessionPolicy: vi.fn().mockResolvedValue({ idle_lock_minutes: 10 }) };
});

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

  it('"Sair" com alterações não salvas pede decisão antes de encerrar a sessão', async () => {
    vi.mocked(api.logout).mockClear();
    const router = createMemoryRouter(
      [
        {
          element: <AppShell />,
          children: [{ path: "/", element: <UnsavedChangesGuard isDirty /> }],
        },
        { path: "/login", element: <Login /> },
      ],
      { initialEntries: ["/"] },
    );
    render(<RouterProvider router={router} />);

    await userEvent.click(screen.getByRole("button", { name: "Sair" }));
    expect(await screen.findByText("Alterações não salvas")).toBeInTheDocument();
    expect(api.logout).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));
    expect(useSessionStore.getState().state).toBe("active");

    await userEvent.click(screen.getByRole("button", { name: "Sair" }));
    await userEvent.click(await screen.findByRole("button", { name: "Descartar" }));

    await waitFor(() => expect(api.logout).toHaveBeenCalledWith("t"));
    await waitFor(() => expect(useSessionStore.getState().state).toBe("anonymous"));
    expect(await screen.findByRole("button", { name: "Entrar" })).toBeInTheDocument();
  });
});
