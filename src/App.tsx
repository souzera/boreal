import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { createHashRouter, RouterProvider } from "react-router-dom";

import { needsBootstrap, startupStatus, type ApiError } from "@/lib/api";
import FirstRun from "@/pages/FirstRun";
import StartupError from "@/pages/StartupError";
import { appRoutes } from "@/routes";

async function loadStartup(): Promise<{ bootstrap: boolean }> {
  await startupStatus();
  return { bootstrap: await needsBootstrap() };
}

export default function App() {
  const router = useMemo(() => createHashRouter(appRoutes), []);
  const startup = useQuery({ queryKey: ["startup"], queryFn: loadStartup, retry: false });

  if (startup.isPending) return null;
  if (startup.isError) {
    return <StartupError message={(startup.error as ApiError).message} />;
  }
  if (startup.data.bootstrap) return <FirstRun />;

  return <RouterProvider router={router} />;
}
