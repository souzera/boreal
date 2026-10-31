import { useLocation, useNavigate } from "react-router-dom";

import Icon from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { SHORTCUT_DESTINATIONS, SHORTCUTS, useGlobalShortcuts } from "@/hooks/useGlobalShortcuts";
import { useSessionStore } from "@/stores/session";

interface Props {
  /** Leva o foco ao campo de busca. */
  onSearch: () => void;
}

/** Ações de alto uso do cabeçalho, com atalho de teclado (brief §2). */
export default function GlobalActions({ onSearch }: Props) {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const covered = useSessionStore((s) => s.state === "locked" || s.state === "expired");
  // Repetir o atalho na própria tela não empilha entradas no histórico.
  const go = (to: string) => {
    if (to !== pathname) navigate(to);
  };
  useGlobalShortcuts({ onSearch, onNavigate: go, enabled: !covered });

  const actions = [
    { label: "Nova reserva", icon: "add", shortcut: SHORTCUTS.newReservation, aria: "Alt+N", run: () => go(SHORTCUT_DESTINATIONS.newReservation), primary: true },
    { label: "Buscar", icon: "search", shortcut: SHORTCUTS.search, aria: "Control+K", run: onSearch },
    { label: "Check-in", icon: "login", shortcut: SHORTCUTS.checkIn, aria: "Alt+C", run: () => go(SHORTCUT_DESTINATIONS.checkIn) },
    { label: "Receber pagamento", icon: "payments", shortcut: SHORTCUTS.receivePayment, aria: "Alt+R", run: () => go(SHORTCUT_DESTINATIONS.receivePayment) },
    { label: "Check-out", icon: "logout", shortcut: SHORTCUTS.checkOut, aria: "Alt+S", run: () => go(SHORTCUT_DESTINATIONS.checkOut) },
  ];

  return (
    <div className="flex flex-wrap items-center gap-1.5" role="toolbar" aria-label="Ações rápidas">
      {actions.map((a) => (
        <Button
          key={a.label}
          type="button"
          size="sm"
          variant={a.primary ? "default" : "outline"}
          title={`${a.label} (${a.shortcut})`}
          aria-keyshortcuts={a.aria}
          onClick={a.run}
        >
          <Icon name={a.icon} size="sm" />
          <span className="max-xl:sr-only">{a.label}</span>
        </Button>
      ))}
    </div>
  );
}
