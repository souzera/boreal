import type { Role } from "@/stores/session";

export const roleLabel: Record<Role, string> = {
  admin: "Administrador",
  recepcao: "Recepção",
};

/** Nomes, em linguagem da pousada, das permissões que podem ser concedidas. */
export const permissionLabel: Record<string, string> = {
  apply_discount: "Aplicar descontos",
  exceed_capacity: "Exceder a capacidade do quarto",
  reverse_entry: "Estornar lançamentos",
  cancellation_exception: "Autorizar exceções de cancelamento",
  change_reservation: "Alterar datas ou quarto de reservas",
  release_room: "Liberar quartos",
  early_check_in: "Fazer check-in antecipado",
  checkout_with_balance: "Concluir check-out com saldo pendente",
  waive_late_fee: "Dispensar cobrança de saída tardia",
};
