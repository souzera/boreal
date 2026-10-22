import type { ReactNode } from "react";

import { useActivityHeartbeat } from "@/hooks/useActivityHeartbeat";
import { useIdleLock } from "@/hooks/useIdleLock";
import { useSessionPolicy } from "@/hooks/useSessionPolicy";
import { LockContext } from "@/lib/lock-context";
import LockScreen from "@/pages/LockScreen";
import SessionExpired from "@/pages/SessionExpired";
import { useSessionStore } from "@/stores/session";

/**
 * Envolve qualquer área autenticada. Bloqueada por inatividade (ou com a sessão encerrada
 * pelo backend), a área continua montada, mas oculta e inerte, e os diálogos abertos deixam
 * de ser exibidos; a tela de bloqueio cobre tudo.
 */
export default function LockGate({ children }: { children: ReactNode }) {
  const { token, state, lock } = useSessionStore();
  const covered = state === "locked" || state === "expired";
  const idleMinutes = useSessionPolicy(token);
  useIdleLock(idleMinutes, lock, !covered);
  useActivityHeartbeat(token, !covered);

  return (
    <>
      <LockContext.Provider value={covered}>
        <div hidden={covered} inert={covered}>
          {children}
        </div>
      </LockContext.Provider>
      {state === "locked" && <LockScreen />}
      {state === "expired" && <SessionExpired />}
    </>
  );
}
