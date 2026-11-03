import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { renderPage, signIn } from "@/test/render";
import SettingsPage from "./SettingsPage";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    getPousada: vi.fn(),
    getPousadaLogo: vi.fn(),
    updatePousada: vi.fn(),
    getAppInfo: vi.fn(),
    getCancellationPolicy: vi.fn(),
    updateCancellationPolicy: vi.fn(),
  };
});

const settings: api.AppSettings = {
  max_failed_attempts: 5,
  lockout_minutes: 5,
  idle_lock_minutes: 10,
  default_check_in_time: "14:00",
  default_check_out_time: "12:00",
  late_checkout_tolerance_minutes: 30,
  no_show_tolerance_minutes: 60,
  cleaning_interval_minutes: 60,
  deposit_percent_bp: 3000,
  deposit_due_days_after_booking: 3,
  deposit_due_days_before_arrival: 1,
  late_checkout_fee_bp: 5000,
};

beforeEach(() => {
  signIn();
  vi.mocked(api.getSettings).mockResolvedValue({ settings, version: 1 });
  vi.mocked(api.updateSettings).mockReset().mockResolvedValue(2);
  vi.mocked(api.getPousada).mockResolvedValue({ name: "", address: "", phone: "", has_logo: false, version: 1 });
  vi.mocked(api.getPousadaLogo).mockResolvedValue(null);
  vi.mocked(api.updatePousada).mockReset().mockResolvedValue(2);
  vi.mocked(api.getCancellationPolicy).mockReset().mockResolvedValue({
    tiers: [
      { min_days_before: 7, refund_bp: 10000 },
      { min_days_before: 3, refund_bp: 5000 },
      { min_days_before: 0, refund_bp: 0 },
    ],
    version: 1,
  });
  vi.mocked(api.updateCancellationPolicy).mockReset().mockResolvedValue(2);
  vi.mocked(api.getAppInfo).mockResolvedValue({
    app_version: "0.1.0",
    schema_version: 3,
    last_migration_at: "2026-10-21T12:00:00Z",
  });
});

async function open() {
  const view = renderPage(<SettingsPage />);
  await waitFor(() => expect(screen.getByLabelText(/Minutos de inatividade/)).toHaveValue(10));
  await waitFor(() => expect(screen.getByRole("button", { name: "Salvar dados da pousada" })).toBeEnabled());
  return view;
}

describe("SettingsPage", () => {
  it("mostra versão do aplicativo, do banco e a data da última atualização", async () => {
    await open();

    expect(await screen.findByText("0.1.0")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("21/10/2026")).toBeInTheDocument();
  });

  it("carrega os valores atuais, com * e legenda de obrigatório", async () => {
    await open();

    expect(screen.getByLabelText(/Minutos de inatividade/)).toHaveValue(10);
    expect(screen.getByLabelText(/Horário padrão de check-in/)).toHaveValue("14:00");
    expect(screen.getAllByText("* campo obrigatório").length).toBeGreaterThan(0);
  });

  it("recusa ociosidade 0 sem chamar o backend", async () => {
    await open();
    const field = screen.getByLabelText(/Minutos de inatividade/);

    await userEvent.clear(field);
    await userEvent.type(field, "0");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    expect(await screen.findByText("Informe de 1 a 480 minutos.")).toBeInTheDocument();
    expect(api.updateSettings).not.toHaveBeenCalled();
  });

  it("recusa horário inválido", async () => {
    await open();
    const field = screen.getByLabelText(/Horário padrão de check-in/);

    await userEvent.clear(field);
    await userEvent.type(field, "25:00");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    expect(await screen.findByText("Use o formato HH:MM, por exemplo 14:00.")).toBeInTheDocument();
    expect(api.updateSettings).not.toHaveBeenCalled();
  });

  it("salva e confirma o resultado", async () => {
    await open();
    const field = screen.getByLabelText(/Minutos de inatividade/);

    await userEvent.clear(field);
    await userEvent.type(field, "15");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    expect(await screen.findByText("Configurações salvas.")).toBeInTheDocument();
    expect(api.updateSettings).toHaveBeenCalledWith("t", { ...settings, idle_lock_minutes: 15 }, 1);
  });

  it("conflito de versão explica e mantém o que foi digitado", async () => {
    vi.mocked(api.updateSettings).mockRejectedValue(
      new api.ApiError("conflict", "O registro foi alterado por outra operação. Revise e tente novamente."),
    );
    await open();
    const field = screen.getByLabelText(/Minutos de inatividade/);
    await userEvent.clear(field);
    await userEvent.type(field, "15");

    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    expect(await screen.findByText(/foi alterado por outra operação/)).toBeInTheDocument();
    expect(field).toHaveValue(15);
  });

  it("nome da pousada é obrigatório; salva mantendo o logotipo", async () => {
    await open();

    await userEvent.click(screen.getByRole("button", { name: "Salvar dados da pousada" }));
    expect(await screen.findByText("Informe o nome da pousada.")).toBeInTheDocument();
    expect(api.updatePousada).not.toHaveBeenCalled();

    await userEvent.type(screen.getByLabelText(/Nome da pousada/), "Pousada Boreal");
    await userEvent.click(screen.getByRole("button", { name: "Salvar dados da pousada" }));

    expect(await screen.findByText("Dados da pousada salvos.")).toBeInTheDocument();
    expect(api.updatePousada).toHaveBeenCalledWith(
      "t",
      { name: "Pousada Boreal", address: "", phone: "", logo: { kind: "keep" } },
      1,
    );
  });

  it("recusa arquivo de logotipo que não é PNG/JPEG", async () => {
    await open();
    const file = new File(["texto"], "logo.txt", { type: "text/plain" });

    await userEvent.upload(screen.getByLabelText(/Logotipo/), file, { applyAccept: false });

    expect(await screen.findByText("Escolha uma imagem PNG ou JPEG de até 512 KB.")).toBeInTheDocument();
  });

  it("sair com formulário alterado pede decisão", async () => {
    const { router } = await (async () => {
      const view = await open();
      return view;
    })();
    await userEvent.type(screen.getByLabelText(/Nome da pousada/), "Boreal");

    act(() => void router.navigate("/outra"));

    expect(await screen.findByText("Alterações não salvas")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));
    await waitFor(() => expect(screen.queryByText("Alterações não salvas")).toBeNull());
  });

  it("depois de salvar, a interface passa a usar o novo tempo de inatividade", async () => {
    await open();
    const field = screen.getByLabelText(/Minutos de inatividade/);

    await userEvent.clear(field);
    await userEvent.type(field, "3");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    await screen.findByText("Configurações salvas.");
    expect(useSessionStore.getState().idleMinutes).toBe(3);
  });

  it("mostra o sinal em percentual e salva em pontos-base", async () => {
    await open();
    const field = screen.getByLabelText(/Sinal \(% do total/);
    expect(field).toHaveValue(30);

    await userEvent.clear(field);
    await userEvent.type(field, "25.5");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    await waitFor(() =>
      expect(api.updateSettings).toHaveBeenCalledWith("t", { ...settings, deposit_percent_bp: 2550 }, 1),
    );
  });

  it("mostra a cobrança de saída tardia em percentual e salva em pontos-base", async () => {
    await open();
    const field = screen.getByLabelText(/Cobrança de saída tardia/);
    expect(field).toHaveValue(50);

    await userEvent.clear(field);
    await userEvent.type(field, "30");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    await waitFor(() =>
      expect(api.updateSettings).toHaveBeenCalledWith("t", { ...settings, late_checkout_fee_bp: 3000 }, 1),
    );
  });

  it("valida o sinal e os prazos sem chamar o backend", async () => {
    await open();
    const percent = screen.getByLabelText(/Sinal \(% do total/);
    const after = screen.getByLabelText(/Sinal vence em até/);

    await userEvent.clear(percent);
    await userEvent.type(percent, "150");
    await userEvent.clear(after);
    await userEvent.type(after, "61");
    await userEvent.click(screen.getByRole("button", { name: "Salvar configurações" }));

    expect(await screen.findByText("Informe de 0 a 100%.")).toBeInTheDocument();
    expect(screen.getByText("Informe de 0 a 60 dias.")).toBeInTheDocument();
    expect(api.updateSettings).not.toHaveBeenCalled();
  });

  describe("política de cancelamento", () => {
    async function openPolicy() {
      await open();
      await screen.findByLabelText("Devolução do sinal (%), faixa 2");
    }

    it("mostra as faixas e avisa que os valores são provisórios", async () => {
      await openPolicy();

      expect(screen.getByLabelText("Dias de antecedência, faixa 1")).toHaveValue(7);
      expect(screen.getByLabelText("Devolução do sinal (%), faixa 1")).toHaveValue(100);
      expect(screen.getByLabelText("Devolução do sinal (%), faixa 2")).toHaveValue(50);
      expect(screen.getByText(/valores provisórios/i)).toBeInTheDocument();
    });

    it("salva uma faixa alterada em pontos-base com a versão", async () => {
      await openPolicy();
      const field = screen.getByLabelText("Devolução do sinal (%), faixa 2");

      await userEvent.clear(field);
      await userEvent.type(field, "40");
      await userEvent.click(screen.getByRole("button", { name: "Salvar política" }));

      await waitFor(() =>
        expect(api.updateCancellationPolicy).toHaveBeenCalledWith(
          "t",
          [
            { min_days_before: 7, refund_bp: 10000 },
            { min_days_before: 3, refund_bp: 4000 },
            { min_days_before: 0, refund_bp: 0 },
          ],
          1,
        ),
      );
      expect(await screen.findByText("Política de cancelamento salva.")).toBeInTheDocument();
    });

    it("adiciona e remove faixas", async () => {
      await openPolicy();

      await userEvent.click(screen.getByRole("button", { name: "Adicionar faixa" }));
      await userEvent.type(screen.getByLabelText("Dias de antecedência, faixa 4"), "15");
      await userEvent.type(screen.getByLabelText("Devolução do sinal (%), faixa 4"), "100");
      await userEvent.click(screen.getByRole("button", { name: "Remover faixa 2" }));
      await userEvent.click(screen.getByRole("button", { name: "Salvar política" }));

      await waitFor(() =>
        expect(api.updateCancellationPolicy).toHaveBeenCalledWith(
          "t",
          [
            { min_days_before: 7, refund_bp: 10000 },
            { min_days_before: 0, refund_bp: 0 },
            { min_days_before: 15, refund_bp: 10000 },
          ],
          1,
        ),
      );
    });

    it("recusa política sem a faixa de 0 dia ou com devolução acima de 100%", async () => {
      await openPolicy();

      await userEvent.click(screen.getByRole("button", { name: "Remover faixa 3" }));
      await userEvent.click(screen.getByRole("button", { name: "Salvar política" }));
      expect(await screen.findByText("Inclua a faixa de 0 dia.")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Adicionar faixa" }));
      await userEvent.type(screen.getByLabelText("Dias de antecedência, faixa 3"), "0");
      await userEvent.type(screen.getByLabelText("Devolução do sinal (%), faixa 3"), "120");
      await userEvent.click(screen.getByRole("button", { name: "Salvar política" }));
      expect(await screen.findByText("A devolução vai de 0% a 100%.")).toBeInTheDocument();
      expect(api.updateCancellationPolicy).not.toHaveBeenCalled();
    });

    it("mostra o erro do backend e mantém as faixas digitadas", async () => {
      vi.mocked(api.updateCancellationPolicy).mockRejectedValue(
        new api.ApiError("validation", "A devolução não pode aumentar quando a antecedência diminui."),
      );
      await openPolicy();
      const field = screen.getByLabelText("Devolução do sinal (%), faixa 3");
      await userEvent.clear(field);
      await userEvent.type(field, "80");

      await userEvent.click(screen.getByRole("button", { name: "Salvar política" }));

      expect(await screen.findByText(/não pode aumentar quando a antecedência diminui/)).toBeInTheDocument();
      expect(field).toHaveValue(80);
    });
  });
});
