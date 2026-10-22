import { Outlet, useNavigate } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { useSessionStore } from "@/stores/session";
import GlobalSearch from "./GlobalSearch";
import LockGate from "./LockGate";
import Sidebar from "./Sidebar";

export default function AppShell() {
  const user = useSessionStore((s) => s.user);
  const navigate = useNavigate();

  // Sair é uma navegação: se houver alterações não salvas, o aviso aparece antes.
  // O encerramento da sessão acontece ao chegar em /login.
  const onLogout = () => navigate("/login");

  return (
    <LockGate>
      <div className="flex min-h-screen">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <header className="flex items-center justify-between gap-3 border-b px-4 py-2">
            <GlobalSearch />
            <div className="flex items-center gap-3">
              <span className="text-sm">{user?.display_name}</span>
              <Button variant="outline" size="sm" onClick={onLogout}>
                Sair
              </Button>
            </div>
          </header>
          <main className="flex-1 p-6">
            <Outlet />
          </main>
        </div>
      </div>
    </LockGate>
  );
}
