import { useEffect } from "react";

import { getSessionPolicy } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

/**
 * Minutos de inatividade para o bloqueio da sessão. Vem do backend ao entrar e é atualizado
 * quando um administrador altera a configuração (`setIdleMinutes`).
 */
export function useSessionPolicy(token: string | null): number {
  const idleMinutes = useSessionStore((s) => s.idleMinutes);
  const setIdleMinutes = useSessionStore((s) => s.setIdleMinutes);

  useEffect(() => {
    if (!token) return;
    let cancelled = false;
    Promise.resolve(getSessionPolicy(token))
      .then((policy) => {
        if (!cancelled && policy.idle_lock_minutes > 0) setIdleMinutes(policy.idle_lock_minutes);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [token, setIdleMinutes]);

  return idleMinutes;
}
