import { vi } from "vitest";

import { useSessionStore } from "@/stores/session";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import { sessionStatus } from "./api";

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
  invoke.mockReset();
  useSessionStore.getState().clear();
});

describe("tratamento de erros do backend", () => {
  it('"unauthorized" com sessão aberta mantém a tela e marca a sessão como encerrada', async () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    invoke.mockRejectedValue({ code: "unauthorized", message: "É preciso entrar no sistema para continuar." });

    await expect(sessionStatus("t")).rejects.toMatchObject({ code: "unauthorized" });

    expect(useSessionStore.getState().state).toBe("expired");
    expect(useSessionStore.getState().user).toEqual(user);
  });

  it('"locked" bloqueia a sessão', async () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    invoke.mockRejectedValue({ code: "locked", message: "bloqueada" });

    await expect(sessionStatus("t")).rejects.toMatchObject({ code: "locked" });

    expect(useSessionStore.getState().state).toBe("locked");
  });

  it('"unauthorized" sem sessão continua anônimo', async () => {
    invoke.mockRejectedValue({ code: "unauthorized", message: "x" });

    await expect(sessionStatus("t")).rejects.toBeTruthy();

    expect(useSessionStore.getState().state).toBe("anonymous");
  });
});
