import { vi } from "vitest";

import { useSessionStore } from "@/stores/session";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import {
  completeCleaning,
  createBlock,
  listRoomStatus,
  openMaintenance,
  releaseBlock,
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
