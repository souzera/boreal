import type { ReportTable } from "@/lib/api";
import { cellText } from "@/lib/report";
import { cn } from "@/lib/utils";

const numeric = new Set(["int", "money", "percent"]);

/** A tabela do relatório: a mesma que vira CSV e PDF. Os totais ficam no rodapé. */
export function ReportTableView({ table }: { table: ReportTable }) {
  return (
    <div className="grid gap-2">
      <div className="overflow-x-auto rounded-xl border bg-card">
        <table className="w-full border-collapse text-sm">
          <caption className="sr-only">{table.title}</caption>
          <thead className="bg-muted">
            <tr>
              {table.columns.map((c) => (
                <th key={c.key} scope="col" className={cn("px-3 py-2 font-medium", numeric.has(c.kind) ? "text-right" : "text-left")}>
                  {c.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {table.rows.length === 0 && (
              <tr>
                <td colSpan={table.columns.length} className="px-3 py-4 text-muted-foreground">
                  Nenhum registro para estes filtros.
                </td>
              </tr>
            )}
            {table.rows.map((row, i) => (
              <tr key={i} className="border-t">
                {row.map((cell, j) => (
                  <td key={j} className={cn("px-3 py-1.5", numeric.has(table.columns[j]?.kind) && "text-right tabular-nums")}>
                    {cellText(cell)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
          {table.totals && table.rows.length > 0 && (
            <tfoot>
              <tr className="border-t-2 bg-muted/60 font-semibold">
                {table.totals.map((cell, j) => (
                  <td key={j} className={cn("px-3 py-2", numeric.has(table.columns[j]?.kind) && "text-right tabular-nums")}>
                    {cellText(cell)}
                  </td>
                ))}
              </tr>
            </tfoot>
          )}
        </table>
      </div>
      {table.truncated && (
        <p role="status" className="text-sm text-warning-foreground">
          Mostrando só as primeiras linhas; refine os filtros para ver o resto.
        </p>
      )}
    </div>
  );
}
