import { useId } from "react";

import { cn } from "@/lib/utils";

/** Marca do Boreal: fitas de aurora sobre o teal da identidade. O desenho é o mesmo do ícone do aplicativo. */
export function LogoMark({ size = 28, className }: { size?: number; className?: string }) {
  const gradient = useId();
  return (
    <svg viewBox="0 0 64 64" width={size} height={size} aria-hidden="true" focusable="false" className={cn("shrink-0", className)}>
      <defs>
        <linearGradient id={gradient} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#0F766E" />
          <stop offset="1" stopColor="#0B3F3B" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="14" fill={`url(#${gradient})`} />
      <path d="M13 47 C20 32 26 44 32 31 S45 26 51 15" fill="none" stroke="#CCFBF1" strokeWidth="5.5" strokeLinecap="round" />
      <path d="M13 55 C20 42 26 52 32 41 S45 37 51 27" fill="none" stroke="#5EEAD4" strokeWidth="5.5" strokeLinecap="round" opacity="0.8" />
      <path d="M19 59 C24 52 28 57 33 50 S43 47 47 40" fill="none" stroke="#2DD4BF" strokeWidth="5" strokeLinecap="round" opacity="0.5" />
      <circle cx="45" cy="11" r="2.4" fill="#FFFFFF" />
    </svg>
  );
}

/** Marca com o nome ao lado. O nome é o texto acessível; o desenho é decorativo. */
export function Logo({ size = 28, className }: { size?: number; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2 font-semibold text-primary", className)}>
      <LogoMark size={size} />
      <span>Boreal PMS</span>
    </span>
  );
}
