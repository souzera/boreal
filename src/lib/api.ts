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
    // Sessão encerrada pelo backend: mantém a tela montada para entrar de novo sem perder edições.
    if (error.code === "unauthorized") useSessionStore.getState().expire();
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
  deposit_percent_bp: number;
  deposit_due_days_after_booking: number;
  deposit_due_days_before_arrival: number;
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

// ---- Categorias e quartos ----

export interface Category {
  id: number;
  name: string;
  description: string;
  active: boolean;
  version: number;
}

export interface CategoryInput {
  name: string;
  description: string;
}

export interface Room {
  id: number;
  code: string;
  category_id: number;
  category_name: string;
  max_capacity: number;
  description: string;
  notes: string;
  active: boolean;
  version: number;
}

export interface RoomInput {
  code: string;
  category_id: number;
  max_capacity: number;
  description: string;
  notes: string;
}

export interface RoomQuery {
  text?: string;
  category_id?: number;
  active?: boolean;
  page: number;
  page_size: number;
  sort?: string;
  direction?: "asc" | "desc";
}

export const listCategories = (token: string, activeOnly: boolean) =>
  call<Category[]>("list_categories", { token, activeOnly });
export const createCategory = (token: string, input: CategoryInput) =>
  call<number>("create_category", { token, input });
export const updateCategory = (token: string, id: number, input: CategoryInput, expectedVersion: number) =>
  call<null>("update_category", { token, id, input, expectedVersion });
export const setCategoryActive = (token: string, id: number, active: boolean) =>
  call<null>("set_category_active", { token, id, active });
export const listRooms = (token: string, query: RoomQuery) => call<Page<Room>>("list_rooms", { token, query });
export const createRoom = (token: string, input: RoomInput) => call<number>("create_room", { token, input });
export const updateRoom = (token: string, id: number, input: RoomInput, expectedVersion: number) =>
  call<null>("update_room", { token, id, input, expectedVersion });
export const setRoomActive = (token: string, id: number, active: boolean) =>
  call<null>("set_room_active", { token, id, active });

// ---- Hóspedes ----

export interface Guest {
  id: number;
  code: string;
  full_name: string;
  phone: string;
  cpf: string | null;
  email: string;
  address: string;
  active: boolean;
  version: number;
}

export interface GuestInput {
  full_name: string;
  phone: string;
  cpf: string | null;
  email: string;
  address: string;
}

export type DuplicateReason = "name" | "phone" | "cpf";

export interface DuplicateMatch {
  guest: Guest;
  reasons: DuplicateReason[];
}

export type CreateGuestOutcome =
  | { status: "created"; id: number; code: string }
  | { status: "duplicates"; matches: DuplicateMatch[] };

export interface GuestQuery {
  text?: string;
  active?: boolean;
  page: number;
  page_size: number;
  sort?: string;
  direction?: "asc" | "desc";
}

export const listGuests = (token: string, query: GuestQuery) => call<Page<Guest>>("list_guests", { token, query });
export const getGuest = (token: string, id: number) => call<Guest>("get_guest", { token, id });
export const createGuest = (token: string, input: GuestInput, confirmDuplicates: boolean) =>
  call<CreateGuestOutcome>("create_guest", { token, input, confirmDuplicates });
export const updateGuest = (token: string, id: number, input: GuestInput, expectedVersion: number) =>
  call<null>("update_guest", { token, id, input, expectedVersion });
export const setGuestActive = (token: string, id: number, active: boolean) =>
  call<null>("set_guest_active", { token, id, active });

// ---- Tarifas ----

export type RateRuleType = "weekday" | "season" | "holiday" | "date_override";

export interface RateRule {
  id: number;
  category_id: number;
  rule_type: RateRuleType;
  weekday: number | null;
  start_date: string | null;
  end_date: string | null;
  date: string | null;
  amount_cents: number;
  label: string;
  active: boolean;
  version: number;
}

export interface RateRuleInput {
  category_id: number;
  rule_type: RateRuleType;
  weekday: number | null;
  start_date: string | null;
  end_date: string | null;
  date: string | null;
  amount_cents: number;
  label: string;
}

export interface Holiday {
  id: number;
  date: string;
  name: string;
}

export type AppliedRule = "date_override" | "holiday" | "season" | "weekday";

export interface NightPrice {
  date: string;
  amount_cents: number;
  rule: AppliedRule;
}

export interface Quote {
  nights: NightPrice[];
  total_cents: number;
  missing_dates: string[];
}

export const listRateRules = (token: string, categoryId?: number) =>
  call<RateRule[]>("list_rate_rules", { token, categoryId: categoryId ?? null });
export const createRateRule = (token: string, input: RateRuleInput) =>
  call<number>("create_rate_rule", { token, input });
export const updateRateRule = (token: string, id: number, input: RateRuleInput, expectedVersion: number) =>
  call<null>("update_rate_rule", { token, id, input, expectedVersion });
export const setRateRuleActive = (token: string, id: number, active: boolean) =>
  call<null>("set_rate_rule_active", { token, id, active });
export const deleteRateRule = (token: string, id: number) => call<null>("delete_rate_rule", { token, id });
export const listHolidays = (token: string, year?: number) =>
  call<Holiday[]>("list_holidays", { token, year: year ?? null });
export const createHoliday = (token: string, date: string, name: string) =>
  call<number>("create_holiday", { token, date, name });
export const deleteHoliday = (token: string, id: number) => call<null>("delete_holiday", { token, id });
export const quoteStay = (token: string, categoryId: number, checkIn: string, checkOut: string) =>
  call<Quote>("quote_stay", { token, categoryId, checkIn, checkOut });

// ---- Política de cancelamento ----

export interface CancellationTier {
  min_days_before: number;
  refund_bp: number;
}

export interface CancellationPolicy {
  tiers: CancellationTier[];
  version: number;
}

export const getCancellationPolicy = (token: string) =>
  call<CancellationPolicy>("get_cancellation_policy", { token });
export const updateCancellationPolicy = (token: string, tiers: CancellationTier[], expectedVersion: number) =>
  call<number>("update_cancellation_policy", { token, tiers, expectedVersion });
