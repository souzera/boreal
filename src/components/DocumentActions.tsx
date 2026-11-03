import { useQuery, useQueryClient } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { issueDocument, listDocuments, openExportedFile, saveDocumentPdf, type ApiError, type DocumentKind } from "@/lib/api";
import { formatInstant } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  kind: DocumentKind;
  sourceId: number;
  /** Texto do botão, por exemplo "Emitir comprovante". */
  label: string;
  /** Início do nome sugerido do PDF, por exemplo "comprovante-PAG-000004". */
  fileStem: string;
}

type Outcome = { kind: "saved"; path: string } | { kind: "error"; message: string; retry: () => void };

/**
 * Emitir um documento interno e guardar o PDF de qualquer versão. Emitir de novo sem mudanças devolve a mesma
 * versão; se os dados mudaram, nasce a versão seguinte e as antigas continuam como foram emitidas.
 * Falha ao gravar o arquivo nunca emite de novo: o documento já existe.
 */
export default function DocumentActions({ kind, sourceId, label, fileStem }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const queryKey = ["documents", kind, sourceId];
  const { data: versions = [] } = useQuery({ queryKey, queryFn: () => listDocuments(token, kind, sourceId), enabled: !!token, retry: false });
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  const issue = async () => {
    setBusy(true);
    setOutcome(null);
    setMessage(null);
    try {
      const before = versions[0]?.version ?? 0;
      const doc = await issueDocument(token, kind, sourceId);
      setMessage(doc.version === before ? `Nada mudou desde a versão ${doc.version}: ela continua valendo.` : `Documento emitido: versão ${doc.version}.`);
      await queryClient.invalidateQueries({ queryKey });
    } catch (e) {
      setOutcome({ kind: "error", message: (e as ApiError).message, retry: () => void issue() });
    } finally {
      setBusy(false);
    }
  };

  const write = async (documentId: number, path: string) => {
    setBusy(true);
    try {
      const result = await saveDocumentPdf(token, documentId, path);
      setOutcome({ kind: "saved", path: result.path });
    } catch (e) {
      setOutcome({ kind: "error", message: (e as ApiError).message, retry: () => void write(documentId, path) });
    } finally {
      setBusy(false);
    }
  };

  const savePdf = async (documentId: number, version: number) => {
    try {
      const path = await save({ defaultPath: `${fileStem}-v${version}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
      if (path) await write(documentId, path);
    } catch {
      setOutcome({ kind: "error", message: "Não foi possível abrir a janela de salvar. Tente de novo.", retry: () => void savePdf(documentId, version) });
    }
  };

  const open = async (path: string) => {
    try {
      await openExportedFile(token, path);
    } catch (e) {
      setOutcome({ kind: "error", message: (e as ApiError).message, retry: () => void open(path) });
    }
  };

  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="outline" size="sm" disabled={busy} onClick={() => void issue()}>
          {label}
        </Button>
        {message && (
          <span role="status" className="text-sm">
            {message}
          </span>
        )}
      </div>
      {versions.length > 0 && (
        <ul aria-label="Versões emitidas" className="grid gap-1 text-sm">
          {versions.map((d) => (
            <li key={d.id} className="flex flex-wrap items-center gap-2">
              <span>{`Versão ${d.version} · emitida em ${formatInstant(d.issued_at)} por ${d.issued_by_name}${d.current ? " · vigente" : ""}`}</span>
              <Button type="button" size="sm" variant="outline" disabled={busy} onClick={() => void savePdf(d.id, d.version)}>
                {`Salvar PDF da versão ${d.version}`}
              </Button>
            </li>
          ))}
        </ul>
      )}
      {outcome?.kind === "saved" && (
        <p role="status" className="flex flex-wrap items-center gap-2 text-sm">
          <span>{`PDF salvo em ${outcome.path}`}</span>
          <Button type="button" size="sm" variant="outline" onClick={() => void open(outcome.path)}>
            Abrir arquivo
          </Button>
        </p>
      )}
      {outcome?.kind === "error" && (
        <p role="alert" className="flex flex-wrap items-center gap-2 text-sm text-destructive">
          <span>{outcome.message}</span>
          <Button type="button" size="sm" variant="outline" onClick={outcome.retry}>
            Tentar de novo
          </Button>
        </p>
      )}
    </div>
  );
}
