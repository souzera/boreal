import { useEffect, useState } from "react";

import { getSessionPolicy } from "@/lib/api";

// Valor provisório usado até o backend responder (spec §13.3, pendência §25.5).
const DEFAULT_IDLE_MINUTES = 10;

/** Minutos de inatividade configurados no backend para o bloqueio da sessão. */
export function useSessionPolicy(token: string | null): number {
  const [idleMinutes, setIdleMinutes] = useState(DEFAULT_IDLE_MINUTES);

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
  }, [token]);

  return idleMinutes;
}
