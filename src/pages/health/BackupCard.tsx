import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import Icon from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { backupDestination, listBackupFiles, recentBackupRuns, runBackup, setBackupDestination, type ApiError } from "@/lib/api";
import { formatBytes } from "@/lib/backup";
import { formatInstant } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import { RestoreDialog } from "./RestoreDialog";

interface Props {
  /** Depois de restaurar, a página volta ao login. */
  onRestored: () => void;
}

/** Pasta de destino, backup manual, histórico e a lista de arquivos para restaurar. */
export function BackupCard({ onRestored }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [restoring, setRestoring] = useState<string | null>(null);

  const destination = useQuery({ queryKey: ["backup-destination"], queryFn: () => backupDestination(token), enabled: !!token });
  const runs = useQuery({ queryKey: ["backup-runs"], queryFn: () => recentBackupRuns(token, 10), enabled: !!token });
  const files = useQuery({ queryKey: ["backup-files"], queryFn: () => listBackupFiles(token), enabled: !!token });

  const refresh = () => {
    for (const key of ["backup-destination", "backup-runs", "backup-files", "alert-summary", "disk-status"]) {
      void queryClient.invalidateQueries({ queryKey: [key] });
    }
  };

  const backupNow = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const run = await runBackup(token);
      setMessage({ ok: true, text: `Backup concluído: ${run.file_name ?? ""}` });
    } catch (e) {
      setMessage({ ok: false, text: (e as ApiError).message });
    } finally {
      setBusy(false);
      refresh();
    }
  };

  const chooseFolder = async () => {
    try {
      const chosen = await open({ directory: true, multiple: false, title: "Escolha a pasta dos backups" });
      if (typeof chosen !== "string") return;
      await setBackupDestination(token, chosen);
      setMessage({ ok: true, text: "Pasta de backup alterada." });
      refresh();
    } catch (e) {
      setMessage({ ok: false, text: (e as ApiError).message ?? "Não foi possível escolher a pasta." });
    }
  };

  const useDefault = async () => {
    try {
      await setBackupDestination(token, "");
      setMessage({ ok: true, text: "Voltou à pasta padrão." });
      refresh();
    } catch (e) {
      setMessage({ ok: false, text: (e as ApiError).message });
    }
  };

  const chooseFile = async () => {
    try {
      const chosen = await open({ multiple: false, title: "Escolha o arquivo de backup", filters: [{ name: "Backup do Boreal", extensions: ["bak"] }] });
      if (typeof chosen === "string") setRestoring(chosen);
    } catch {
      setMessage({ ok: false, text: "Não foi possível abrir a janela de escolha do arquivo." });
    }
  };

  const folder = destination.data?.path ?? "";
  const pathOf = (name: string) => `${folder.replace(/[\\/]+$/, "")}${folder.includes("\\") ? "\\" : "/"}${name}`;

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2 className="text-base font-medium">Backup</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4 text-sm">
        <div className="grid gap-1">
          <p>
            <span className="text-muted-foreground">Pasta dos backups: </span>
            <span className="font-medium break-all">{folder}</span>
            {destination.data?.is_default && " (padrão, ao lado do banco)"}
          </p>
          <p className="text-muted-foreground">Guarde cópias também em outro dispositivo (pen drive ou disco externo).</p>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={() => void chooseFolder()}>
              Escolher pasta
            </Button>
            {destination.data && !destination.data.is_default && (
              <Button variant="outline" size="sm" onClick={() => void useDefault()}>
                Usar a pasta padrão
              </Button>
            )}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button disabled={busy} onClick={() => void backupNow()}>
            Fazer backup agora
          </Button>
          {message && (
            <p role={message.ok ? "status" : "alert"} className={message.ok ? "" : "text-destructive"}>
              {message.text}
            </p>
          )}
        </div>

        <section aria-label="Últimos backups" className="grid gap-1">
          <h3 className="font-semibold">Últimos backups</h3>
          {runs.data && runs.data.length === 0 && <p className="text-muted-foreground">Nenhum backup foi feito ainda.</p>}
          <ul className="grid gap-1">
            {(runs.data ?? []).map((r) => (
              <li key={r.id} className="flex flex-wrap items-center gap-2">
                <span className={r.status === "ok" ? "inline-flex items-center gap-1" : "inline-flex items-center gap-1 text-destructive"}>
                  <Icon name={r.status === "ok" ? "check_circle" : "error"} size="xs" />
                  {r.status === "ok" ? "Válido" : "Falhou"}
                </span>
                <span>{`${r.kind_label} · ${formatInstant(r.finished_at)}`}</span>
                {r.status === "ok" ? <span className="text-muted-foreground">{formatBytes(r.bytes)}</span> : <span className="text-destructive">{r.message}</span>}
              </li>
            ))}
          </ul>
        </section>

        <section aria-label="Arquivos de backup" className="grid gap-1">
          <h3 className="font-semibold">Restaurar um backup</h3>
          {files.data && files.data.length === 0 && <p className="text-muted-foreground">Não há arquivos de backup na pasta.</p>}
          <ul className="grid gap-1">
            {(files.data ?? []).map((f) => (
              <li key={f.file_name} className="flex flex-wrap items-center gap-2">
                <span>{`${f.at} · ${f.safety ? "cópia de segurança" : "backup"} · ${formatBytes(f.bytes)}`}</span>
                <Button size="sm" variant="outline" onClick={() => setRestoring(pathOf(f.file_name))}>
                  {`Restaurar o backup de ${f.at}`}
                </Button>
              </li>
            ))}
          </ul>
          <div>
            <Button size="sm" variant="outline" onClick={() => void chooseFile()}>
              Escolher outro arquivo
            </Button>
          </div>
        </section>
      </CardContent>
      <RestoreDialog
        path={restoring}
        onClose={() => setRestoring(null)}
        onRestored={() => {
          setRestoring(null);
          onRestored();
        }}
      />
    </Card>
  );
}
