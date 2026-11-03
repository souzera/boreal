import { appRoutes } from "@/routes";
import { NAV, visibleNav, type NavEntry, type NavLeaf } from "./NavItems";

function leaves(entries: NavEntry[]): NavLeaf[] {
  return entries.flatMap((e) => ("items" in e ? e.items : [e]));
}

function routePaths(routes = appRoutes, prefix = ""): string[] {
  return routes.flatMap((r) => {
    const here = r.path ? (r.path.startsWith("/") ? r.path : `${prefix}/${r.path}`) : prefix;
    return [...(r.path && r.path !== "*" ? [here] : []), ...(r.children ? routePaths(r.children, here) : [])];
  });
}

const matches = (to: string, path: string) =>
  new RegExp(`^${path.replace(/:[^/]+/g, "[^/]+")}$`).test(to);

const recepcao = { role: "recepcao" as const, can: () => false };
const admin = { role: "admin" as const, can: () => true };
const labels = (entries: NavEntry[]) => leaves(entries).map((l) => l.label);

describe("menu lateral", () => {
  it("todo link do menu aponta para uma rota que existe", () => {
    const paths = routePaths();
    for (const leaf of leaves(NAV)) {
      if (leaf.to) expect(paths.some((p) => matches(leaf.to as string, p)), leaf.label).toBe(true);
    }
  });

  it("não repete destinos nem rótulos", () => {
    const destinations = leaves(NAV).flatMap((l) => (l.to ? [l.to] : []));
    expect(new Set(destinations).size).toBe(destinations.length);
    expect(new Set(labels(NAV)).size).toBe(labels(NAV).length);
  });

  it("todo item tem ícone", () => {
    for (const leaf of leaves(NAV)) expect(leaf.icon, leaf.label).toMatch(/^[a-z][a-z0-9_]*$/);
  });

  it("Relatórios e Alertas já têm tela e todos veem Relatórios", () => {
    const reports = leaves(NAV).find((l) => l.label === "Relatórios");
    expect(reports?.soon).toBeFalsy();
    expect(reports?.to).toBe("/relatorios");
    expect(labels(visibleNav(NAV, recepcao))).toContain("Relatórios");
    const alerts = leaves(NAV).find((l) => l.label === "Alertas");
    expect(alerts?.soon).toBeFalsy();
    expect(alerts?.to).toBe("/alertas");
  });

  it("Visão gerencial é só do administrador", () => {
    const leaf = leaves(NAV).find((l) => l.label === "Visão gerencial");
    expect(leaf?.to).toBe("/gerencial");
    expect(labels(visibleNav(NAV, recepcao))).not.toContain("Visão gerencial");
    expect(labels(visibleNav(NAV, { role: "admin", can: () => true }))).toContain("Visão gerencial");
  });

  it("Saúde do sistema é só do administrador", () => {
    const leaf = leaves(NAV).find((l) => l.label === "Saúde do sistema");
    expect(leaf?.to).toBe("/saude");
    expect(labels(visibleNav(NAV, recepcao))).not.toContain("Saúde do sistema");
    expect(labels(visibleNav(NAV, admin))).toContain("Saúde do sistema");
  });

  it("a recepção vê o dia a dia e continua vendo Tarifas e Itens e serviços, mas não Configurações nem Usuários", () => {
    const seen = labels(visibleNav(NAV, recepcao));
    for (const label of ["Recepção", "Reservas", "Hóspedes", "Hospedagens", "Mapa de quartos", "Limpeza", "Bloqueios", "Manutenção", "Cadastro de quartos", "Fechamento de caixa", "Tarifas", "Itens e serviços"]) {
      expect(seen, label).toContain(label);
    }
    expect(seen).not.toContain("Configurações");
    expect(seen).not.toContain("Usuários");
    expect(seen).not.toContain("Contas a pagar");
  });

  it("Contas a pagar só aparece com a permissão", () => {
    const withPermission = { role: "recepcao" as const, can: (p: string) => p === "manage_payables" };
    expect(labels(visibleNav(NAV, withPermission))).toContain("Contas a pagar");
  });

  it("o administrador vê tudo", () => {
    expect(labels(visibleNav(NAV, admin))).toEqual(expect.arrayContaining(["Configurações", "Usuários", "Contas a pagar"]));
  });

  it("grupo sem itens visíveis some", () => {
    const onlyAdmin: NavEntry[] = [{ label: "Só admin", items: [{ label: "X", to: "/x", icon: "home", admin: true }] }];
    expect(visibleNav(onlyAdmin, recepcao)).toEqual([]);
  });
});
