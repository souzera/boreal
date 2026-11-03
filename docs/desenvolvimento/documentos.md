# Documentos internos (Marco 6.4)

Plano: `docs/superpowers/plans/2026-11-03-marco-6-4-relatorios-e-documentos.md`; decisões D9 a D11 e Anexo A.8. **Migração `0013`** (`documents`).

## Modelo

`documents(kind, source_id, version, snapshot, issued_at, issued_by)` com `UNIQUE (kind, source_id, version)` e gatilhos `BEFORE UPDATE`/`BEFORE DELETE` que abortam: **imutável**. `kind`: `reservation_confirmation` (origem = reserva), `payment_receipt` (recebimento), `stay_record` e `final_statement` (hospedagem).

## Emissão (`DocumentService`)

- `issue(token, kind, source_id)`: monta o **instantâneo** (JSON com chaves em ordem estável) lendo os repositórios sob a mesma trava da conexão (sem chamar outros serviços, para não aninhar a trava). Numa transação: se o instantâneo é **idêntico** ao da última versão, devolve essa versão sem gravar nada; senão grava a versão seguinte e audita (`documents`/`issue_document`, com tipo, origem e versão, **sem** o conteúdo) na mesma transação.
- `list`: versões da mais nova para a mais antiga (`current` = a primeira).
- `pdf(document_id)`: gera o PDF **só do instantâneo gravado** (reimprimir nunca lê os dados atuais). O cabeçalho (nome, endereço e telefone da pousada) também é congelado na emissão.
- `save_pdf(document_id, path)`: grava o arquivo com `write_export`; falhar não emite nem repete nada.

## Condições (D10)

Qualquer sessão válida emite e lista. Confirmação só para reserva `pending|confirmed|checked_in|completed`; demonstrativo só com a hospedagem `checked_out`; comprovante de um recebimento existente; ficha de qualquer hospedagem. Origem inexistente, tipo desconhecido ou condição não atendida ⇒ `Validation` com texto da pousada.

## Conteúdo (D11, provisório ⚠)

O instantâneo não inclui o CPF cadastrado do hóspede, e-mail, observações internas nem justificativas. **Exceção:** a ficha grava o "documento" que o ocupante informou, que é texto livre e pode ser um CPF completo; o teste de ausência de CPF cobre a confirmação. Confirmação: reserva, hóspede e telefone, quarto e categoria, datas e horários, ocupantes, diárias, valores e sinal. Comprovante: número, data e hora locais, quem recebeu, referência e componentes por forma de pagamento. Ficha: ocupantes com o documento informado e linha de assinatura. Demonstrativo: lançamentos, recebimentos, devoluções e situação final em palavras.

## Frontend

`DocumentActions` (`kind`, `sourceId`, `label`, `fileStem`): botão **Emitir …**, mensagem ("Documento emitido: versão N." ou "Nada mudou desde a versão N: ela continua valendo."), lista **Versões emitidas** com a vigente marcada em texto, **Salvar PDF da versão N** (diálogo "Salvar como" com nome sugerido) e **Abrir arquivo**. Erro ao gravar mostra a mensagem e **Tentar de novo** só da gravação. Aparece em `ReservationPage`, `StayPage` e em cada recebimento.

## Limitações

- Sem logotipo nos PDFs (v1) e sem assinatura digital.
- O instantâneo guarda só o que o PDF mostra (a ficha não guarda a situação da hospedagem, para não criar versão sem mudança visível).
- O instantâneo só muda quando os dados exibidos mudam; mudanças que não aparecem no documento (por exemplo, observações) não criam versão.
- O backup do Marco 7 deve incluir a tabela `documents`.

## Testes

`services/document_service.rs`: versão 1 com auditoria na mesma transação, reemissão sem mudança, versão 2 preservando a 1, congelamento do cabeçalho da pousada, imutabilidade por SQL, condições e origens inválidas, comprovante com pagamento dividido, ficha, demonstrativo conferido com a conta, ausência de CPF/e-mail/observações, sessão obrigatória. Migração: `migrations` (versão 13). Front: `DocumentActions`, `ReservationPage`, `StayPage`.
