import { NavLink } from "react-router-dom";

import { cn } from "@/lib/utils";

const upcoming = ["Reservas", "Hóspedes", "Quartos", "Financeiro", "Relatórios", "Configurações"];

/** Menu lateral fixo e permanente. Módulos futuros aparecem desabilitados. */
export default function Sidebar() {
  return (
    <nav
      aria-label="Menu principal"
      className="flex w-56 shrink-0 flex-col gap-1 border-r bg-card p-3"
    >
      <p className="px-2 pb-2 text-sm font-semibold">Boreal PMS</p>
      <NavLink
        to="/"
        end
        className={({ isActive }) =>
          cn(
            "rounded-md px-2 py-1.5 text-sm",
            isActive ? "bg-primary text-primary-foreground" : "hover:bg-muted",
          )
        }
      >
        Dashboard
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
    </nav>
  );
}
