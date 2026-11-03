import { useQuery } from "@tanstack/react-query";

import { alertSummary, refreshAlerts } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

/**
 * Resumo dos alertas ativos. Pede ao backend para reavaliar os alertas e a cada minuto repete a consulta
 * enquanto a sessão está ativa; falha de uma das chamadas nunca derruba a tela.
 */
export function useAlertSummary() {
  const token = useSessionStore((s) => s.token) ?? "";
  const active = useSessionStore((s) => s.state === "active");
  return useQuery({
    queryKey: ["alert-summary"],
    queryFn: async () => {
      try {
        await refreshAlerts(token);
      } catch {
        // a reavaliação falhou: o resumo abaixo mostra o que já está salvo
      }
      return alertSummary(token);
    },
    enabled: !!token && active,
    refetchInterval: 60_000,
    retry: false,
  });
}
