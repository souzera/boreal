import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactElement } from "react";
import { createMemoryRouter, RouterProvider, type RouteObject } from "react-router-dom";

import { useSessionStore, type User } from "@/stores/session";

export const adminUser: User = {
  id: 1,
  username: "admin",
  display_name: "Administrador",
  role: "admin",
  active: true,
  must_change_password: false,
  failed_attempts: 0,
  locked_until: null,
  version: 1,
};

export const recepcaoUser: User = {
  ...adminUser,
  id: 2,
  username: "rec",
  display_name: "Recepção",
  role: "recepcao",
};

/** Abre uma sessão ativa no store. */
export function signIn(user: User = adminUser, token = "t") {
  useSessionStore.getState().clear();
  useSessionStore.getState().setSession({ token, user, must_change_password: false });
}

interface PageOptions {
  /** Padrão da rota da página (pode ter parâmetros, como `/hospedes/:id`). */
  path?: string;
  /** Endereço inicial; por padrão, o próprio `path`. */
  entry?: string;
  /** Rotas de destino de navegações, com conteúdo simples para o teste enxergar. */
  extraRoutes?: RouteObject[];
}

/**
 * Renderiza uma página dentro de um roteador de dados (necessário ao aviso de alterações
 * não salvas) e de um `QueryClientProvider`. A rota `/outra` serve para testar navegação.
 */
export function renderPage(ui: ReactElement, options: PageOptions = {}) {
  const { path = "/", entry = path, extraRoutes = [] } = options;
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter(
    [{ path, element: ui }, { path: "/outra", element: <p>Outra página</p> }, ...extraRoutes],
    { initialEntries: [entry] },
  );
  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return { router, queryClient };
}
