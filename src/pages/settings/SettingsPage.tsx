import { useCallback, useState } from "react";

import { PageHeader } from "@/components/PageHeader";
import { UnsavedChangesGuard } from "@/components/UnsavedChangesGuard";
import AboutCard from "./AboutCard";
import PousadaForm from "./PousadaForm";
import SystemForm from "./SystemForm";

export default function SettingsPage() {
  const [dirty, setDirty] = useState({ pousada: false, system: false });
  const onPousadaDirty = useCallback((pousada: boolean) => setDirty((d) => ({ ...d, pousada })), []);
  const onSystemDirty = useCallback((system: boolean) => setDirty((d) => ({ ...d, system })), []);

  return (
    <section className="grid max-w-4xl gap-6">
      <PageHeader title="Configurações" description="Dados da pousada e regras de funcionamento do sistema." />
      <PousadaForm onDirtyChange={onPousadaDirty} />
      <SystemForm onDirtyChange={onSystemDirty} />
      <AboutCard />
      <UnsavedChangesGuard isDirty={dirty.pousada || dirty.system} />
    </section>
  );
}
