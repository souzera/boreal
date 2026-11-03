import { Navigate, Outlet, type RouteObject } from "react-router-dom";

import AppShell from "@/components/layout/AppShell";
import LockGate from "@/components/layout/LockGate";
import RoomMapPage from "@/pages/calendar/RoomMapPage";
import ChangePassword from "@/pages/ChangePassword";
import CategoriesPage from "@/pages/catalog/CategoriesPage";
import RoomsPage from "@/pages/catalog/RoomsPage";
import HealthPage from "@/pages/health/HealthPage";
import ManagementPage from "@/pages/management/ManagementPage";
import ReceptionPage from "@/pages/reception/ReceptionPage";
import ReportsPage from "@/pages/reports/ReportsPage";
import GuestFormPage from "@/pages/guests/GuestFormPage";
import GuestsPage from "@/pages/guests/GuestsPage";
import AlertsPage from "@/pages/alerts/AlertsPage";
import ClosingPage from "@/pages/closing/ClosingPage";
import ItemsPage from "@/pages/items/ItemsPage";
import Login from "@/pages/Login";
import PayablesPage from "@/pages/payables/PayablesPage";
import RatesPage from "@/pages/rates/RatesPage";
import Search from "@/pages/Search";
import ReservationFormPage from "@/pages/reservations/ReservationFormPage";
import ReservationPage from "@/pages/reservations/ReservationPage";
import ReservationsPage from "@/pages/reservations/ReservationsPage";
import BlocksPage from "@/pages/blocks/BlocksPage";
import CleaningQueuePage from "@/pages/cleaning/CleaningQueuePage";
import MaintenanceOrderPage from "@/pages/maintenance/MaintenanceOrderPage";
import MaintenancePage from "@/pages/maintenance/MaintenancePage";
import StayPage from "@/pages/stays/StayPage";
import WalkInPage from "@/pages/stays/WalkInPage";
import StaysPage from "@/pages/stays/StaysPage";
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
          { path: "/", element: <ReceptionPage /> },
          { path: "/configuracoes", element: <SettingsPage /> },
          { path: "/usuarios", element: <UsersPage /> },
          { path: "/saude", element: <HealthPage /> },
          { path: "/busca", element: <Search /> },
          { path: "/hospedes", element: <GuestsPage /> },
          { path: "/hospedes/:id", element: <GuestFormPage /> },
          { path: "/mapa", element: <RoomMapPage /> },
          { path: "/gerencial", element: <ManagementPage /> },
          { path: "/relatorios", element: <ReportsPage /> },
          { path: "/reservas", element: <ReservationsPage /> },
          { path: "/reservas/nova", element: <ReservationFormPage /> },
          { path: "/reservas/:id", element: <ReservationPage /> },
          { path: "/hospedagens", element: <StaysPage /> },
          { path: "/limpeza", element: <CleaningQueuePage /> },
          { path: "/bloqueios", element: <BlocksPage /> },
          { path: "/manutencao", element: <MaintenancePage /> },
          { path: "/manutencao/:id", element: <MaintenanceOrderPage /> },
          { path: "/hospedagens/nova", element: <WalkInPage /> },
          { path: "/hospedagens/:id", element: <StayPage /> },
          { path: "/contas-a-pagar", element: <PayablesPage /> },
          { path: "/alertas", element: <AlertsPage /> },
          { path: "/fechamento", element: <ClosingPage /> },
          { path: "/fechamento/:data", element: <ClosingPage /> },
          { path: "/tarifas", element: <RatesPage /> },
          { path: "/quartos", element: <RoomsPage /> },
          { path: "/itens", element: <ItemsPage /> },
          { path: "/quartos/categorias", element: <CategoriesPage /> },
        ],
      },
    ],
  },
  { path: "*", element: <Navigate to="/" replace /> },
];
