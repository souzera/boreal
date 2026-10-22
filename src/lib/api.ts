import { invoke } from "@tauri-apps/api/core";

import { useSessionStore, type LoginResult, type Role, type User } from "@/stores/session";

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
  last_migration_at: string | null;
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

// ---- Configurações e dados da pousada ----

export interface AppSettings {
  max_failed_attempts: number;
  lockout_minutes: number;
  idle_lock_minutes: number;
  default_check_in_time: string;
  default_check_out_time: string;
  late_checkout_tolerance_minutes: number;
  no_show_tolerance_minutes: number;
  cleaning_interval_minutes: number;
}

export interface SettingsView {
  settings: AppSettings;
  version: number;
}

export interface PousadaView {
  name: string;
  address: string;
  phone: string;
  has_logo: boolean;
  version: number;
}

export interface LogoData {
  mime: string;
  base64: string;
}

export type LogoChange =
  | { kind: "keep" }
  | { kind: "remove" }
  | { kind: "set"; mime: string; base64: string };

export interface UpdatePousadaInput {
  name: string;
  address: string;
  phone: string;
  logo: LogoChange;
}

export const getSettings = (token: string) => call<SettingsView>("get_settings", { token });
export const getSessionPolicy = (token: string) =>
  call<{ idle_lock_minutes: number }>("get_session_policy", { token });
export const updateSettings = (token: string, settings: AppSettings, expectedVersion: number) =>
  call<number>("update_settings", { token, input: { settings, expected_version: expectedVersion } });
export const getPousada = (token: string) => call<PousadaView>("get_pousada", { token });
export const getPousadaLogo = (token: string) => call<LogoData | null>("get_pousada_logo", { token });
export const updatePousada = (token: string, input: UpdatePousadaInput, expectedVersion: number) =>
  call<number>("update_pousada", { token, input, expectedVersion });

// ---- Listagens ----

export interface Page<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

// ---- Usuários ----

export interface UserRow {
  id: number;
  username: string;
  display_name: string;
  role: Role;
  active: boolean;
  must_change_password: boolean;
  version: number;
  extra_permissions: string[];
}

export interface UserQuery {
  text?: string;
  active?: boolean;
  page: number;
  page_size: number;
  sort?: string;
  direction?: "asc" | "desc";
}

export interface NewUserInput {
  username: string;
  display_name: string;
  role: Role;
  initial_password: string;
}

export const listUsers = (token: string, query: UserQuery) =>
  call<Page<UserRow>>("list_users", { token, query });
export const createUser = (token: string, input: NewUserInput) =>
  call<number>("create_user", { token, input });
export const resetPassword = (token: string, userId: number, tempPassword: string, expectedVersion: number) =>
  call<null>("reset_password", { token, userId, tempPassword, expectedVersion });
export const setUserRole = (token: string, userId: number, role: Role, expectedVersion: number) =>
  call<null>("set_user_role", { token, userId, role, expectedVersion });
export const setUserActive = (token: string, userId: number, active: boolean) =>
  call<null>("set_user_active", { token, userId, active });
export const grantablePermissions = () => call<string[]>("grantable_permissions");
export const setExtraPermissions = (token: string, userId: number, permissions: string[]) =>
  call<null>("set_extra_permissions", { token, userId, permissions });
