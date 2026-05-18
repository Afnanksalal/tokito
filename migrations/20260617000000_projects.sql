CREATE TABLE projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL UNIQUE,
    workspace_path  TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_projects_slug ON projects (slug);

ALTER TABLE designs
    ADD COLUMN project_id UUID REFERENCES projects (id) ON DELETE SET NULL;

CREATE INDEX idx_designs_project ON designs (project_id);

-- Default project for existing designs.
INSERT INTO projects (id, name, slug, workspace_path)
VALUES (
    '00000000-0000-4000-8000-000000000001',
    'Default',
    'default',
    'default'
);

UPDATE designs SET project_id = '00000000-0000-4000-8000-000000000001'
WHERE project_id IS NULL;
