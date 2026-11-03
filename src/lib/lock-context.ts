import { createContext } from "react";

/** `true` enquanto a área está coberta pela tela de bloqueio ou de sessão encerrada. */
export const LockContext = createContext(false);
