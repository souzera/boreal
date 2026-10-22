import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import Login from "./Login";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, login: vi.fn() };
});

const user = {
  id: 1,
  username: "maria",
  display_name: "Maria",
  role: "recepcao" as const,
  active: true,
  must_change_password: true,
  failed_attempts: 0,
  locked_until: null,
  version: 1,
};

function renderLogin() {
  return render(
    <MemoryRouter initialEntries={["/login"]}>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/change-password" element={<p>Trocar senha</p>} />
        <Route path="/" element={<p>Início</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  useSessionStore.getState().clear();
  vi.mocked(api.login).mockReset();
});

describe("Login", () => {
  it("valida campos vazios e marca os obrigatórios com asterisco e legenda", async () => {
    renderLogin();

    expect(screen.getByText("* campo obrigatório")).toBeInTheDocument();
    expect(screen.getByText(/Usuário/, { selector: "label" }).textContent).toContain("*");
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    expect(await screen.findByText("Informe o usuário.")).toBeInTheDocument();
    expect(screen.getByText("Informe a senha.")).toBeInTheDocument();
    expect(api.login).not.toHaveBeenCalled();
  });

  it("mostra a mensagem do backend para credenciais inválidas", async () => {
    vi.mocked(api.login).mockRejectedValue(
      new api.ApiError("invalid_credentials", "Usuário ou senha inválidos."),
    );
    renderLogin();

    await userEvent.type(screen.getByLabelText(/Usuário/), "maria");
    await userEvent.type(screen.getByLabelText(/Senha/), "errada-errada");
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    expect(await screen.findByText("Usuário ou senha inválidos")).toBeInTheDocument();
  });

  it("leva à troca de senha quando o backend exige", async () => {
    vi.mocked(api.login).mockResolvedValue({ token: "t", user, must_change_password: true });
    renderLogin();

    await userEvent.type(screen.getByLabelText(/Usuário/), "maria");
    await userEvent.type(screen.getByLabelText(/Senha/), "Temp-Senha-123");
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    await waitFor(() => expect(screen.getByText("Trocar senha")).toBeInTheDocument());
    expect(useSessionStore.getState().mustChangePassword).toBe(true);
  });
});
