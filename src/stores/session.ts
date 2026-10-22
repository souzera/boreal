import { create } from "zustand";

import { queryClient } from "@/lib/queryClient";

export type Role = "admin" | "recepcao";

export interface User {
  id: number;
  username: string;
  display_name: string;
  role: Role;
  active: boolean;
  must_change_password: boolean;
  failed_attempts: number;
  locked_until: string | null;
  version: number;
}

export interface LoginResult {
  token: string;
  user: User;
  must_change_password: boolean;
}

/**
 * - `locked`: bloqueada por inatividade (desbloqueia com a senha).
 * - `expired`: o backend encerrou a sessão (senha redefinida, permissões alteradas…);
 *   a tela continua montada para a pessoa entrar de novo sem perder o que digitou.
 */
type SessionState = "anonymous" | "active" | "locked" | "expired";

// Valor provisório até o backend informar o configurado (spec §13.3, pendência §25.5).
const DEFAULT_IDLE_MINUTES = 10;

interface SessionStore {
  token: string | null;
  user: User | null;
  state: SessionState;
  mustChangePassword: boolean;
  idleMinutes: number;
  setSession: (result: LoginResult) => void;
  setMustChangePassword: (value: boolean) => void;
  setIdleMinutes: (minutes: number) => void;
  lock: () => void;
  unlock: () => void;
  expire: () => void;
  clear: () => void;
}

const anonymous = {
  token: null,
  user: null,
  state: "anonymous" as SessionState,
  mustChangePassword: false,
};

export const useSessionStore = create<SessionStore>((set, get) => ({
  ...anonymous,
  idleMinutes: DEFAULT_IDLE_MINUTES,
  setSession: ({ token, user, must_change_password }) => {
    const previous = get().user;
    // Outra pessoa entrando não pode ver dados em cache da anterior.
    if (previous && previous.id !== user.id) queryClient.clear();
    set({ token, user, state: "active", mustChangePassword: must_change_password });
  },
  setMustChangePassword: (value) => set({ mustChangePassword: value }),
  setIdleMinutes: (minutes) => set({ idleMinutes: minutes }),
  lock: () => set((s) => (s.state === "active" ? { state: "locked" } : s)),
  unlock: () => set((s) => (s.state === "locked" ? { state: "active" } : s)),
  expire: () => set((s) => (s.state === "anonymous" ? s : { state: "expired" })),
  clear: () => {
    queryClient.clear();
    set({ ...anonymous });
  },
}));
