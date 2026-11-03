import { useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { recoverCorrupted, restoreOnNewComputer, unlockWithRecoveryKey, type ApiError } from "@/lib/api";
import { CONFIRMATION_WORD } from "@/lib/backup";

type Mode = "new-computer" | "corrupted" | "unlock";

interface Props {
  mode: Mode;
  /** Volta à tela anterior (só na troca de computador). */
  onBack?: () => void;
}

const TEXT: Record<Mode, { title: string; intro: string; action: string }> = {
  "new-computer": {
    title: "Restaurar de um backup",
    intro: "Use este caminho ao trocar de computador: escolha o arquivo de backup e digite a chave de recuperação.",
    action: "Restaurar neste computador",
  },
  corrupted: {
    title: "Recuperar o banco de dados",
    intro: "O banco de dados deste computador tem problemas. Escolha um backup e digite a chave de recuperação. O banco com problema é guardado ao lado, sem ser apagado.",
    action: "Restaurar o backup",
  },
  unlock: {
    title: "Desbloquear com a chave de recuperação",
    intro: "A chave guardada neste computador não está disponível. Digite a chave de recuperação para reabrir o banco de dados que já existe aqui.",
    action: "Desbloquear",
  },
};

/** Telas de recuperação sem login: troca de computador, banco com problema e chave perdida (spec §14, §16.2). */
export default function RecoverFromBackup({ mode, onBack }: Props) {
  const queryClient = useQueryClient();
  const [file, setFile] = useState<string | null>(null);
  const [key, setKey] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const text = TEXT[mode];
  const needsFile = mode !== "unlock";
  const needsConfirmation = mode === "corrupted";
  const ready = key.trim().length > 0 && (!needsFile || !!file) && (!needsConfirmation || confirmation.trim().toUpperCase() === CONFIRMATION_WORD) && !busy;

  const chooseFile = async () => {
    try {
      const chosen = await open({ multiple: false, title: "Escolha o arquivo de backup", filters: [{ name: "Backup do Boreal", extensions: ["bak"] }] });
      if (typeof chosen === "string") setFile(chosen);
    } catch {
      setError("Não foi possível abrir a janela de escolha do arquivo.");
    }
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      if (mode === "new-computer") await restoreOnNewComputer(file as string, key);
      else if (mode === "corrupted") await recoverCorrupted(file as string, key, confirmation);
      else await unlockWithRecoveryKey(key);
      await queryClient.invalidateQueries({ queryKey: ["startup"] });
    } catch (e) {
      setError((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="grid min-h-[calc(100vh-var(--titlebar-h))] place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>
            <h1 className="text-base font-medium">{text.title}</h1>
          </CardTitle>
          <p className="text-sm text-muted-foreground">{text.intro}</p>
        </CardHeader>
        <CardContent className="grid gap-3 text-sm">
          {needsFile && (
            <div className="grid gap-1">
              <div className="flex flex-wrap items-center gap-2">
                <Button type="button" variant="outline" onClick={() => void chooseFile()}>
                  Escolher arquivo de backup
                </Button>
                <span className="break-all text-muted-foreground">{file ? file.split(/[\\/]/).pop() : "Nenhum arquivo escolhido."}</span>
              </div>
            </div>
          )}
          <label className="grid gap-1">
            Chave de recuperação
            <Input value={key} autoComplete="off" spellCheck={false} placeholder="XXXXXXXX-XXXXXXXX-…" onChange={(e) => setKey(e.target.value)} />
          </label>
          {needsConfirmation && (
            <label className="grid gap-1">
              {`Digite ${CONFIRMATION_WORD} para confirmar`}
              <Input value={confirmation} autoComplete="off" onChange={(e) => setConfirmation(e.target.value)} />
            </label>
          )}
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button disabled={!ready} onClick={() => void submit()}>
              {text.action}
            </Button>
            {onBack && (
              <Button type="button" variant="outline" onClick={onBack}>
                Voltar
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
    </main>
  );
}
