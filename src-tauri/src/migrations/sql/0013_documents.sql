-- Documentos internos emitidos (spec §17): cada emissão grava um instantâneo (JSON) com tudo o que o documento
-- mostra, inclusive o cabeçalho da pousada. Reimprimir usa sempre o instantâneo; mudar a origem e emitir de novo
-- cria a versão seguinte. Nada é alterado nem apagado.
CREATE TABLE documents (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    kind      TEXT    NOT NULL CHECK (kind IN ('reservation_confirmation', 'payment_receipt', 'stay_record', 'final_statement')),
    source_id INTEGER NOT NULL,
    version   INTEGER NOT NULL CHECK (version >= 1),
    snapshot  TEXT    NOT NULL,
    issued_at TEXT    NOT NULL,
    issued_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    UNIQUE (kind, source_id, version)
);

CREATE INDEX idx_documents_source ON documents(kind, source_id);

CREATE TRIGGER documents_no_update BEFORE UPDATE ON documents
BEGIN
    SELECT RAISE(ABORT, 'documentos emitidos são imutáveis');
END;

CREATE TRIGGER documents_no_delete BEFORE DELETE ON documents
BEGIN
    SELECT RAISE(ABORT, 'documentos emitidos são imutáveis');
END;
