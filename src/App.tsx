import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { createHashRouter, RouterProvider } from "react-router-dom";

import { needsBootstrap, startupStatus, type ApiError } from "@/lib/api";
import FirstRun from "@/pages/FirstRun";
import StartupError from "@/pages/StartupError";
import TitleBar from "@/components/TitleBar";
import { appRoutes } from "@/routes";

async function loadStartup(): Promise<{ bootstrap: boolean }> {
  await startupStatus();
  return { bootstrap: await needsBootstrap() };
}

export default function App() {
  const router = useMemo(() => createHashRouter(appRoutes), []);
  const startup = useQuery({ queryKey: ["startup"], queryFn: loadStartup, retry: false });

  let body;
  if (startup.isPending) body = null;
  else if (startup.isError) body = <StartupError message={(startup.error as ApiError).message} />;
  else if (startup.data.bootstrap) body = <FirstRun />;
  else body = <RouterProvider router={router} />;

  // A janela não tem moldura nativa: a barra própria acompanha qualquer tela.
  return (
    <>
      <TitleBar />
      {body}
    </>
  );
}
