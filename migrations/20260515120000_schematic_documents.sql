CREATE TABLE design_schematic_documents (
    design_id       UUID PRIMARY KEY REFERENCES designs (id) ON DELETE CASCADE,
    document_json   JSONB NOT NULL,
    schema_version  INTEGER NOT NULL DEFAULT 1,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_design_schematic_documents_updated
    ON design_schematic_documents (updated_at DESC);
