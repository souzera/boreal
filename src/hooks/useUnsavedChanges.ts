import { useEffect } from "react";
import { useBlocker } from "react-router-dom";

/**
 * Bloqueia a navegação interna (inclusive o botão Sair) enquanto houver alterações não
 * salvas e pede confirmação ao recarregar a página. Fechar a janela do aplicativo ainda
 * não é interceptado (pendente: `onCloseRequested` do Tauri). Exige um roteador de dados.
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
