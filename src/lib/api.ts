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
  late_checkout_fee_bp: number;
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
  cleaning_interval_minutes: number | null;
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
// ---- Itens e serviços (catálogo de lançamentos) ----

export interface Item {
  id: number;
  name: string;
  default_price_cents: number;
  active: boolean;
  version: number;
}

export interface ItemInput {
  name: string;
  default_price_cents: number;
}

export const listItems = (token: string, onlyActive: boolean) => call<Item[]>("list_items", { token, onlyActive });
export const createItem = (token: string, input: ItemInput) => call<number>("create_item", { token, input });
export const updateItem = (token: string, id: number, input: ItemInput, expectedVersion: number) =>
  call<null>("update_item", { token, id, input, expectedVersion });
export const setItemActive = (token: string, id: number, active: boolean, expectedVersion: number) =>
  call<null>("set_item_active", { token, id, active, expectedVersion });

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

// ---- Reservas ----

export type ReservationStatus = "pending" | "confirmed" | "checked_in" | "completed" | "cancelled" | "no_show";

export interface Reservation {
  id: number;
  code: string;
  room_id: number;
  room_code: string;
  guest_id: number;
  guest_name: string;
  check_in_date: string;
  check_out_date: string;
  expected_check_in_time: string;
  expected_check_out_time: string;
  nights: number;
  occupants: number;
  status: ReservationStatus;
  list_total_cents: number;
  discount_cents: number;
  discount_reason: string | null;
  agreed_total_cents: number;
  deposit_expected_cents: number;
  deposit_paid_cents: number;
  deposit_due_date: string;
  deposit_overdue: boolean;
  notes: string;
  cancelled_at: string | null;
  cancellation_fee_cents: number | null;
  version: number;
  created_at: string;
  /** `booking` (reserva) ou `walk_in` (hospedagem sem reserva prévia). */
  origin: "booking" | "walk_in";
}

export interface ReservationNight {
  date: string;
  amount_cents: number;
  rule: AppliedRule;
}

export interface ReservationChange {
  id: number;
  changed_at: string;
  changed_by: number;
  changed_by_name: string;
  kind: string;
  before_json: string;
  after_json: string;
  price_before_cents: number | null;
  price_after_cents: number | null;
  justification: string | null;
}

export interface ReservationDetail {
  reservation: Reservation;
  nights: ReservationNight[];
  history: ReservationChange[];
  /** Hospedagem aberta a partir desta reserva, se houver. */
  stay_id: number | null;
}

export interface ReservationQuery {
  text?: string;
  status?: ReservationStatus[];
  room_id?: number;
  from?: string;
  to?: string;
  page: number;
  page_size: number;
  sort?: string;
  direction?: "asc" | "desc";
}

export interface NewReservationInput {
  room_id: number;
  guest_id: number;
  check_in_date: string;
  check_out_date: string;
  expected_check_in_time: string | null;
  expected_check_out_time: string | null;
  occupants: number;
  notes: string;
  exceed_capacity: boolean;
}

export interface CreatedReservation {
  id: number;
  code: string;
  agreed_total_cents: number;
  deposit_expected_cents: number;
  deposit_due_date: string;
}

export interface ConflictInfo {
  reservation_id: number;
  code: string;
  guest_name: string;
  check_in_date: string;
  check_out_date: string;
  status: ReservationStatus;
}

export interface AvailabilityResult {
  available: boolean;
  conflicts: ConflictInfo[];
  blocks: RoomBlock[];
  /** Motivo de o quarto não estar pronto para hospedar hoje (limpeza pendente ou intervalo). */
  not_ready: string | null;
}

export interface ChangeRequest {
  room_id: number | null;
  check_in_date: string | null;
  check_out_date: string | null;
}

export interface NightChange {
  date: string;
  amount_cents: number;
  kept: boolean;
  rule: AppliedRule;
}

export interface ChangePreview {
  feasible: boolean;
  fingerprint: string;
  price_before_cents: number;
  price_after_cents: number;
  difference_cents: number;
  new_deposit_expected_cents: number;
  nights: NightChange[];
  missing_dates: string[];
  conflicts: ConflictInfo[];
  problems: string[];
}

export interface CancellationPreview {
  days_before: number;
  deposit_paid_cents: number;
  refund_cents: number;
  retained_cents: number;
  tier_min_days: number | null;
  can_cancel: boolean;
}

export interface CancellationOverride {
  retained_cents: number;
  justification: string;
}

export interface CancellationResult {
  refund_cents: number;
  retained_cents: number;
}

export interface DiscountInput {
  kind: "amount" | "percent";
  amount_cents: number | null;
  percent_bp: number | null;
  justification: string;
}

export interface ReviewItem {
  reservation: Reservation;
  reasons: ("no_show_candidate" | "deposit_overdue")[];
}

export const createReservation = (token: string, input: NewReservationInput) =>
  call<CreatedReservation>("create_reservation", { token, input });
export const getReservation = (token: string, id: number) => call<ReservationDetail>("get_reservation", { token, id });
export const listReservations = (token: string, query: ReservationQuery) =>
  call<Page<Reservation>>("list_reservations", { token, query });
export const checkRoomAvailability = (
  token: string,
  roomId: number,
  checkIn: string,
  checkOut: string,
  ignoreReservationId?: number,
) =>
  call<AvailabilityResult>("check_room_availability", {
    token,
    roomId,
    checkIn,
    checkOut,
    ignoreReservationId: ignoreReservationId ?? null,
  });
export const previewReservationChange = (token: string, id: number, request: ChangeRequest) =>
  call<ChangePreview>("preview_reservation_change", { token, id, request });
export const applyReservationChange = (
  token: string,
  id: number,
  request: ChangeRequest,
  expectedVersion: number,
  justification: string,
  fingerprint: string,
) => call<null>("apply_reservation_change", { token, id, request, expectedVersion, justification, fingerprint });
export const previewReservationCancellation = (token: string, id: number) =>
  call<CancellationPreview>("preview_reservation_cancellation", { token, id });
export const cancelReservation = (
  token: string,
  id: number,
  expectedVersion: number,
  exception: CancellationOverride | null,
) => call<CancellationResult>("cancel_reservation", { token, id, expectedVersion, exception });
export const markReservationNoShow = (token: string, id: number, expectedVersion: number) =>
  call<null>("mark_reservation_no_show", { token, id, expectedVersion });
export const listReservationsNeedingReview = (token: string) =>
  call<ReviewItem[]>("list_reservations_needing_review", { token });
export const applyReservationDiscount = (token: string, id: number, input: DiscountInput, expectedVersion: number) =>
  call<null>("apply_reservation_discount", { token, id, input, expectedVersion });
export const confirmReservation = (token: string, id: number, expectedVersion: number) =>
  call<null>("confirm_reservation", { token, id, expectedVersion });
export const recordReservationDeposit = (token: string, id: number, amountCents: number, expectedVersion: number) =>
  call<null>("record_reservation_deposit", { token, id, amountCents, expectedVersion });
export const setReservationNotes = (token: string, id: number, notes: string, expectedVersion: number) =>
  call<null>("set_reservation_notes", { token, id, notes, expectedVersion });

// ---- Permissões da sessão ----

export const myPermissions = (token: string) => call<string[]>("my_permissions", { token });

// ---- Mapa de quartos ----

export interface RoomMapRoom {
  id: number;
  code: string;
  category_name: string;
  active: boolean;
  state: RoomStateKey | null;
  state_label: string | null;
  free_at: string | null;
}

export interface RoomMapCell {
  room_id: number;
  date: string;
  reservation_id: number;
  code: string;
  guest_name: string;
  status: ReservationStatus;
  is_check_in: boolean;
  is_check_out: boolean;
}

export interface RoomMap {
  from: string;
  to: string;
  rooms: RoomMapRoom[];
  cells: RoomMapCell[];
}

export const roomMap = (token: string, from: string, days: number) => call<RoomMap>("room_map", { token, from, days });

// ---- Hospedagens e conta do hóspede ----

export type StayStatus = "in_house" | "checked_out";
export type ChargeKind = "night" | "consumption" | "extra" | "discount" | "adjustment" | "late_fee" | "reversal";
export type PaymentMethod = "cash" | "pix" | "debit" | "credit" | "transfer" | "prior_deposit";

export interface Stay {
  id: number;
  code: string;
  reservation_id: number;
  status: StayStatus;
  room_id: number;
  room_code: string;
  guest_id: number;
  guest_name: string;
  check_in_date: string;
  check_out_date: string;
  expected_check_in_time: string;
  expected_check_out_time: string;
  checked_in_at: string;
  checked_out_at: string | null;
  early_check_in: boolean;
  late_tolerance_minutes: number | null;
  version: number;
}

export interface Occupant {
  id: number;
  full_name: string;
  guest_id: number | null;
  document: string;
  is_holder: boolean;
}

export interface ChargeLine {
  id: number;
  kind: ChargeKind;
  description: string;
  amount_cents: number;
  night_date: string | null;
  justification: string | null;
  reverses_charge_id: number | null;
  reversed: boolean;
  created_by_name: string;
  created_at: string;
}

export interface PaymentLine {
  id: number;
  code: string;
  amount_cents: number;
  method: PaymentMethod;
  received_at: string;
  received_by_name: string;
}

export interface RefundLine {
  id: number;
  code: string;
  payment_code: string;
  amount_cents: number;
  reason: string;
  refunded_at: string;
}

export interface StayAccount {
  charges: ChargeLine[];
  payments: PaymentLine[];
  refunds: RefundLine[];
  charges_total_cents: number;
  received_cents: number;
  refunded_cents: number;
  balance_cents: number;
}

export interface StayDetail {
  stay: Stay;
  occupants: Occupant[];
  account: StayAccount;
}

export interface DepartureRisk {
  next_reservation_id: number;
  next_code: string;
  next_start: string;
  ready_at: string;
  overrun_minutes: number;
}

export interface StaySummary {
  stay: Stay;
  balance_cents: number;
  minutes_late: number;
  risk: DepartureRisk | null;
}

export interface StayQuery {
  text?: string;
  status?: StayStatus;
  page: number;
  page_size: number;
}

export interface OccupantInput {
  full_name: string;
  guest_id: number | null;
  document: string;
}

export interface CheckInInput {
  reservation_id: number;
  expected_version: number;
  occupants: OccupantInput[];
  exceed_capacity: boolean;
}

export interface WalkInInput {
  room_id: number;
  guest_id: number;
  check_out_date: string;
  expected_check_out_time: string | null;
  occupants: OccupantInput[];
  exceed_capacity: boolean;
}

export interface ChargeInput {
  kind: "consumption" | "extra";
  description: string;
  amount_cents: number;
  item_id: number | null;
}

export interface AdjustmentInput {
  description: string;
  amount_cents: number;
  justification: string;
}

export interface PaymentInput {
  amount_cents: number;
  method: Exclude<PaymentMethod, "prior_deposit">;
  note: string;
}

export interface LateInfo {
  minutes_late: number;
  tolerance_minutes: number;
  applies: boolean;
  fee_cents: number;
}

export interface CheckoutPreview {
  stay: Stay;
  account: StayAccount;
  late: LateInfo;
  balance_cents: number;
  balance_with_late_fee_cents: number;
  risk: DepartureRisk | null;
}

export type LateFeeChoice = "not_applicable" | "charge" | { waive: { justification: string } };

export interface CheckoutInput {
  stay_id: number;
  expected_version: number;
  late_fee: LateFeeChoice;
  balance_exception: { justification: string } | null;
}

export const checkInReservation = (token: string, input: CheckInInput) =>
  call<number>("check_in_reservation", { token, input });
export const walkIn = (token: string, input: WalkInInput) => call<number>("walk_in", { token, input });
export const getStay = (token: string, id: number) => call<StayDetail>("get_stay", { token, id });
export const listStays = (token: string, query: StayQuery) => call<Page<StaySummary>>("list_stays", { token, query });
export const addStayCharge = (token: string, stayId: number, input: ChargeInput) =>
  call<number>("add_stay_charge", { token, stayId, input });
export const addStayAdjustment = (token: string, stayId: number, input: AdjustmentInput) =>
  call<number>("add_stay_adjustment", { token, stayId, input });
export const reverseStayCharge = (
  token: string,
  stayId: number,
  chargeId: number,
  justification: string,
  replacement: ChargeInput | null,
) => call<null>("reverse_stay_charge", { token, stayId, chargeId, justification, replacement });
export const recordStayPayment = (token: string, stayId: number, input: PaymentInput) =>
  call<number>("record_stay_payment", { token, stayId, input });
export const previewStayCheckout = (token: string, stayId: number) =>
  call<CheckoutPreview>("preview_stay_checkout", { token, stayId });
export const checkoutStay = (token: string, input: CheckoutInput) => call<null>("checkout_stay", { token, input });
export const setStayLateTolerance = (token: string, stayId: number, minutes: number | null, expectedVersion: number) =>
  call<null>("set_stay_late_tolerance", { token, stayId, minutes, expectedVersion });

// ---- Limpeza, bloqueios e manutenção ----

export type RoomStateKey =
  | "occupied"
  | "awaiting_cleaning"
  | "cleaning"
  | "clean_waiting_interval"
  | "available"
  | "blocked";
export type CleaningPriority = "urgent" | "high" | "normal" | "low";
export type CleaningStatus = "awaiting" | "in_progress" | "done";

export interface RoomStatus {
  room_id: number;
  code: string;
  category_name: string;
  state: RoomStateKey;
  label_pt: string;
  free_at: string | null;
  cleaning_task_id: number | null;
}

export interface CleaningTask {
  id: number;
  room_id: number;
  room_code: string;
  stay_id: number | null;
  status: CleaningStatus;
  ready_since: string;
  priority_override: CleaningPriority | null;
  priority_justification: string | null;
  started_at: string | null;
  started_by: number | null;
  completed_at: string | null;
  completed_by: number | null;
  version: number;
}

export interface CleaningItem {
  task: CleaningTask;
  priority: CleaningPriority;
  suggested_priority: CleaningPriority;
  next_arrival: string | null;
}

export const cleaningQueue = (token: string) => call<CleaningItem[]>("cleaning_queue", { token });
export const startCleaning = (token: string, taskId: number, expectedVersion: number) =>
  call<null>("start_cleaning", { token, taskId, expectedVersion });
export const completeCleaning = (token: string, taskId: number, expectedVersion: number) =>
  call<null>("complete_cleaning", { token, taskId, expectedVersion });
export const setCleaningPriority = (
  token: string,
  taskId: number,
  priority: CleaningPriority | null,
  justification: string,
  expectedVersion: number,
) => call<null>("set_cleaning_priority", { token, taskId, priority, justification, expectedVersion });
export const listRoomStatus = (token: string) => call<RoomStatus[]>("list_room_status", { token });
export const setRoomCleaningInterval = (
  token: string,
  roomId: number,
  minutes: number | null,
  expectedVersion: number,
) => call<null>("set_room_cleaning_interval", { token, roomId, minutes, expectedVersion });

export type BlockKind = "manual" | "preventive" | "maintenance";
export type BlockStatus = "preventive" | "active" | "awaiting_release" | "released";

export interface RoomBlock {
  id: number;
  room_id: number;
  room_code: string;
  kind: BlockKind;
  status: BlockStatus;
  start_date: string;
  end_date: string | null;
  reason: string;
  maintenance_order_id: number | null;
  released_at: string | null;
  release_justification: string | null;
  version: number;
}

export interface NewBlockInput {
  room_id: number;
  start_date: string;
  end_date: string | null;
  reason: string;
}

export const listBlocks = (token: string, includeReleased: boolean) =>
  call<RoomBlock[]>("list_blocks", { token, includeReleased });
export const createBlock = (token: string, input: NewBlockInput) => call<number>("create_block", { token, input });
export const releaseBlock = (token: string, blockId: number, justification: string, expectedVersion: number) =>
  call<null>("release_block", { token, blockId, justification, expectedVersion });

export type MaintenanceStatus = "open" | "in_progress" | "awaiting_resolution" | "completed";

export interface MaintenanceOrder {
  id: number;
  code: string;
  room_id: number;
  room_code: string;
  description: string;
  priority: CleaningPriority;
  status: MaintenanceStatus;
  planned_start: string;
  planned_end: string | null;
  actual_start: string | null;
  actual_end: string | null;
  responsible: string;
  notes: string;
  version: number;
}

export interface MaintenanceConflict {
  id: number;
  order_id: number;
  reservation_id: number;
  reservation_code: string;
  guest_name: string;
  check_in_date: string;
  check_out_date: string;
  reservation_status: ReservationStatus;
  status: "open" | "resolved";
  resolution_note: string | null;
}

export interface NewOrderInput {
  room_id: number;
  description: string;
  priority: CleaningPriority;
  planned_start: string;
  planned_end: string | null;
  responsible: string;
  notes: string;
}

export interface OpenedOrder {
  id: number;
  code: string;
  conflicts: ConflictInfo[];
}

export interface ApplyResult {
  still_open: MaintenanceConflict[];
  effective: boolean;
}

export interface OrderDetail {
  order: MaintenanceOrder;
  conflicts: MaintenanceConflict[];
  block: RoomBlock | null;
}

export const listMaintenance = (token: string, onlyOpen: boolean) =>
  call<MaintenanceOrder[]>("list_maintenance", { token, onlyOpen });
export const getMaintenance = (token: string, id: number) => call<OrderDetail>("get_maintenance", { token, id });
export const openMaintenance = (token: string, input: NewOrderInput) =>
  call<OpenedOrder>("open_maintenance", { token, input });
export const applyMaintenanceUnavailability = (token: string, orderId: number, expectedVersion: number) =>
  call<ApplyResult>("apply_maintenance_unavailability", { token, orderId, expectedVersion });
export const resolveMaintenanceConflict = (token: string, conflictId: number, note: string) =>
  call<null>("resolve_maintenance_conflict", { token, conflictId, note });
export const startMaintenance = (token: string, orderId: number, expectedVersion: number) =>
  call<null>("start_maintenance", { token, orderId, expectedVersion });
export const requestMaintenanceResolution = (token: string, orderId: number, expectedVersion: number) =>
  call<null>("request_maintenance_resolution", { token, orderId, expectedVersion });
export const completeMaintenance = (token: string, orderId: number, expectedVersion: number) =>
  call<null>("complete_maintenance", { token, orderId, expectedVersion });
