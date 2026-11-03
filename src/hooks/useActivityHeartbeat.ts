import { useEffect } from "react";

import { sessionStatus } from "@/lib/api";

const EVENTS = ["mousemove", "mousedown", "keydown", "wheel", "touchstart"] as const;
const MIN_INTERVAL_MS = 30_000;

/**
 * Avisa o backend da atividade do usuário (no máximo a cada 30 s), mantendo o
 * relógio de inatividade do backend alinhado com o da interface.
 */
export function useActivityHeartbeat(token: string | null, enabled = true) {
  useEffect(() => {
    if (!enabled || !token) return;
    let last = 0;
    const ping = () => {
      const now = Date.now();
      if (now - last < MIN_INTERVAL_MS) return;
      last = now;
      // Erros (inclusive "locked") são tratados em lib/api; aqui só importa a atividade.
      sessionStatus(token).catch(() => undefined);
    };
    EVENTS.forEach((e) => window.addEventListener(e, ping, { passive: true }));
    return () => EVENTS.forEach((e) => window.removeEventListener(e, ping));
  }, [token, enabled]);
}
