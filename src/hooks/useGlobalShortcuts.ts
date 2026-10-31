import { useEffect, useLayoutEffect, useRef } from "react";

/** Atalhos globais mostrados nos botões do cabeçalho (a ação também existe visualmente). */
export const SHORTCUTS = {
  search: "Ctrl+K",
  newReservation: "Alt+N",
  checkIn: "Alt+C",
  receivePayment: "Alt+R",
  checkOut: "Alt+S",
} as const;

/** Destinos das ações globais: a tela de cada fluxo específico chega junto com a Recepção (6.3). */
export const SHORTCUT_DESTINATIONS = {
  newReservation: "/reservas/nova",
  checkIn: "/reservas",
  receivePayment: "/hospedagens",
  checkOut: "/hospedagens",
} as const;

const ALT_KEYS: Record<string, keyof typeof SHORTCUT_DESTINATIONS> = {
  KeyN: "newReservation",
  KeyC: "checkIn",
  KeyR: "receivePayment",
  KeyS: "checkOut",
};

interface Options {
  onSearch: () => void;
  onNavigate: (to: string) => void;
  /** Desligado com a sessão bloqueada. */
  enabled?: boolean;
}

/**
 * Atalhos de teclado do cabeçalho: `Ctrl+K` busca; `Alt+N/C/R/S` abrem nova reserva, check-in, receber
 * pagamento e check-out. Não disparam com diálogo aberto, com tecla repetida nem com outros modificadores.
 * A tecla é reconhecida pelo código físico, porque o Alt pode trocar o caractere em alguns teclados.
 */
export function useGlobalShortcuts({ onSearch, onNavigate, enabled = true }: Options): void {
  const handlers = useRef({ onSearch, onNavigate });
  useLayoutEffect(() => {
    handlers.current = { onSearch, onNavigate };
  });

  useEffect(() => {
    if (!enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.repeat || event.metaKey || document.querySelector('[role="dialog"], [role="alertdialog"]')) return;
      if (event.ctrlKey && !event.altKey && !event.shiftKey && (event.code === "KeyK" || event.key.toLowerCase() === "k")) {
        event.preventDefault();
        handlers.current.onSearch();
        return;
      }
      if (event.altKey && !event.ctrlKey && !event.shiftKey) {
        const action = ALT_KEYS[event.code];
        if (!action) return;
        event.preventDefault();
        handlers.current.onNavigate(SHORTCUT_DESTINATIONS[action]);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [enabled]);
}
