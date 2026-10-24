# Permissões

Controle de acesso é aplicado **no backend**; esconder botões na interface é só conveniência.

## Perfis

- **Administrador**: todas as permissões.
- **Recepção**: nenhuma permissão sensível por padrão; opera reservas, hospedagens, limpeza, lançamentos e consultas dos marcos seguintes.

## Permissões (`auth::permissions::Permission`)

`ManageUsers`, `ManageSettings`, `ManageCatalog`, `ViewFinancialReports`, `RestoreBackup` (somente administrador) e as **concedíveis** a usuários da recepção, uma a uma: `ApplyDiscount`, `ExceedCapacity`, `ReverseEntry`, `CancellationException`, `ChangeReservation`, `ReleaseRoom`, `EarlyCheckIn`, `CheckoutWithBalance`, `WaiveLateFee` (`Permission::grantable()`).

## Como funciona

1. No login, as permissões extras do usuário (`user_permissions`) são carregadas na sessão como `PermissionSet` (máscara de bits, `Copy`).
2. Todo serviço chama `require(&info, Permission::X)`: passa se o perfil tem a permissão **ou** se ela está no conjunto extra.
3. `UserService::set_extra_permissions` (exige `ManageUsers`) recusa permissões fora de `grantable()` e usuários administradores, audita antes/depois e **encerra as sessões do usuário**, para a mudança valer na hora do próximo login.

## Onde cada permissão é exigida (Marco 2)

- `ChangeReservation`: `apply_change` (alterar datas ou quarto).
- `ApplyDiscount`: `apply_discount`.
- `CancellationException`: `cancel` com exceção à política.
- `ExceedCapacity`: criar reserva acima da capacidade do quarto.
- `ManageCatalog`: tarifas, feriados, política de cancelamento e catálogo de itens e serviços.

## Onde cada permissão é exigida (Marco 3)

- `EarlyCheckIn`: `check_in` antes de data + horário previsto de entrada.
- `ExceedCapacity`: `check_in` e `walk_in` com mais ocupantes que a capacidade.
- `ApplyDiscount`: `add_adjustment` (ajuste da conta).
- `ReverseEntry`: `reverse_charge` (estorno de lançamento).
- `WaiveLateFee`: `checkout` dispensando a cobrança de saída tardia.
- `CheckoutWithBalance`: `checkout` com saldo diferente de zero (devedor ou crédito).
- `ChangeReservation`: `set_late_tolerance` (tolerância de saída desta hospedagem).
- A interface consulta `my_permissions` só para decidir quais botões oferecer; o backend é quem autoriza.

## Regras de proteção

- Sempre existe pelo menos um administrador ativo (`set_role` e `set_active` recusam o contrário).
- Ninguém altera o próprio perfil nem se desativa.
- Sessão com troca de senha pendente só chama trocar senha, consultar estado e sair (`SessionManager::touch` estrito × `touch_allow_pending`).
- Desativar ou redefinir a senha de um usuário encerra as sessões dele.

## Adicionando uma permissão nova

1. Incluir a variante em `Permission`, em `ALL` e em `as_str` (nome estável gravado no banco).
2. Se for concedível, incluir em `grantable()` e em `pages/users/labels.ts` (rótulo em português).
3. Chamar `require` no serviço e testar `Forbidden` para recepção sem a permissão e sucesso com ela.
