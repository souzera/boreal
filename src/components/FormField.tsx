import type { ReactNode } from "react";

import { Label } from "@/components/ui/label";

interface Props {
  id: string;
  label: string;
  required?: boolean;
  error?: string;
  children: ReactNode;
}

/** Campo de formulário: rótulo (com asterisco se obrigatório) e mensagem de erro. */
export function FormField({ id, label, required, error, children }: Props) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>
        {label}
        {required && <span aria-hidden="true"> *</span>}
      </Label>
      {children}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

export function RequiredLegend() {
  return <p className="text-xs text-muted-foreground">* campo obrigatório</p>;
}
