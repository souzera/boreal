import { useState, type Ref } from "react";
import { useNavigate } from "react-router-dom";

import { Input } from "@/components/ui/input";

/** Campo de busca geral do cabeçalho: leva a `/busca?q=...`. Recebe `inputRef` para o atalho Ctrl+K. */
export default function GlobalSearch({ inputRef }: { inputRef?: Ref<HTMLInputElement> }) {
  const navigate = useNavigate();
  const [text, setText] = useState("");

  return (
    <form
      role="search"
      className="min-w-48 max-w-sm flex-1"
      onSubmit={(e) => {
        e.preventDefault();
        const query = text.trim();
        if (!query) return;
        navigate(`/busca?${new URLSearchParams({ q: query })}`);
      }}
    >
      <Input
        ref={inputRef}
        type="search"
        aria-label="Busca geral"
        placeholder="Buscar hóspede, telefone, CPF ou código"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
    </form>
  );
}
