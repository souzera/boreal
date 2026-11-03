import { QueryClient } from "@tanstack/react-query";

/** Cache de consultas único do aplicativo; é esvaziado quando a sessão termina ou muda de pessoa. */
export const queryClient = new QueryClient();
