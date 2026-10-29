import { NavLink } from "react-router-dom";

import { cn } from "@/lib/utils";
import { useSessionStore } from "@/stores/session";

const upcoming = ["Financeiro", "Relatórios"];

function linkClass({ isActive }: { isActive: boolean }) {
  return cn(
    "rounded-md px-2 py-1.5 text-sm",
    isActive ? "bg-primary text-primary-foreground" : "hover:bg-muted",
  );
}

/** Menu lateral fixo e permanente. Módulos futuros aparecem desabilitados. */
export default function Sidebar() {
  const role = useSessionStore((s) => s.user?.role);

  return (
    <nav
      aria-label="Menu principal"
      className="flex w-56 shrink-0 flex-col gap-1 border-r bg-card p-3"
    >
      <p className="px-2 pb-2 text-sm font-semibold">Boreal PMS</p>
      <NavLink to="/" end className={linkClass}>
        Dashboard
      </NavLink>
      <NavLink to="/mapa" className={linkClass}>
        Mapa de quartos
      </NavLink>
      <NavLink to="/reservas" className={linkClass}>
        Reservas
      </NavLink>
      <NavLink to="/hospedagens" className={linkClass}>
        Hospedagens
      </NavLink>
      <NavLink to="/limpeza" className={linkClass}>
        Limpeza
      </NavLink>
      <NavLink to="/bloqueios" className={linkClass}>
        Bloqueios
      </NavLink>
      <NavLink to="/manutencao" className={linkClass}>
        Manutenção
      </NavLink>
      <NavLink to="/hospedes" className={linkClass}>
        Hóspedes
      </NavLink>
      <NavLink to="/quartos" className={linkClass}>
        Quartos
      </NavLink>
      <NavLink to="/tarifas" className={linkClass}>
        Tarifas
      </NavLink>
      <NavLink to="/itens" className={linkClass}>
        Itens e serviços
      </NavLink>
      {upcoming.map((item) => (
        <span
          key={item}
          aria-disabled="true"
          title="Em breve"
          className="cursor-not-allowed rounded-md px-2 py-1.5 text-sm text-muted-foreground"
        >
          {item}
        </span>
      ))}
      {role === "admin" && (
        <>
          <NavLink to="/usuarios" className={linkClass}>
            Usuários
          </NavLink>
          <NavLink to="/configuracoes" className={linkClass}>
            Configurações
          </NavLink>
        </>
      )}
    </nav>
  );
}
