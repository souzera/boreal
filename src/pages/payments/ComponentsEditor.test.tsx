import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";

import { emptyRow, type SplitRow } from "@/lib/payment";
import ComponentsEditor from "./ComponentsEditor";

function Harness({ total, initial }: { total: number | null; initial?: SplitRow[] }) {
  const [rows, setRows] = useState<SplitRow[]>(initial ?? [emptyRow()]);
  return <ComponentsEditor totalCents={total} rows={rows} onChange={setRows} />;
}

describe("ComponentsEditor", () => {
  it("começa com uma forma de pagamento e oferece Pix, dinheiro, débito, crédito e transferência", () => {
    render(<Harness total={10000} />);

    const options = screen.getAllByRole("option").map((o) => o.textContent);
    expect(options).toEqual(["Escolha…", "Dinheiro", "Pix", "Cartão de débito", "Cartão de crédito", "Transferência bancária"]);
    expect(screen.queryByText(/Soma/)).toBeNull();
  });

  it("dividir mostra os valores, a soma e quanto falta, com texto e ícone", async () => {
    render(<Harness total={10000} />);
    await userEvent.click(screen.getByRole("button", { name: "Dividir em outra forma" }));

    await userEvent.selectOptions(screen.getByLabelText("Forma de pagamento 1"), "pix");
    await userEvent.type(screen.getByLabelText("Valor da forma 1"), "60,00");
    await userEvent.selectOptions(screen.getByLabelText("Forma de pagamento 2"), "cash");
    await userEvent.type(screen.getByLabelText("Valor da forma 2"), "20,00");

    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("Soma R$ 80,00 de R$ 100,00");
    expect(status).toHaveTextContent("Falta R$ 20,00");
    expect(status.querySelector('[data-icon="warning"]')).not.toBeNull();
  });

  it("mostra Sobra e Confere", async () => {
    render(
      <Harness
        total={10000}
        initial={[
          { method: "debit", amount: "70,00" },
          { method: "credit", amount: "40,00" },
        ]}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Sobra R$ 10,00");

    await userEvent.clear(screen.getByLabelText("Valor da forma 2"));
    await userEvent.type(screen.getByLabelText("Valor da forma 2"), "30,00");

    expect(screen.getByRole("status")).toHaveTextContent("Confere");
  });

  it("remove uma forma e volta ao formato de uma só", async () => {
    render(
      <Harness
        total={10000}
        initial={[
          { method: "pix", amount: "60,00" },
          { method: "cash", amount: "40,00" },
        ]}
      />,
    );

    await userEvent.click(screen.getAllByRole("button", { name: "Remover forma" })[1]);

    expect(screen.queryByLabelText("Valor da forma 1")).toBeNull();
    expect(screen.getByLabelText("Forma de pagamento")).toHaveValue("pix");
  });
});
