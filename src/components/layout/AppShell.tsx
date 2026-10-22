import { Outlet } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { logout } from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import LockGate from "./LockGate";
import Sidebar from "./Sidebar";

export default function AppShell() {
  const { token, user, clear } = useSessionStore();

  const onLogout = async () => {
    try {
      if (token) await logout(token);
    } finally {
      clear();
    }
  };

  return (
    <LockGate>
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
    </LockGate>
  );
}
