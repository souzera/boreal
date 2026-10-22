import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import LockGate from "./LockGate";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, login: vi.fn(), logout: vi.fn().mockResolvedValue(null), sessionStatus: vi.fn().mockResolvedValue({ state: "active", user: null }),
    getSessionPolicy: vi.fn().mockResolvedValue({ idle_lock_minutes: 1 }),
  };
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
  it("trava pelo tempo de inatividade vindo do backend", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      render(
        <LockGate>
          <p>Conteúdo</p>
        </LockGate>,
      );
      await act(async () => {
        await Promise.resolve();
      });

      await act(async () => {
        vi.advanceTimersByTime(61_000);
      });

      expect(useSessionStore.getState().state).toBe("locked");
    } finally {
      vi.useRealTimers();
    }
  });

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

  it("diálogo aberto não vaza para a tela de bloqueio e volta depois de desbloquear", () => {
    render(
      <LockGate>
        <ConfirmDialog
          open
          title="Desativar hóspede?"
          description="Dados do cadastro"
          confirmLabel="Confirmar"
          onConfirm={() => undefined}
          onCancel={() => undefined}
        />
      </LockGate>,
    );
    expect(screen.getByText("Desativar hóspede?")).toBeVisible();

    act(() => useSessionStore.getState().lock());

    expect(screen.queryByText("Desativar hóspede?")).toBeNull();
    expect(screen.queryByText("Dados do cadastro")).toBeNull();
    expect(screen.getByLabelText(/Senha/)).toBeVisible();

    act(() => useSessionStore.getState().unlock());
    expect(screen.getByText("Desativar hóspede?")).toBeVisible();
  });

  describe("sessão encerrada (expired)", () => {
    function renderWithDraft() {
      render(
        <LockGate>
          <input aria-label="Rascunho" />
        </LockGate>,
      );
    }

    it("mantém a página montada, oculta, e pede para entrar de novo", async () => {
      renderWithDraft();
      await userEvent.type(screen.getByLabelText("Rascunho"), "meu texto");

      act(() => useSessionStore.getState().expire());

      expect(screen.getByLabelText("Rascunho", { selector: "input" })).not.toBeVisible();
      expect(screen.getByText(/Sua sessão foi encerrada/)).toBeVisible();
      expect(screen.getByLabelText(/Usuário/)).toHaveValue("admin");

      vi.mocked(api.login).mockResolvedValue({ token: "t2", user, must_change_password: false });
      await userEvent.type(screen.getByLabelText(/Senha/), "Admin-Senha-1");
      await userEvent.click(screen.getByRole("button", { name: "Entrar novamente" }));

      await waitFor(() => expect(useSessionStore.getState().state).toBe("active"));
      expect(useSessionStore.getState().token).toBe("t2");
      expect(api.login).toHaveBeenCalledWith("admin", "Admin-Senha-1");
      expect(screen.getByLabelText("Rascunho")).toHaveValue("meu texto");
    });

    it("senha errada mostra o erro e continua pedindo", async () => {
      renderWithDraft();
      act(() => useSessionStore.getState().expire());
      vi.mocked(api.login).mockRejectedValue(new api.ApiError("invalid_credentials", "Usuário ou senha inválidos."));

      await userEvent.type(screen.getByLabelText(/Senha/), "errada-errada");
      await userEvent.click(screen.getByRole("button", { name: "Entrar novamente" }));

      expect(await screen.findByText("Usuário ou senha inválidos.")).toBeInTheDocument();
      expect(useSessionStore.getState().state).toBe("expired");
    });

    it('"Sair sem salvar" encerra de vez', async () => {
      renderWithDraft();
      act(() => useSessionStore.getState().expire());

      await userEvent.click(screen.getByRole("button", { name: "Sair sem salvar" }));

      expect(useSessionStore.getState().state).toBe("anonymous");
    });
  });
});
