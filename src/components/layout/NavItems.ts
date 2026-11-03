import type { Role } from "@/stores/session";

export interface NavLeaf {
  label: string;
  /** Sem destino = ainda não existe (aparece desabilitado com "Em breve"). */
  to?: string;
  /** Nome do símbolo Material Symbols. */
  icon: string;
  /** Casa só o endereço exato (a Recepção, em `/`). */
  end?: boolean;
  soon?: boolean;
  /** Só administrador. */
  admin?: boolean;
  /** Permissão exigida de quem não é administrador. */
  permission?: string;
}

export interface NavGroup {
  label: string;
  items: NavLeaf[];
}

export type NavEntry = NavLeaf | NavGroup;

/** Menu lateral do brief (§2), agrupado. Itens sem acesso para a pessoa não aparecem. */
export const NAV: NavEntry[] = [
  { label: "Recepção", to: "/", end: true, icon: "home" },
  {
    label: "Reservas",
    items: [
      { label: "Reservas", to: "/reservas", icon: "calendar_month" },
      { label: "Hóspedes", to: "/hospedes", icon: "group" },
    ],
  },
  { label: "Hospedagens", to: "/hospedagens", icon: "hotel" },
  {
    label: "Quartos",
    items: [
      { label: "Mapa de quartos", to: "/mapa", icon: "grid_view" },
      { label: "Limpeza", to: "/limpeza", icon: "cleaning_services" },
      { label: "Bloqueios", to: "/bloqueios", icon: "block" },
      { label: "Manutenção", to: "/manutencao", icon: "build" },
      { label: "Cadastro de quartos", to: "/quartos", icon: "meeting_room" },
    ],
  },
  {
    label: "Financeiro",
    items: [
      { label: "Fechamento de caixa", to: "/fechamento", icon: "point_of_sale" },
      { label: "Contas a pagar", to: "/contas-a-pagar", icon: "receipt_long", permission: "manage_payables" },
    ],
  },
  { label: "Relatórios", to: "/relatorios", icon: "bar_chart" },
  { label: "Visão gerencial", to: "/gerencial", icon: "monitoring", admin: true },
  { label: "Alertas", to: "/alertas", icon: "notifications" },
  {
    label: "Configurações",
    items: [
      { label: "Tarifas", to: "/tarifas", icon: "sell" },
      { label: "Itens e serviços", to: "/itens", icon: "inventory_2" },
      { label: "Configurações", to: "/configuracoes", icon: "settings", admin: true },
      { label: "Usuários", to: "/usuarios", icon: "manage_accounts", admin: true },
    ],
  },
];

interface Viewer {
  role: Role | undefined;
  can: (permission: string) => boolean;
}

const allowed = (leaf: NavLeaf, viewer: Viewer) =>
  viewer.role === "admin" || ((!leaf.admin) && (!leaf.permission || viewer.can(leaf.permission)));

/** Itens do menu que a pessoa pode ver; um grupo sem itens visíveis some. */
export function visibleNav(nav: NavEntry[], viewer: Viewer): NavEntry[] {
  return nav.flatMap((entry): NavEntry[] => {
    if ("items" in entry) {
      const items = entry.items.filter((leaf) => allowed(leaf, viewer));
      return items.length > 0 ? [{ ...entry, items }] : [];
    }
    return allowed(entry, viewer) ? [entry] : [];
  });
}
