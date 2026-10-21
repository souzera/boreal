import { Navigate, Outlet, Route, Routes } from "react-router-dom";

import AppShell from "@/components/layout/AppShell";
import ChangePassword from "@/pages/ChangePassword";
import Dashboard from "@/pages/Dashboard";
import Login from "@/pages/Login";
import { useSessionStore } from "@/stores/session";

/** Exige sessão (ativa ou bloqueada) e, se pendente, a troca de senha obrigatória. */
function RequireSession({ allowPendingPasswordChange = false }: { allowPendingPasswordChange?: boolean }) {
  const { state, mustChangePassword } = useSessionStore();
  if (state === "anonymous") return <Navigate to="/login" replace />;
  if (mustChangePassword && !allowPendingPasswordChange) {
    return <Navigate to="/change-password" replace />;
  }
  return <Outlet />;
}

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route element={<RequireSession allowPendingPasswordChange />}>
        <Route path="/change-password" element={<ChangePassword />} />
      </Route>
      <Route element={<RequireSession />}>
        <Route element={<AppShell />}>
          <Route path="/" element={<Dashboard />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
