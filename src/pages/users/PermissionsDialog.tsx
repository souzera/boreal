import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { grantablePermissions, type ApiError, type UserRow } from "@/lib/api";
import { permissionLabel } from "./labels";

interface Props {
  user: UserRow | null;
  onClose: () => void;
  onSubmit: (user: UserRow, permissions: string[]) => Promise<void>;
}

/** Permissões individuais para a recepção; só aparecem as que o backend permite conceder. */
export default function PermissionsDialog({ user, onClose, onSubmit }: Props) {
  const { data: grantable = [] } = useQuery({ queryKey: ["grantable-permissions"], queryFn: grantablePermissions });
  const [selected, setSelected] = useState<string[]>([]);
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setSelected(user?.extra_permissions ?? []);
    setFormError(null);
  }, [user]);

  const original = user?.extra_permissions ?? [];
  const isDirty =
    selected.length !== original.length || selected.some((name) => !original.includes(name));
  const { requestClose, confirm } = useGuardedClose(isDirty, onClose);

  const toggle = (name: string, checked: boolean) =>
    setSelected((current) => (checked ? [...current, name] : current.filter((n) => n !== name)));

  const save = async () => {
    if (!user) return;
    setSaving(true);
    setFormError(null);
    try {
      await onSubmit(user, selected);
    } catch (e) {
      setFormError((e as ApiError).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
    <Dialog open={user !== null} onOpenChange={(next) => !next && requestClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Permissões de {user?.display_name}</DialogTitle>
          <DialogDescription>
            A recepção faz o dia a dia sem essas permissões. As mudanças valem no próximo acesso da pessoa.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3">
          {grantable.map((name) => (
            <div key={name} className="flex items-center gap-2 text-sm">
              <Checkbox
                id={`perm-${name}`}
                checked={selected.includes(name)}
                onCheckedChange={(checked) => toggle(name, checked === true)}
              />
              <label htmlFor={`perm-${name}`}>{permissionLabel[name] ?? name}</label>
            </div>
          ))}
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={requestClose}>
            Cancelar
          </Button>
          <Button disabled={saving} onClick={save}>
            Salvar permissões
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    {confirm}
    </>
  );
}
