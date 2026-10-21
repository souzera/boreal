import { create } from "zustand";

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

type SessionState = "anonymous" | "active" | "locked";

interface SessionStore {
  token: string | null;
  user: User | null;
  state: SessionState;
  mustChangePassword: boolean;
  setSession: (result: LoginResult) => void;
  setMustChangePassword: (value: boolean) => void;
  lock: () => void;
  unlock: () => void;
  clear: () => void;
}

const anonymous = {
  token: null,
  user: null,
  state: "anonymous" as SessionState,
  mustChangePassword: false,
};

export const useSessionStore = create<SessionStore>((set) => ({
  ...anonymous,
  setSession: ({ token, user, must_change_password }) =>
    set({ token, user, state: "active", mustChangePassword: must_change_password }),
  setMustChangePassword: (value) => set({ mustChangePassword: value }),
  lock: () => set((s) => (s.state === "active" ? { state: "locked" } : s)),
  unlock: () => set((s) => (s.state === "locked" ? { state: "active" } : s)),
  clear: () => set({ ...anonymous }),
}));
