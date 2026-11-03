import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { revealRecoveryKey, type ApiError } from "@/lib/api";
import { keyGroups } from "@/lib/backup";
import { useSessionStore } from "@/stores/session";

/** Chave de recuperação (spec §14): só aparece depois de digitar a senha de novo; nunca fica guardada na tela. */
export function RecoveryKeyCard() {
  const token = useSessionStore((s) => s.token) ?? "";
  const [asking, setAsking] = useState(false);
  const [password, setPassword] = useState("");
  const [key, setKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reveal = async () => {
    setBusy(true);
    setError(null);
    try {
      setKey(await revealRecoveryKey(token, password));
      setAsking(false);
    } catch (e) {
      setError((e as ApiError).message);
    } finally {
      setPassword("");
      setBusy(false);
    }
  };

  const hide = () => {
    setKey(null);
    setAsking(false);
    setError(null);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2 className="text-base font-medium">Chave de recuperação</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <p>
          É a chave que abre o banco e os backups. <strong>Guarde-a impressa, fora do computador.</strong> Sem ela e sem este computador, os dados não podem ser recuperados.
        </p>
        {!key && !asking && (
          <div>
            <Button variant="outline" onClick={() => setAsking(true)}>
              Mostrar chave de recuperação
            </Button>
          </div>
        )}
        {asking && (
          <form
            className="grid max-w-sm gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              void reveal();
            }}
          >
            <label className="grid gap-1">
              Digite sua senha para ver a chave
              <Input type="password" value={password} autoComplete="current-password" onChange={(e) => setPassword(e.target.value)} />
            </label>
            <div className="flex gap-2">
              <Button type="submit" disabled={busy || password.length === 0}>
                Mostrar
              </Button>
              <Button type="button" variant="outline" onClick={hide}>
                Cancelar
              </Button>
            </div>
          </form>
        )}
        {error && (
          <p role="alert" className="text-destructive">
            {error}
          </p>
        )}
        {key && (
          <div className="grid gap-2">
            <ul aria-label="Chave de recuperação" className="grid grid-cols-3 gap-2 font-mono text-base print:grid-cols-3">
              {keyGroups(key).map((group, i) => (
                <li key={i} className="rounded border bg-muted px-2 py-1 text-center">
                  {group}
                </li>
              ))}
            </ul>
            <p className="text-muted-foreground">Os 9 grupos, na ordem. O último grupo é só de conferência.</p>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => window.print()}>
                Imprimir
              </Button>
              <Button variant="outline" onClick={hide}>
                Ocultar chave
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
