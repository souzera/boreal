import { Navigate, Outlet, type RouteObject } from "react-router-dom";

import AppShell from "@/components/layout/AppShell";
import LockGate from "@/components/layout/LockGate";
import RoomMapPage from "@/pages/calendar/RoomMapPage";
import ChangePassword from "@/pages/ChangePassword";
import CategoriesPage from "@/pages/catalog/CategoriesPage";
import RoomsPage from "@/pages/catalog/RoomsPage";
import Dashboard from "@/pages/Dashboard";
import GuestFormPage from "@/pages/guests/GuestFormPage";
import GuestsPage from "@/pages/guests/GuestsPage";
import Login from "@/pages/Login";
import RatesPage from "@/pages/rates/RatesPage";
import Search from "@/pages/Search";
import ReservationFormPage from "@/pages/reservations/ReservationFormPage";
import ReservationPage from "@/pages/reservations/ReservationPage";
import ReservationsPage from "@/pages/reservations/ReservationsPage";
import SettingsPage from "@/pages/settings/SettingsPage";
import UsersPage from "@/pages/users/UsersPage";
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
    children: [
      {
        element: <AppShell />,
        children: [
          { path: "/", element: <Dashboard /> },
          { path: "/configuracoes", element: <SettingsPage /> },
          { path: "/usuarios", element: <UsersPage /> },
          { path: "/busca", element: <Search /> },
          { path: "/hospedes", element: <GuestsPage /> },
          { path: "/hospedes/:id", element: <GuestFormPage /> },
          { path: "/mapa", element: <RoomMapPage /> },
          { path: "/reservas", element: <ReservationsPage /> },
          { path: "/reservas/nova", element: <ReservationFormPage /> },
          { path: "/reservas/:id", element: <ReservationPage /> },
          { path: "/tarifas", element: <RatesPage /> },
          { path: "/quartos", element: <RoomsPage /> },
          { path: "/quartos/categorias", element: <CategoriesPage /> },
        ],
      },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
];
