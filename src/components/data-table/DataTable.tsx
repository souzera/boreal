import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export interface ColumnDef<T> {
  key: string;
  header: string;
  sortable?: boolean;
  render: (row: T) => ReactNode;
  className?: string;
}

export interface SortState {
  key: string;
  direction: "asc" | "desc";
}

interface Props<T> {
  columns: ColumnDef<T>[];
  rows: T[];
  getRowKey: (row: T) => string | number;
  /** Total de registros que atendem ao filtro (a consulta e a ordenação são do backend). */
  total: number;
  /** Página atual, começando em 1. */
  page: number;
  pageSize: number;
  sort?: SortState;
  emptyMessage: string;
  onPageChange: (page: number) => void;
  onSortChange: (sort: SortState) => void;
  onRowClick?: (row: T) => void;
}

function ariaSort(column: { key: string; sortable?: boolean }, sort?: SortState) {
  if (!column.sortable) return undefined;
  if (sort?.key !== column.key) return "none";
  return sort.direction === "asc" ? "ascending" : "descending";
}

export function DataTable<T>({
  columns,
  rows,
  getRowKey,
  total,
  page,
  pageSize,
  sort,
  emptyMessage,
  onPageChange,
  onSortChange,
  onRowClick,
}: Props<T>) {
  const from = (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);

  return (
    <div className="grid gap-3">
      <Table>
        <TableHeader>
          <TableRow>
            {columns.map((column) => {
              const sorted = sort?.key === column.key;
              const Icon = !sorted ? ArrowUpDown : sort.direction === "asc" ? ArrowUp : ArrowDown;
              return (
                <TableHead key={column.key} aria-sort={ariaSort(column, sort)} className={column.className}>
                  {column.sortable ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="-ml-2"
                      onClick={() =>
                        onSortChange({
                          key: column.key,
                          direction: sorted && sort.direction === "asc" ? "desc" : "asc",
                        })
                      }
                    >
                      {column.header}
                      <Icon aria-hidden="true" />
                    </Button>
                  ) : (
                    column.header
                  )}
                </TableHead>
              );
            })}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.length === 0 ? (
            <TableRow>
              <TableCell colSpan={columns.length} className="py-8 text-center text-muted-foreground">
                {emptyMessage}
              </TableCell>
            </TableRow>
          ) : (
            rows.map((row) => (
              <TableRow
                key={getRowKey(row)}
                className={onRowClick ? "cursor-pointer" : undefined}
                tabIndex={onRowClick ? 0 : undefined}
                onClick={onRowClick ? () => onRowClick(row) : undefined}
                onKeyDown={
                  onRowClick
                    ? (e) => {
                        if (e.key === "Enter") onRowClick(row);
                      }
                    : undefined
                }
              >
                {columns.map((column) => (
                  <TableCell key={column.key} className={column.className}>
                    {column.render(row)}
                  </TableCell>
                ))}
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
      {total > 0 && (
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <span>{`Mostrando ${from}–${to} de ${total}`}</span>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => onPageChange(page - 1)}>
              Anterior
            </Button>
            <Button variant="outline" size="sm" disabled={to >= total} onClick={() => onPageChange(page + 1)}>
              Próxima
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
