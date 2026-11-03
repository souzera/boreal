import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage } from "@/test/render";
import FirstRun from "./FirstRun";
import RecoverFromBackup from "./RecoverFromBackup";
import StartupError from "./StartupError";

const open = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: (...args: unknown[]) => open(...args), save: vi.fn() }));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, restoreOnNewComputer: vi.fn(), recoverCorrupted: vi.fn(), unlockWithRecoveryKey: vi.fn() };
});

beforeEach(() => {
  open.mockReset().mockResolvedValue("F:\\pendrive\\boreal-2026-10-21-143000.bak");
  vi.mocked(api.restoreOnNewComputer).mockReset().mockResolvedValue(null);
  vi.mocked(api.recoverCorrupted).mockReset().mockResolvedValue(null);
  vi.mocked(api.unlockWithRecoveryKey).mockReset().mockResolvedValue(null);
});

const KEY = "00010203-04050607-08090A0B-0C0D0E0F-10111213-14151617-18191A1B-1C1D1E1F-ABCD";

describe("troca de computador", () => {
  it("o botão só habilita com o arquivo e a chave; restaura sem token", async () => {
    renderPage(<RecoverFromBackup mode="new-computer" />, { path: "/" });
    const action = screen.getByRole("button", { name: "Restaurar neste computador" });
    expect(action).toBeDisabled();

    await userEvent.click(screen.getByRole("button", { name: "Escolher arquivo de backup" }));
    expect(await screen.findByText("boreal-2026-10-21-143000.bak")).toBeInTheDocument();
    expect(action).toBeDisabled();
    await userEvent.type(screen.getByLabelText("Chave de recuperação"), KEY);
    expect(action).toBeEnabled();
    await userEvent.click(action);

    await waitFor(() => expect(api.restoreOnNewComputer).toHaveBeenCalledWith("F:\\pendrive\\boreal-2026-10-21-143000.bak", KEY));
  });

  it("chave inválida mostra o erro e deixa tentar de novo", async () => {
    vi.mocked(api.restoreOnNewComputer).mockRejectedValue({ code: "validation", message: "A chave de recuperação não abre este backup, ou o arquivo está danificado." });
    renderPage(<RecoverFromBackup mode="new-computer" />, { path: "/" });
    await userEvent.click(screen.getByRole("button", { name: "Escolher arquivo de backup" }));
    await screen.findByText("boreal-2026-10-21-143000.bak");
    await userEvent.type(screen.getByLabelText("Chave de recuperação"), "ABCD");

    await userEvent.click(screen.getByRole("button", { name: "Restaurar neste computador" }));

    expect(await screen.findByText(/não abre este backup/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Restaurar neste computador" })).toBeEnabled();
  });

  it("cancelar o seletor de arquivo não escolhe nada", async () => {
    open.mockResolvedValue(null);
    renderPage(<RecoverFromBackup mode="new-computer" />, { path: "/" });

    await userEvent.click(screen.getByRole("button", { name: "Escolher arquivo de backup" }));

    expect(screen.getByText("Nenhum arquivo escolhido.")).toBeInTheDocument();
  });

  it("a tela de primeiro acesso oferece Já tenho um backup e dá para voltar", async () => {
    renderPage(<FirstRun />, { path: "/" });

    await userEvent.click(screen.getByRole("button", { name: "Já tenho um backup" }));
    expect(screen.getByRole("heading", { name: "Restaurar de um backup" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Voltar" }));

    expect(screen.getByText("Configuração inicial")).toBeInTheDocument();
  });
});

describe("banco com problema e chave perdida", () => {
  it("integridade abre o modo de recuperação e exige a confirmação digitada", async () => {
    renderPage(<StartupError message="O banco de dados apresenta problemas" code="integrity" />, { path: "/" });
    expect(screen.getByRole("heading", { name: "Recuperar o banco de dados" })).toBeInTheDocument();
    const action = screen.getByRole("button", { name: "Restaurar o backup" });
    await userEvent.click(screen.getByRole("button", { name: "Escolher arquivo de backup" }));
    await screen.findByText("boreal-2026-10-21-143000.bak");
    await userEvent.type(screen.getByLabelText("Chave de recuperação"), KEY);
    expect(action).toBeDisabled();

    await userEvent.type(screen.getByLabelText(/Digite RESTAURAR/), "restaurar");
    await userEvent.click(action);

    await waitFor(() => expect(api.recoverCorrupted).toHaveBeenCalledWith("F:\\pendrive\\boreal-2026-10-21-143000.bak", KEY, "restaurar"));
  });

  it("chave indisponível oferece desbloquear com a chave de recuperação", async () => {
    renderPage(<StartupError message="A chave de criptografia do banco não está disponível" code="key_unavailable" />, { path: "/" });

    await userEvent.type(screen.getByLabelText("Chave de recuperação"), KEY);
    await userEvent.click(screen.getByRole("button", { name: "Desbloquear" }));

    await waitFor(() => expect(api.unlockWithRecoveryKey).toHaveBeenCalledWith(KEY));
    expect(screen.queryByRole("button", { name: "Escolher arquivo de backup" })).toBeNull();
  });

  it("falha de atualização explica que os dados estão preservados", () => {
    renderPage(<StartupError message="A atualização do banco de dados não pôde ser concluída." code="update_failed" />, { path: "/" });

    expect(screen.getByRole("alert")).toHaveTextContent("A atualização do banco de dados não pôde ser concluída.");
    expect(screen.getByText(/Seus dados estão preservados/)).toBeInTheDocument();
  });

  it("outros erros mantêm o texto de antes", () => {
    renderPage(<StartupError message="Algo falhou" />, { path: "/" });

    expect(screen.getByText(/Nenhum dado foi alterado/)).toBeInTheDocument();
  });
});
