import type { ReactNode } from "react";

import { useActivityHeartbeat } from "@/hooks/useActivityHeartbeat";
import { useIdleLock } from "@/hooks/useIdleLock";
import LockScreen from "@/pages/LockScreen";
import { useSessionStore } from "@/stores/session";

// Valor padrão provisório; o backend aplica o limite configurado (spec §13.3, pendência §25.5).
const IDLE_MINUTES = 10;

/**
 * Envolve qualquer área autenticada. Bloqueada por inatividade, a área continua montada
 * (para não perder o que está sendo editado), mas oculta e inerte; a tela de bloqueio cobre tudo.
 */
export default function LockGate({ children }: { children: ReactNode }) {
  const { token, state, lock } = useSessionStore();
  const locked = state === "locked";
  useIdleLock(IDLE_MINUTES, lock, !locked);
  useActivityHeartbeat(token, !locked);

  return (
    <>
      <div hidden={locked} inert={locked}>
        {children}
      </div>
      {locked && <LockScreen />}
    </>
  );
}
