import { useQuery } from "@tanstack/react-query";

import Icon from "@/components/Icon";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getAppInfo, lastStartupReport, logFolder } from "@/lib/api";
import { formatInstant } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

/** Versões, resultado da verificação feita ao abrir e a pasta dos logs técnicos. */
export function VersionsCard() {
  const token = useSessionStore((s) => s.token) ?? "";
  const info = useQuery({ queryKey: ["app-info"], queryFn: getAppInfo });
  const report = useQuery({ queryKey: ["startup-report"], queryFn: () => lastStartupReport(token), enabled: !!token, retry: false });
  const logs = useQuery({ queryKey: ["log-folder"], queryFn: () => logFolder(token), enabled: !!token, retry: false });

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2 className="text-base font-medium">Versões e verificação</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-2 text-sm">
        {info.data && (
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt className="text-muted-foreground">Versão do aplicativo</dt>
            <dd>{info.data.app_version}</dd>
            <dt className="text-muted-foreground">Versão do banco de dados</dt>
            <dd>{info.data.schema_version}</dd>
          </dl>
        )}
        {report.data && (
          <p className="flex items-center gap-1">
            <Icon name={report.data.integrity_ok ? "check_circle" : "error"} size="sm" />
            {report.data.integrity_ok
              ? `Verificação ao abrir o sistema: sem problemas (${formatInstant(report.data.checked_at)}).`
              : `Verificação ao abrir o sistema: ${report.data.problems.length} problema(s) encontrado(s).`}
          </p>
        )}
        {logs.data && (
          <p>
            <span className="text-muted-foreground">Pasta dos logs técnicos: </span>
            <span className="break-all font-medium">{logs.data}</span>
          </p>
        )}
      </CardContent>
    </Card>
  );
}
