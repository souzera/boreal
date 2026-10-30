import { describe, expect, it } from "vitest";

import { permissionLabel } from "./labels";

describe("permissionLabel", () => {
  it("nomeia as permissões de hospedagem na linguagem da pousada", () => {
    expect(permissionLabel.early_check_in).toBe("Fazer check-in antecipado");
    expect(permissionLabel.checkout_with_balance).toBe("Concluir check-out com saldo pendente");
    expect(permissionLabel.waive_late_fee).toBe("Dispensar cobrança de saída tardia");
  });

  it("nomeia as permissões do financeiro", () => {
    expect(permissionLabel.release_duplicate_payment).toBe("Liberar recebimento duplicado");
    expect(permissionLabel.manage_payables).toBe("Gerenciar contas a pagar");
  });
});
