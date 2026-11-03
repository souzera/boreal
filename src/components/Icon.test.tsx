import { render } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import Icon, { iconSizePx } from "./Icon";

describe("Icon", () => {
  it("mostra o nome do símbolo como ligadura, escondido dos leitores de tela", () => {
    const { container } = render(<Icon name="warning" />);

    const icon = container.querySelector("span") as HTMLElement;
    expect(icon).toHaveTextContent("warning");
    expect(icon).toHaveAttribute("aria-hidden", "true");
    expect(icon).toHaveAttribute("data-icon", "warning");
    expect(icon).toHaveClass("material-symbols-rounded");
  });

  it("o tamanho vira pixels (14, 16, 20 e 22), com 16 por padrão", () => {
    expect(iconSizePx).toEqual({ xs: 14, sm: 16, md: 20, lg: 22 });
    const { container, rerender } = render(<Icon name="bed" />);
    expect((container.firstChild as HTMLElement).style.fontSize).toBe("16px");
    rerender(<Icon name="bed" size="md" />);
    expect((container.firstChild as HTMLElement).style.fontSize).toBe("20px");
    rerender(<Icon name="bed" size="xs" />);
    expect((container.firstChild as HTMLElement).style.fontSize).toBe("14px");
    rerender(<Icon name="bed" size="lg" />);
    expect((container.firstChild as HTMLElement).style.fontSize).toBe("22px");
  });

  it("ocupa uma caixa fixa de 1em, para o texto da ligadura não alargar a tela antes da fonte carregar", () => {
    const { container } = render(<Icon name="keyboard_double_arrow_up" />);

    expect(container.firstChild).toHaveClass("w-[1em]", "h-[1em]", "overflow-hidden");
  });

  it("junta classes extras", () => {
    const { container } = render(<Icon name="bed" className="text-primary" />);
    expect(container.firstChild).toHaveClass("material-symbols-rounded", "text-primary");
  });

  it("o CSS usa o pacote empacotado e o estilo preenchido (FILL 1)", () => {
    const css = readFileSync(resolve(__dirname, "../index.css"), "utf-8");
    expect(css).toContain('@import "material-symbols/rounded.css"');
    expect(css).toMatch(/\.material-symbols-rounded\s*\{[^}]*"FILL" 1/);
  });
});
