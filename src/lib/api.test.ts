import { vi } from "vitest";

import { useSessionStore } from "@/stores/session";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import {
  issueDocument,
  listDocuments,
  saveDocumentPdf,
  type ReportQuery,
  exportReportCsv,
  exportReportPdf,
  openExportedFile,
  runReport,
  managementDashboard,
  receptionOverview,
  alertHistory,
  alertSummary,
  closeCash,
  closingPendingDates,
  closingPreview,
  completeCleaning,
  createBlock,
  dismissAlert,
  getPayable,
  globalSearch,
  listAlerts,
  listExpenseCategories,
  listPayables,
  listRoomStatus,
  openMaintenance,
  payablesSummary,
  payPayable,
  recordRefund,
  recordReservationPayment,
  refreshAlerts,
  recordStayPayment,
  releaseBlock,
  renameExpenseCategory,
  sessionStatus,
  setCleaningPriority,
  setRoomCleaningInterval,
} from "./api";

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

describe("limpeza, bloqueios e manutenção", () => {
  it("cada função chama o comando certo com os argumentos esperados", async () => {
    invoke.mockResolvedValue(undefined);

    await completeCleaning("t", 7, 3);
    expect(invoke).toHaveBeenLastCalledWith("complete_cleaning", { token: "t", taskId: 7, expectedVersion: 3 });

    await setCleaningPriority("t", 7, "high", "Hóspede chega cedo", 2);
    expect(invoke).toHaveBeenLastCalledWith("set_cleaning_priority", {
      token: "t",
      taskId: 7,
      priority: "high",
      justification: "Hóspede chega cedo",
      expectedVersion: 2,
    });

    const block = { room_id: 1, start_date: "2026-11-01", end_date: null, reason: "Pintura" };
    await createBlock("t", block);
    expect(invoke).toHaveBeenLastCalledWith("create_block", { token: "t", input: block });

    await releaseBlock("t", 4, "Obra terminou", 1);
    expect(invoke).toHaveBeenLastCalledWith("release_block", {
      token: "t",
      blockId: 4,
      justification: "Obra terminou",
      expectedVersion: 1,
    });

    const order = {
      room_id: 1,
      description: "Trocar o chuveiro",
      priority: "high" as const,
      planned_start: "2026-11-10",
      planned_end: null,
      responsible: "",
      notes: "",
    };
    await openMaintenance("t", order);
    expect(invoke).toHaveBeenLastCalledWith("open_maintenance", { token: "t", input: order });

    await setRoomCleaningInterval("t", 1, null, 5);
    expect(invoke).toHaveBeenLastCalledWith("set_room_cleaning_interval", {
      token: "t",
      roomId: 1,
      minutes: null,
      expectedVersion: 5,
    });

    await listRoomStatus("t");
    expect(invoke).toHaveBeenLastCalledWith("list_room_status", { token: "t" });
  });
});

describe("financeiro", () => {
  const split = {
    components: [
      { method: "pix" as const, amount_cents: 6000 },
      { method: "cash" as const, amount_cents: 4000 },
    ],
    note: "",
  };

  it("registra recebimentos com componentes na hospedagem e na reserva", async () => {
    invoke.mockResolvedValue({ id: 1, code: "PAG-000001", total_cents: 10000 });

    await recordStayPayment("t", 7, split);
    expect(invoke).toHaveBeenLastCalledWith("record_stay_payment", { token: "t", stayId: 7, input: split });

    await recordReservationPayment("t", 3, 2, split);
    expect(invoke).toHaveBeenLastCalledWith("record_reservation_payment", {
      token: "t",
      reservationId: 3,
      expectedVersion: 2,
      input: split,
    });
  });

  it("preserva os dados do recebimento parecido no erro de duplicidade", async () => {
    const details = { code: "PAG-000001", received_at: "2026-10-21T12:00:00Z", total_cents: 10000, methods: ["pix"] };
    invoke.mockRejectedValue({ code: "duplicate_payment", message: "Possível recebimento duplicado", details });

    await expect(recordStayPayment("t", 7, split)).rejects.toMatchObject({ code: "duplicate_payment", details });
  });

  it("chama devoluções, contas a pagar e fechamento com os nomes certos", async () => {
    invoke.mockResolvedValue(null);
    const refund = { payment_id: 4, components: [{ method: "pix" as const, amount_cents: 500 }], reason: "Engano" };
    await recordRefund("t", refund);
    expect(invoke).toHaveBeenLastCalledWith("record_refund", { token: "t", input: refund });

    const query = { status: "overdue" as const, page: 1, page_size: 20 };
    await listPayables("t", query);
    expect(invoke).toHaveBeenLastCalledWith("list_payables", { token: "t", query });
    await getPayable("t", 2);
    expect(invoke).toHaveBeenLastCalledWith("get_payable", { token: "t", id: 2 });
    await payPayable("t", 2, { amount_cents: 100, method: "cash", note: "" }, 1);
    expect(invoke).toHaveBeenLastCalledWith("pay_payable", {
      token: "t",
      id: 2,
      input: { amount_cents: 100, method: "cash", note: "" },
      expectedVersion: 1,
    });
    const summary = { from: "2026-10-01", to: "2026-10-31", basis: "payment" as const };
    await payablesSummary("t", summary);
    expect(invoke).toHaveBeenLastCalledWith("payables_summary", { token: "t", query: summary });
    await listExpenseCategories("t", true);
    expect(invoke).toHaveBeenLastCalledWith("list_expense_categories", { token: "t", onlyActive: true });
    await renameExpenseCategory("t", 1, "Luz", 3);
    expect(invoke).toHaveBeenLastCalledWith("rename_expense_category", { token: "t", id: 1, name: "Luz", expectedVersion: 3 });

    await closingPreview("t", "2026-10-21");
    expect(invoke).toHaveBeenLastCalledWith("closing_preview", { token: "t", date: "2026-10-21" });
    const close = { date: "2026-10-21", counted: [{ method: "cash" as const, counted_cents: 100, justification: null }] };
    await closeCash("t", close);
    expect(invoke).toHaveBeenLastCalledWith("close_cash", { token: "t", input: close });
    await closingPendingDates("t");
    expect(invoke).toHaveBeenLastCalledWith("closing_pending_dates", { token: "t" });
  });
});

describe("alertas", () => {
  it("chama os comandos de alerta com os nomes e argumentos certos", async () => {
    invoke.mockResolvedValue(null);

    await refreshAlerts("t");
    expect(invoke).toHaveBeenLastCalledWith("refresh_alerts", { token: "t" });

    const query = { status: "history" as const, kind: "deposit_overdue" as const, severity: "high" as const, page: 2, page_size: 10 };
    await listAlerts("t", query);
    expect(invoke).toHaveBeenLastCalledWith("list_alerts", { token: "t", query });

    await alertSummary("t");
    expect(invoke).toHaveBeenLastCalledWith("alert_summary", { token: "t" });

    await alertHistory("t", 4);
    expect(invoke).toHaveBeenLastCalledWith("alert_history", { token: "t", alertId: 4 });

    await dismissAlert("t", 4, "Hóspede avisou que chega tarde", 3);
    expect(invoke).toHaveBeenLastCalledWith("dismiss_alert", { token: "t", alertId: 4, note: "Hóspede avisou que chega tarde", expectedVersion: 3 });
  });
});

describe("busca global", () => {
  it("chama global_search com o texto digitado", async () => {
    invoke.mockResolvedValue(null);

    await globalSearch("t", "ana souza");

    expect(invoke).toHaveBeenLastCalledWith("global_search", { token: "t", text: "ana souza" });
  });
});

describe("recepção e visão gerencial", () => {
  it("chama reception_overview e management_dashboard", async () => {
    invoke.mockResolvedValue(null);

    await receptionOverview("t");
    expect(invoke).toHaveBeenLastCalledWith("reception_overview", { token: "t" });

    await managementDashboard("t", "2026-10-21", "2026-10-28");
    expect(invoke).toHaveBeenLastCalledWith("management_dashboard", { token: "t", from: "2026-10-21", to: "2026-10-28" });
  });
});

describe("relatórios", () => {
  const query: ReportQuery = { kind: "receipts", from: "2026-10-01", to: "2026-10-31", method: "pix" };

  it("run_report, exportações e abrir arquivo chamam os comandos certos", async () => {
    invoke.mockResolvedValue(null);

    await runReport("t", query);
    expect(invoke).toHaveBeenLastCalledWith("run_report", { token: "t", query });

    await exportReportCsv("t", query, ";", "C:/x/r.csv");
    expect(invoke).toHaveBeenLastCalledWith("export_report_csv", { token: "t", query, delimiter: ";", path: "C:/x/r.csv" });

    await exportReportPdf("t", query, "C:/x/r.pdf");
    expect(invoke).toHaveBeenLastCalledWith("export_report_pdf", { token: "t", query, path: "C:/x/r.pdf" });

    await openExportedFile("t", "C:/x/r.pdf");
    expect(invoke).toHaveBeenLastCalledWith("open_exported_file", { token: "t", path: "C:/x/r.pdf" });
  });
});

describe("documentos internos", () => {
  it("issue_document, list_documents e save_document_pdf chamam os comandos certos", async () => {
    invoke.mockResolvedValue(null);

    await issueDocument("t", "payment_receipt", 9);
    expect(invoke).toHaveBeenLastCalledWith("issue_document", { token: "t", kind: "payment_receipt", sourceId: 9 });

    await listDocuments("t", "stay_record", 3);
    expect(invoke).toHaveBeenLastCalledWith("list_documents", { token: "t", kind: "stay_record", sourceId: 3 });

    await saveDocumentPdf("t", 12, "C:/x/d.pdf");
    expect(invoke).toHaveBeenLastCalledWith("save_document_pdf", { token: "t", documentId: 12, path: "C:/x/d.pdf" });
  });
});
