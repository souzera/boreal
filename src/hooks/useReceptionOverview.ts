import { useQuery } from "@tanstack/react-query";

import { receptionOverview, refreshAlerts } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

/** Resumo da Recepção; refaz a cada minuto com a sessão ativa e mantém o que já estava na tela se falhar. */
export function useReceptionOverview() {
  const token = useSessionStore((s) => s.token) ?? "";
  const active = useSessionStore((s) => s.state === "active");
  return useQuery({
    queryKey: ["reception-overview"],
    queryFn: async () => {
      try {
        // Os riscos vêm dos alertas; reavalia antes de ler, como o resumo de alertas faz.
        await refreshAlerts(token);
      } catch {
        // a reavaliação falhou: a leitura abaixo mostra o que já está salvo
      }
      return receptionOverview(token);
    },
    enabled: !!token && active,
    refetchInterval: 60_000,
    retry: false,
  });
}
