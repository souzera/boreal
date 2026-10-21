import { invoke } from "@tauri-apps/api/core";

import { useSessionStore, type LoginResult, type User } from "@/stores/session";

/** Erro normalizado do backend: `code` estável e `message` em linguagem simples. */
export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public until?: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function toApiError(e: unknown): ApiError {
  if (e instanceof ApiError) return e;
  if (e && typeof e === "object" && "code" in e && "message" in e) {
    const { code, message, until } = e as { code: string; message: string; until?: string };
    return new ApiError(code, message, until);
  }
  return new ApiError("unknown", "Ocorreu um erro inesperado. Tente novamente.");
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const error = toApiError(e);
    // O backend é a autoridade do bloqueio por inatividade.
    if (error.code === "locked") useSessionStore.getState().lock();
    if (error.code === "unauthorized") useSessionStore.getState().clear();
    throw error;
  }
}

export interface SessionStatus {
  state: "active" | "locked";
  user: User | null;
}

export interface AppInfo {
  app_version: string;
  schema_version: number;
}

export const startupStatus = () => call<null>("startup_status");
export const needsBootstrap = () => call<boolean>("needs_bootstrap");
export const bootstrapAdmin = (username: string, displayName: string, password: string) =>
  call<null>("bootstrap_admin", { username, displayName, password });
export const login = (username: string, password: string) =>
  call<LoginResult>("login", { username, password });
export const logout = (token: string) => call<null>("logout", { token });
export const changeOwnPassword = (token: string, current: string, next: string) =>
  call<null>("change_own_password", { token, current, new: next });
export const unlockSession = (token: string, password: string) =>
  call<null>("unlock_session", { token, password });
export const sessionStatus = (token: string) => call<SessionStatus>("session_status", { token });
export const getAppInfo = () => call<AppInfo>("get_app_info");
