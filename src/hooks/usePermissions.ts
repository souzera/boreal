import { useQuery } from "@tanstack/react-query";

import { myPermissions } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

/**
 * Permissões da sessão atual, só para decidir o que oferecer na tela.
 * Quem de fato autoriza cada operação é o backend.
 */
export function usePermissions() {
  const token = useSessionStore((s) => s.token);
  const { data = [] } = useQuery({
    queryKey: ["my-permissions", token],
    queryFn: () => myPermissions(token ?? ""),
    enabled: !!token,
    staleTime: 60_000,
  });
  return { can: (permission: string) => data.includes(permission), permissions: data };
}
