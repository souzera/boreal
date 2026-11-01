import { NavLink } from "react-router-dom";

import Icon from "@/components/Icon";
import { useAlertSummary } from "@/hooks/useAlertSummary";
import { usePermissions } from "@/hooks/usePermissions";
import { alertCountLabel } from "@/lib/alerts";
import { cn } from "@/lib/utils";
import { useSessionStore } from "@/stores/session";
import { NAV, visibleNav, type NavLeaf } from "./NavItems";

const itemBase = "flex items-center gap-2.5 rounded-lg border-l-[3px] px-2.5 py-1.5 text-sm";

function Leaf({ leaf, alerts }: { leaf: NavLeaf; alerts?: { critical: number; total: number } }) {
  if (leaf.soon || !leaf.to) {
    return (
      <span
        aria-disabled="true"
        title="Em breve"
        className={cn(itemBase, "cursor-not-allowed border-transparent text-muted-foreground")}
      >
        <Icon name={leaf.icon} size="md" />
        <span>{leaf.label}</span>
        <span className="ml-auto rounded-md bg-muted px-1.5 text-[11px]">Em breve</span>
      </span>
    );
  }
  return (
    <NavLink
      to={leaf.to}
      end={leaf.end}
      className={({ isActive }) =>
        cn(
          itemBase,
          isActive
            ? "border-primary bg-primary-selected font-medium text-primary"
            : "border-transparent text-foreground hover:bg-muted",
        )
      }
    >
      <Icon name={leaf.icon} size="md" />
      {leaf.label}
      {alerts && alerts.total > 0 && (
        <>
          <span
            aria-hidden="true"
            className={cn(
              "ml-auto inline-flex items-center gap-0.5 rounded-md px-1.5 text-[11px] font-medium",
              alerts.critical > 0 ? "bg-danger-subtle text-danger-foreground" : "bg-muted text-foreground",
            )}
          >
            {alerts.critical > 0 && <Icon name="error" size="xs" />}
            {alerts.total}
          </span>
          <span className="sr-only">{alertCountLabel(alerts)}</span>
        </>
      )}
    </NavLink>
  );
}

/** Menu lateral fixo e permanente, em grupos. Cada item combina ícone, rótulo e (se ativo) indicador lateral. */
export default function Sidebar() {
  const role = useSessionStore((s) => s.user?.role);
  const { can } = usePermissions();
  const entries = visibleNav(NAV, { role, can });
  const { data: summary } = useAlertSummary();

  return (
    <nav aria-label="Menu principal" className="flex w-60 shrink-0 flex-col gap-1 border-r bg-sidebar p-3">
      <p className="px-2.5 pb-3 pt-1 text-lg font-semibold text-primary">Boreal PMS</p>
      {entries.map((entry) =>
        "items" in entry ? (
          <div key={entry.label} role="group" aria-label={entry.label} className="mt-2 grid gap-0.5">
            <p className="px-2.5 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">{entry.label}</p>
            {entry.items.map((leaf) => (
              <Leaf key={leaf.label} leaf={leaf} alerts={leaf.to === "/alertas" ? summary : undefined} />
            ))}
          </div>
        ) : (
          <Leaf key={entry.label} leaf={entry} alerts={entry.to === "/alertas" ? summary : undefined} />
        ),
      )}
    </nav>
  );
}
