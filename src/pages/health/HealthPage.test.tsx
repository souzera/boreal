import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { recepcaoUser, renderPage, signIn } from "@/test/render";
import HealthPage from "./HealthPage";

const open = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: (...args: unknown[]) => open(...args), save: vi.fn() }));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    runBackup: vi.fn(),
    listBackupFiles: vi.fn(),
    recentBackupRuns: vi.fn(),
    backupDestination: vi.fn(),
    setBackupDestination: vi.fn(),
    previewRestore: vi.fn(),
    restoreBackup: vi.fn(),
    revealRecoveryKey: vi.fn(),
    diskStatus: vi.fn(),
    lastStartupReport: vi.fn(),
    logFolder: vi.fn(),
    getAppInfo: vi.fn(),
  };
});

const counts = (n: number) => ({ reservations: n, stays: n, payments: n, guests: n });

function preview(patch: Partial<api.RestorePreview> = {}): api.RestorePreview {
  return {
    file_name: "boreal-2026-10-21-143000.bak", backup_at: "21/10/2026 14:30", schema_version: 14, app_schema_version: 14,
    restorable: true, problem: null, counts: counts(3), current_counts: counts(8), loses_recent_data: true, ...patch,
  };
}

beforeEach(() => {
  signIn();
  open.mockReset();
  vi.mocked(api.backupDestination).mockReset().mockResolvedValue({ path: "D:\\Boreal\\backups", is_default: true });
  vi.mocked(api.recentBackupRuns).mockReset().mockResolvedValue([
    { id: 2, kind: "auto", kind_label: "Automático", status: "failed", finished_at: "2026-10-22T12:00:00Z", file_name: null, bytes: 0, message: "A pasta de backup não está acessível." },
    { id: 1, kind: "manual", kind_label: "Manual", status: "ok", finished_at: "2026-10-21T17:30:00Z", file_name: "boreal-2026-10-21-143000.bak", bytes: 2_097_152, message: "" },
  ]);
  vi.mocked(api.listBackupFiles).mockReset().mockResolvedValue([{ file_name: "boreal-2026-10-21-143000.bak", safety: false, at: "21/10/2026 14:30", bytes: 2_097_152 }]);
  vi.mocked(api.runBackup).mockReset();
  vi.mocked(api.setBackupDestination).mockReset().mockResolvedValue({ path: "E:/externo", is_default: false });
  vi.mocked(api.previewRestore).mockReset().mockResolvedValue(preview());
  vi.mocked(api.restoreBackup).mockReset().mockResolvedValue({ safety_copy: "boreal-seguranca-2026-10-22-090000.bak", backup_at: "21/10/2026 14:30" });
  vi.mocked(api.revealRecoveryKey).mockReset();
  vi.mocked(api.diskStatus).mockReset().mockResolvedValue({ free_bytes: 5 * 1024 ** 3, db_bytes: 10 * 1024 ** 2, threshold_bytes: 1024 ** 3, low: false });
  vi.mocked(api.lastStartupReport).mockReset().mockResolvedValue({ checked_at: "2026-10-22T11:00:00Z", integrity_ok: true, problems: [], app_version: "0.1.0", schema_version: 14 });
  vi.mocked(api.logFolder).mockReset().mockResolvedValue("C:\\Dados\\Boreal\\logs");
  vi.mocked(api.getAppInfo).mockReset().mockResolvedValue({ app_version: "0.1.0", schema_version: 14, last_migration_at: null, last_valid_backup_at: null });
});

const routes = [{ path: "/login", element: <p>Tela de entrada</p> }];
const show = () => renderPage(<HealthPage />, { path: "/saude", extraRoutes: routes });

describe("HealthPage", () => {
  it("a recepção vê o aviso e nada é consultado", () => {
    signIn(recepcaoUser);
    show();

    expect(screen.getByText("Esta área é só para administradores.")).toBeInTheDocument();
    expect(api.recentBackupRuns).not.toHaveBeenCalled();
    expect(api.diskStatus).not.toHaveBeenCalled();
  });

  it("o administrador vê as cinco áreas", async () => {
    show();

    for (const heading of ["Backup", "Chave de recuperação", "Espaço em disco", "Versões e verificação", "Últimos backups", "Restaurar um backup"]) {
      expect(await screen.findByRole("heading", { name: heading })).toBeInTheDocument();
    }
    expect(await screen.findByText("D:\\Boreal\\backups")).toBeInTheDocument();
    expect(screen.getByText(/padrão, ao lado do banco/)).toBeInTheDocument();
  });

  it("o histórico mostra a situação em texto e ícone e o motivo da falha", async () => {
    show();

    expect(await screen.findByText("Falhou")).toBeInTheDocument();
    expect(screen.getByText("A pasta de backup não está acessível.")).toBeInTheDocument();
    expect(screen.getByText("Válido")).toBeInTheDocument();
    expect(screen.getByText("2 MB")).toBeInTheDocument();
  });

  it("Fazer backup agora chama o backup e mostra o resultado", async () => {
    vi.mocked(api.runBackup).mockResolvedValue({ id: 3, kind: "manual", kind_label: "Manual", status: "ok", finished_at: "2026-10-22T12:00:00Z", file_name: "boreal-2026-10-22-090000.bak", bytes: 10, message: "" });
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Fazer backup agora" }));

    expect(await screen.findByText("Backup concluído: boreal-2026-10-22-090000.bak")).toBeInTheDocument();
    expect(api.runBackup).toHaveBeenCalledWith("t");
  });

  it("falha no backup mostra a mensagem do sistema e a tela continua utilizável", async () => {
    vi.mocked(api.runBackup).mockRejectedValue({ code: "validation", message: "Não há espaço suficiente na pasta de backup." });
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Fazer backup agora" }));

    expect(await screen.findByText("Não há espaço suficiente na pasta de backup.", { selector: "p[role='alert']" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Fazer backup agora" })).toBeEnabled();
  });

  it("Escolher pasta usa o seletor e grava o destino; cancelar não faz nada", async () => {
    open.mockResolvedValueOnce(null).mockResolvedValueOnce("E:/externo");
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Escolher pasta" }));
    expect(api.setBackupDestination).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));

    await waitFor(() => expect(api.setBackupDestination).toHaveBeenCalledWith("t", "E:/externo"));
    expect(await screen.findByText("Pasta de backup alterada.")).toBeInTheDocument();
  });

  it("restaurar: prévia em texto, botão só com RESTAURAR e senha, depois volta ao login", async () => {
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Restaurar o backup de 21/10/2026 14:30" }));

    const dialog = await screen.findByRole("dialog", { name: "Restaurar um backup" });
    expect(within(dialog).getByText("21/10/2026 14:30")).toBeInTheDocument();
    expect(within(dialog).getByText("3 reservas, 3 hospedagens, 3 recebimentos, 3 hóspedes")).toBeInTheDocument();
    expect(within(dialog).getByText("8 reservas, 8 hospedagens, 8 recebimentos, 8 hóspedes")).toBeInTheDocument();
    expect(within(dialog).getByText(/Tudo o que foi feito depois dele será perdido/)).toBeInTheDocument();
    const confirm = within(dialog).getByRole("button", { name: "Restaurar este backup" });
    expect(confirm).toBeDisabled();

    await userEvent.type(within(dialog).getByLabelText(/Digite RESTAURAR/), "restaurar");
    expect(confirm).toBeDisabled();
    await userEvent.type(within(dialog).getByLabelText("Sua senha"), "Admin-Senha-1");
    expect(confirm).toBeEnabled();
    await userEvent.click(confirm);

    await waitFor(() => expect(api.restoreBackup).toHaveBeenCalledWith("t", "D:\\Boreal\\backups\\boreal-2026-10-21-143000.bak", "restaurar", "Admin-Senha-1"));
    expect(await screen.findByText("Tela de entrada")).toBeInTheDocument();
    expect(useSessionStore.getState().token).toBeNull();
  });

  it("senha errada mostra o erro, mantém o diálogo e limpa a senha", async () => {
    vi.mocked(api.restoreBackup).mockRejectedValue({ code: "validation", message: "A senha está incorreta." });
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Restaurar o backup de 21/10/2026 14:30" }));
    const dialog = await screen.findByRole("dialog", { name: "Restaurar um backup" });
    await userEvent.type(within(dialog).getByLabelText(/Digite RESTAURAR/), "RESTAURAR");
    await userEvent.type(within(dialog).getByLabelText("Sua senha"), "errada");

    await userEvent.click(within(dialog).getByRole("button", { name: "Restaurar este backup" }));

    expect(await within(dialog).findByText("A senha está incorreta.")).toBeInTheDocument();
    expect(within(dialog).getByLabelText("Sua senha")).toHaveValue("");
    expect(screen.queryByText("Tela de entrada")).toBeNull();
  });

  it("backup que não pode ser restaurado mostra o motivo e não oferece a confirmação", async () => {
    vi.mocked(api.previewRestore).mockResolvedValue(preview({ restorable: false, problem: "Este backup foi feito por uma versão mais nova do Boreal.", counts: counts(0) }));
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Restaurar o backup de 21/10/2026 14:30" }));

    const dialog = await screen.findByRole("dialog", { name: "Restaurar um backup" });

    expect(await within(dialog).findByText(/versão mais nova do Boreal/)).toBeInTheDocument();
    expect(within(dialog).queryByLabelText("Sua senha")).toBeNull();
    expect(within(dialog).getByRole("button", { name: "Restaurar este backup" })).toBeDisabled();
  });

  it("cancelar o diálogo não restaura nada", async () => {
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Restaurar o backup de 21/10/2026 14:30" }));
    const dialog = await screen.findByRole("dialog", { name: "Restaurar um backup" });

    await userEvent.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(api.restoreBackup).not.toHaveBeenCalled();
  });

  it("Escolher outro arquivo abre a prévia do arquivo escolhido", async () => {
    open.mockResolvedValue("F:/pendrive/boreal-2026-10-20-180000.bak");
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Escolher outro arquivo" }));

    await waitFor(() => expect(api.previewRestore).toHaveBeenCalledWith("t", "F:/pendrive/boreal-2026-10-20-180000.bak"));
  });
});

describe("chave de recuperação", () => {
  it("pede a senha antes e depois mostra os 9 grupos; Ocultar esconde", async () => {
    vi.mocked(api.revealRecoveryKey).mockResolvedValue("00010203-04050607-08090A0B-0C0D0E0F-10111213-14151617-18191A1B-1C1D1E1F-ABCD");
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Mostrar chave de recuperação" }));
    expect(screen.queryByRole("list", { name: "Chave de recuperação" })).toBeNull();

    await userEvent.type(screen.getByLabelText("Digite sua senha para ver a chave"), "Admin-Senha-1");
    await userEvent.click(screen.getByRole("button", { name: "Mostrar" }));

    const groups = await screen.findByRole("list", { name: "Chave de recuperação" });
    expect(within(groups).getAllByRole("listitem")).toHaveLength(9);
    expect(within(groups).getByText("ABCD")).toBeInTheDocument();
    expect(api.revealRecoveryKey).toHaveBeenCalledWith("t", "Admin-Senha-1");
    await userEvent.click(screen.getByRole("button", { name: "Ocultar chave" }));
    expect(screen.queryByRole("list", { name: "Chave de recuperação" })).toBeNull();
  });

  it("senha errada não mostra a chave e limpa o campo", async () => {
    vi.mocked(api.revealRecoveryKey).mockRejectedValue({ code: "validation", message: "A senha está incorreta." });
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Mostrar chave de recuperação" }));
    await userEvent.type(screen.getByLabelText("Digite sua senha para ver a chave"), "errada");

    await userEvent.click(screen.getByRole("button", { name: "Mostrar" }));

    expect(await screen.findByText("A senha está incorreta.")).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Chave de recuperação" })).toBeNull();
    expect(screen.getByLabelText("Digite sua senha para ver a chave")).toHaveValue("");
  });

  it("Imprimir chama a impressão do navegador", async () => {
    const print = vi.spyOn(window, "print").mockImplementation(() => {});
    vi.mocked(api.revealRecoveryKey).mockResolvedValue("AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD-EEEEEEEE-FFFFFFFF-00000000-11111111-2222");
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Mostrar chave de recuperação" }));
    await userEvent.type(screen.getByLabelText("Digite sua senha para ver a chave"), "x");
    await userEvent.click(screen.getByRole("button", { name: "Mostrar" }));

    await userEvent.click(await screen.findByRole("button", { name: "Imprimir" }));

    expect(print).toHaveBeenCalledTimes(1);
    print.mockRestore();
  });
});

describe("espaço em disco e versões", () => {
  it("espaço suficiente aparece em texto", async () => {
    show();

    expect(await screen.findByText("Espaço suficiente")).toBeInTheDocument();
    expect(screen.getByText("Livre: 5,0 GB. O recomendado é pelo menos 1,0 GB.")).toBeInTheDocument();
  });

  it("pouco espaço aparece com texto e orientação", async () => {
    vi.mocked(api.diskStatus).mockResolvedValue({ free_bytes: 300 * 1024 ** 2, db_bytes: 10 * 1024 ** 2, threshold_bytes: 1024 ** 3, low: true });
    show();

    expect(await screen.findByText("Pouco espaço em disco")).toBeInTheDocument();
    expect(screen.getByText(/Libere espaço no computador/)).toBeInTheDocument();
  });

  it("mostra as versões, a verificação ao abrir e a pasta dos logs", async () => {
    show();

    expect(await screen.findByText(/Verificação ao abrir o sistema: sem problemas/)).toBeInTheDocument();
    expect(await screen.findByText("C:\\Dados\\Boreal\\logs")).toBeInTheDocument();
    expect(await screen.findByText("0.1.0")).toBeInTheDocument();
  });

  it("problemas de integridade aparecem em texto", async () => {
    vi.mocked(api.lastStartupReport).mockResolvedValue({ checked_at: "2026-10-22T11:00:00Z", integrity_ok: false, problems: ["a", "b"], app_version: "0.1.0", schema_version: 14 });
    show();

    expect(await screen.findByText("Verificação ao abrir o sistema: 2 problema(s) encontrado(s).")).toBeInTheDocument();
  });
});
