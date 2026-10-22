import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import LockGate from "./LockGate";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, logout: vi.fn().mockResolvedValue(null), sessionStatus: vi.fn().mockResolvedValue({ state: "active", user: null }) };
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

beforeEach(() => {
  vi.mocked(api.logout).mockClear();
  useSessionStore.getState().clear();
  useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
});

describe("LockGate", () => {
  it("sem bloqueio mostra os filhos", () => {
    render(
      <LockGate>
        <p>Conteúdo</p>
      </LockGate>,
    );

    expect(screen.getByText("Conteúdo")).toBeVisible();
  });

  it("bloqueado oculta os filhos, mas os mantém montados, e pede a senha", async () => {
    render(
      <LockGate>
        <input aria-label="Rascunho" />
      </LockGate>,
    );
    await userEvent.type(screen.getByLabelText("Rascunho"), "texto");

    act(() => useSessionStore.getState().lock());
    expect(screen.getByLabelText("Rascunho", { selector: "input" })).not.toBeVisible();
    expect(screen.getByLabelText(/Senha/)).toBeVisible();

    act(() => useSessionStore.getState().unlock());
    expect(screen.getByLabelText("Rascunho")).toHaveValue("texto");
  });

  it('"Sair" na tela de bloqueio encerra a sessão também no backend', async () => {
    useSessionStore.getState().lock();
    render(
      <LockGate>
        <p>Conteúdo</p>
      </LockGate>,
    );

    await userEvent.click(screen.getByRole("button", { name: "Sair" }));

    await waitFor(() => expect(api.logout).toHaveBeenCalledWith("t"));
    expect(useSessionStore.getState().state).toBe("anonymous");
  });
});
