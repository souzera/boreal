import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import ChangePassword from "./ChangePassword";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, changeOwnPassword: vi.fn() };
});

const user = {
  id: 2,
  username: "maria",
  display_name: "Maria",
  role: "recepcao" as const,
  active: true,
  must_change_password: true,
  failed_attempts: 0,
  locked_until: null,
  version: 1,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/change-password"]}>
      <Routes>
        <Route path="/change-password" element={<ChangePassword />} />
        <Route path="/" element={<p>Início</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(api.changeOwnPassword).mockReset();
  useSessionStore.getState().clear();
  useSessionStore.getState().setSession({ token: "t", user, must_change_password: true });
});

describe("ChangePassword", () => {
  it("recusa confirmação diferente da nova senha", async () => {
    renderPage();

    await userEvent.type(screen.getByLabelText(/Senha atual/), "Temp-Senha-123");
    await userEvent.type(screen.getByLabelText(/^Nova senha/), "Definitiva-456");
    await userEvent.type(screen.getByLabelText(/Confirmar nova senha/), "Outra-coisa-789");
    await userEvent.click(screen.getByRole("button", { name: "Salvar nova senha" }));

    expect(await screen.findByText("As senhas não conferem.")).toBeInTheDocument();
    expect(api.changeOwnPassword).not.toHaveBeenCalled();
  });

  it("exige no mínimo 6 caracteres na nova senha", async () => {
    renderPage();

    await userEvent.type(screen.getByLabelText(/Senha atual/), "Temp-Senha-123");
    await userEvent.type(screen.getByLabelText(/^Nova senha/), "12345");
    await userEvent.type(screen.getByLabelText(/Confirmar nova senha/), "12345");
    await userEvent.click(screen.getByRole("button", { name: "Salvar nova senha" }));

    expect(await screen.findByText("A senha deve ter pelo menos 6 caracteres.")).toBeInTheDocument();
    expect(api.changeOwnPassword).not.toHaveBeenCalled();
  });

  it("troca a senha, limpa a exigência e vai ao início", async () => {
    vi.mocked(api.changeOwnPassword).mockResolvedValue(null);
    renderPage();

    await userEvent.type(screen.getByLabelText(/Senha atual/), "Temp-Senha-123");
    await userEvent.type(screen.getByLabelText(/^Nova senha/), "Definitiva-456");
    await userEvent.type(screen.getByLabelText(/Confirmar nova senha/), "Definitiva-456");
    await userEvent.click(screen.getByRole("button", { name: "Salvar nova senha" }));

    await waitFor(() => expect(screen.getByText("Início")).toBeInTheDocument());
    expect(api.changeOwnPassword).toHaveBeenCalledWith("t", "Temp-Senha-123", "Definitiva-456");
    expect(useSessionStore.getState().mustChangePassword).toBe(false);
  });
});
