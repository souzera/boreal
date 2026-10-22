import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { Input } from "@/components/ui/input";

/** Campo de busca geral do cabeçalho: leva a `/busca?q=...`. */
export default function GlobalSearch() {
  const navigate = useNavigate();
  const [text, setText] = useState("");

  return (
    <form
      role="search"
      className="w-full max-w-sm"
      onSubmit={(e) => {
        e.preventDefault();
        const query = text.trim();
        if (!query) return;
        navigate(`/busca?${new URLSearchParams({ q: query })}`);
      }}
    >
      <Input
        type="search"
        aria-label="Busca geral"
        placeholder="Buscar hóspede, telefone, CPF ou código"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
    </form>
  );
}
