import { Outlet } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { useActivityHeartbeat } from "@/hooks/useActivityHeartbeat";
import { useIdleLock } from "@/hooks/useIdleLock";
import { logout } from "@/lib/api";
import LockScreen from "@/pages/LockScreen";
import { useSessionStore } from "@/stores/session";
import Sidebar from "./Sidebar";

// Valor padrão provisório; o backend aplica o limite configurado (spec §13.3, pendência §25.5).
const IDLE_MINUTES = 10;

export default function AppShell() {
  const { token, user, state, lock, clear } = useSessionStore();
  const locked = state === "locked";
  useIdleLock(IDLE_MINUTES, lock, !locked);
  useActivityHeartbeat(token, !locked);

  const onLogout = async () => {
    try {
      if (token) await logout(token);
    } finally {
      clear();
    }
  };

  // Bloqueada, a página continua montada (para não perder o que está sendo editado),
  // mas oculta e inerte; a tela de bloqueio cobre tudo.
  return (
    <>
      <div hidden={locked} inert={locked}>
        <div className="flex min-h-screen">
          <Sidebar />
          <div className="flex min-w-0 flex-1 flex-col">
            <header className="flex items-center justify-end gap-3 border-b px-4 py-2">
              <span className="text-sm">{user?.display_name}</span>
              <Button variant="outline" size="sm" onClick={onLogout}>
                Sair
              </Button>
            </header>
            <main className="flex-1 p-6">
              <Outlet />
            </main>
          </div>
        </div>
      </div>
      {locked && <LockScreen />}
    </>
  );
}
