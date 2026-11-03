import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import RecoverFromBackup from "./RecoverFromBackup";

interface Props {
  message: string;
  /** Código do erro de inicialização (`integrity`, `key_unavailable`, `update_failed`…). */
  code?: string;
}

export default function StartupError({ message, code }: Props) {
  // Banco com problema de integridade: oferece restaurar um backup (modo de recuperação).
  if (code === "integrity") return <RecoverFromBackup mode="corrupted" />;
  // Chave não disponível neste computador: a chave de recuperação reabre o banco que já existe.
  if (code === "key_unavailable" || code === "key_invalid") return <RecoverFromBackup mode="unlock" />;

  return (
    <main className="grid min-h-[calc(100vh-var(--titlebar-h))] place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Não foi possível iniciar o sistema</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-2 text-sm">
          <p role="alert">{message}</p>
          <p className="text-muted-foreground">
            {code === "update_failed" || code === "update_backup_failed"
              ? "Seus dados estão preservados. Feche o aplicativo e procure o administrador."
              : "Nenhum dado foi alterado. Feche o aplicativo e procure o administrador."}
          </p>
        </CardContent>
      </Card>
    </main>
  );
}
