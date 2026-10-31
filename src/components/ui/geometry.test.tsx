import { render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { PageHeader } from "@/components/PageHeader";
import { Badge } from "./badge";
import { Button } from "./button";
import { Card } from "./card";
import { Dialog, DialogContent, DialogTitle } from "./dialog";
import { Input } from "./input";

const css = readFileSync(resolve(__dirname, "../../index.css"), "utf-8");

describe("geometria e elevação do Boreal", () => {
  it("os raios do tema seguem o brief: 8 px em botões e campos, 12 px em cards e diálogos", () => {
    expect(css).toMatch(/--radius:\s*0\.5rem/);
    expect(css).toMatch(/--radius-lg:\s*var\(--radius\)/);
    expect(css).toMatch(/--radius-xl:\s*var\(--radius-card\)/);
    expect(css).toMatch(/--radius-card:\s*0\.75rem/);
  });

  it("botão e campo usam o raio de 8 px e mostram o foco", () => {
    render(
      <>
        <Button>Salvar</Button>
        <Input aria-label="campo" />
      </>,
    );

    for (const el of [screen.getByRole("button", { name: "Salvar" }), screen.getByLabelText("campo")]) {
      expect(el.className).toContain("rounded-lg");
      expect(el.className).toContain("focus-visible:ring");
    }
  });

  it("o botão principal escurece ao passar o mouse com o token do tema (contraste AA), não com transparência", () => {
    render(<Button>Confirmar</Button>);

    const cls = screen.getByRole("button", { name: "Confirmar" }).className;
    expect(cls).toContain("hover:bg-primary-hover");
    expect(cls).not.toContain("bg-primary/80");
  });

  it("o foco usa o anel sólido do tema (3:1), sem a meia-transparência que some sobre o teal", () => {
    render(
      <>
        <Button>Salvar</Button>
        <Input aria-label="campo" />
      </>,
    );

    for (const el of [screen.getByRole("button", { name: "Salvar" }), screen.getByLabelText("campo")]) {
      expect(el.className, el.tagName).toContain("focus-visible:ring-ring");
      expect(el.className, el.tagName).not.toContain("ring-ring/50");
    }
  });

  it("botões pequenos e compactos também têm raio de 8 px", () => {
    render(
      <>
        <Button size="sm">Pequeno</Button>
        <Button size="xs">Mínimo</Button>
        <Button size="icon-sm" aria-label="ícone" />
      </>,
    );

    for (const name of ["Pequeno", "Mínimo", "ícone"]) {
      const cls = screen.getByRole("button", { name }).className;
      expect(cls, name).toContain("rounded-lg");
      expect(cls, name).not.toContain("rounded-[min(");
    }
  });

  it("card tem raio de 12 px, borda e sombra suave (sem anel pesado)", () => {
    const { container } = render(<Card>conteúdo</Card>);

    const card = container.firstChild as HTMLElement;
    expect(card.className).toContain("rounded-xl");
    expect(card.className).toContain("shadow-card");
    expect(card.className).toContain("border");
    expect(card.className).not.toContain("ring-foreground");
  });

  it("diálogo tem raio de 12 px e sombra de camada sobreposta", async () => {
    render(
      <Dialog open>
        <DialogContent>
          <DialogTitle>Título</DialogTitle>
        </DialogContent>
      </Dialog>,
    );

    const dialog = await screen.findByRole("dialog");
    expect(dialog.className).toContain("rounded-xl");
    expect(dialog.className).toContain("shadow-overlay");
  });

  it("badge usa o raio de 6 px", () => {
    render(<Badge>Novo</Badge>);

    expect(screen.getByText("Novo").className).toContain("rounded-md");
  });

  it("o título da página tem 26 px e peso 650", () => {
    render(<PageHeader title="Reservas" />);

    const title = screen.getByRole("heading", { name: "Reservas" });
    expect(title.className).toContain("text-[26px]");
    expect(title.className).toContain("font-[650]");
  });
});
