import { Navigate, Outlet, type RouteObject } from "react-router-dom";

import AppShell from "@/components/layout/AppShell";
import LockGate from "@/components/layout/LockGate";
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

export const appRoutes: RouteObject[] = [
  { path: "/login", element: <Login /> },
  {
    element: <RequireSession allowPendingPasswordChange />,
    children: [
      {
        path: "/change-password",
        element: (
          <LockGate>
            <ChangePassword />
          </LockGate>
        ),
      },
    ],
  },
  {
    element: <RequireSession />,
    children: [{ element: <AppShell />, children: [{ path: "/", element: <Dashboard /> }] }],
  },
  { path: "*", element: <Navigate to="/" replace /> },
];
