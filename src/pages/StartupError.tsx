import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export default function StartupError({ message }: { message: string }) {
  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Não foi possível iniciar o sistema</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-2 text-sm">
          <p role="alert">{message}</p>
          <p className="text-muted-foreground">
            Nenhum dado foi alterado. Feche o aplicativo e procure o administrador.
          </p>
        </CardContent>
      </Card>
    </main>
  );
}
