import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { useSessionStore } from "@/stores/session";
import { BackupCard } from "./BackupCard";
import { DiskCard } from "./DiskCard";
import { RecoveryKeyCard } from "./RecoveryKeyCard";
import { VersionsCard } from "./VersionsCard";

/** Saúde do sistema (só administrador): backup, restauração, chave de recuperação, disco e versões. */
export default function HealthPage() {
  const isAdmin = useSessionStore((s) => s.user?.role === "admin");
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  if (!isAdmin) {
    return (
      <section>
        <PageHeader title="Saúde do sistema" description="Esta área é só para administradores." />
      </section>
    );
  }

  // Depois de restaurar, todas as sessões foram encerradas: volta ao login.
  const afterRestore = () => {
    useSessionStore.getState().clear();
    // Os dados em memória eram do banco de antes.
    queryClient.clear();
    navigate("/login", { replace: true });
  };

  return (
    <section className="grid gap-4">
      <PageHeader title="Saúde do sistema" description="Backup, restauração, chave de recuperação e espaço em disco." />
      <div className="grid gap-4 lg:grid-cols-2">
        <div className="grid content-start gap-4">
          <BackupCard onRestored={afterRestore} />
        </div>
        <div className="grid content-start gap-4">
          <RecoveryKeyCard />
          <DiskCard />
          <VersionsCard />
        </div>
      </div>
    </section>
  );
}
