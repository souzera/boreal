import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import GlobalSearch from "./GlobalSearch";

describe("GlobalSearch", () => {
  it("encolhe em vez de empurrar as ações do cabeçalho (flex-1 e largura mínima)", () => {
    const { container } = render(
      <MemoryRouter>
        <GlobalSearch />
      </MemoryRouter>,
    );

    const form = container.querySelector("form") as HTMLElement;
    expect(form.className).toContain("flex-1");
    expect(form.className).toContain("min-w-48");
  });
});
