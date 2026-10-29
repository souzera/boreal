import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { cleaningQueue } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

function todayIso(): string {
  const d = new Date();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${month}-${day}`;
}

export default function Dashboard() {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data: queue } = useQuery({ queryKey: ["cleaning-queue"], queryFn: () => cleaningQueue(token), enabled: !!token });

  return (
    <section>
      <h1 className="text-2xl font-semibold">Dashboard</h1>
      <p className="text-sm text-muted-foreground">Hoje, {formatDate(todayIso())}</p>
      {queue &&
        (queue.length > 0 ? (
          <Link to="/limpeza" className="mt-4 inline-block rounded-lg border px-3 py-2 text-sm font-medium underline-offset-2 hover:underline">
            {`Quartos aguardando limpeza: ${queue.length}`}
          </Link>
        ) : (
          <p className="mt-4 text-sm text-muted-foreground">Nenhum quarto aguardando limpeza.</p>
        ))}
    </section>
  );
}
