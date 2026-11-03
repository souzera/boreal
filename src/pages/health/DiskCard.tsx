import { useQuery } from "@tanstack/react-query";

import Icon from "@/components/Icon";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { diskStatus } from "@/lib/api";
import { formatBytes } from "@/lib/backup";
import { useSessionStore } from "@/stores/session";

/** Espaço livre na pasta do banco; a situação sempre em texto e ícone. */
export function DiskCard() {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data, isError } = useQuery({ queryKey: ["disk-status"], queryFn: () => diskStatus(token), enabled: !!token, retry: false });

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2 className="text-base font-medium">Espaço em disco</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-1 text-sm">
        {isError && <p role="alert">Não foi possível consultar o espaço em disco.</p>}
        {data && (
          <>
            <p className={data.low ? "flex items-center gap-1 font-medium text-destructive" : "flex items-center gap-1 font-medium"}>
              <Icon name={data.low ? "error" : "check_circle"} size="sm" />
              {data.low ? "Pouco espaço em disco" : "Espaço suficiente"}
            </p>
            <p>{`Livre: ${formatBytes(data.free_bytes)}. O recomendado é pelo menos ${formatBytes(data.threshold_bytes)}.`}</p>
            <p className="text-muted-foreground">{`Tamanho do banco de dados: ${formatBytes(data.db_bytes)}.`}</p>
            {data.low && <p>Libere espaço no computador ou escolha outra pasta para os backups.</p>}
          </>
        )}
      </CardContent>
    </Card>
  );
}
