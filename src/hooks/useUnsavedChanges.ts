import { useEffect } from "react";
import { useBlocker } from "react-router-dom";

/**
 * Bloqueia a saída da tela (navegação interna e fechamento da janela) enquanto houver
 * alterações não salvas. Exige um roteador de dados (`createHashRouter`).
 */
export function useUnsavedChanges(isDirty: boolean) {
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      isDirty && currentLocation.pathname !== nextLocation.pathname,
  );

  useEffect(() => {
    if (!isDirty) return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [isDirty]);

  return blocker;
}
