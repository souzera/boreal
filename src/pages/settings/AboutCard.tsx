import { useQuery } from "@tanstack/react-query";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getAppInfo } from "@/lib/api";
import { formatDate, toLocalIsoDate } from "@/lib/format";

/** Versão do aplicativo, do banco e data da última atualização concluída (spec §16.2). */
export default function AboutCard() {
  const { data } = useQuery({ queryKey: ["app-info"], queryFn: getAppInfo });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Sobre o sistema</CardTitle>
      </CardHeader>
      <CardContent>
        {data && (
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
            <dt className="text-muted-foreground">Versão do aplicativo</dt>
            <dd>{data.app_version}</dd>
            <dt className="text-muted-foreground">Versão do banco de dados</dt>
            <dd>{data.schema_version}</dd>
            <dt className="text-muted-foreground">Última atualização</dt>
            <dd>
              {data.last_migration_at
                ? formatDate(toLocalIsoDate(new Date(data.last_migration_at)))
                : "—"}
            </dd>
          </dl>
        )}
      </CardContent>
    </Card>
  );
}
