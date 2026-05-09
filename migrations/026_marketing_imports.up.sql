CREATE TABLE IF NOT EXISTS marketing_imports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    provider VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    row_count INT NOT NULL DEFAULT 0,
    imported_by VARCHAR(255),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT marketing_imports_provider_check
        CHECK (provider IN ('google_analytics', 'google_ads', 'search_console')),
    CONSTRAINT marketing_imports_name_check CHECK (name <> '')
);

CREATE INDEX IF NOT EXISTS idx_marketing_imports_project
    ON marketing_imports(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_marketing_imports_provider
    ON marketing_imports(project_id, provider, created_at DESC);

CREATE TABLE IF NOT EXISTS marketing_import_rows (
    id BIGSERIAL PRIMARY KEY,
    import_id UUID NOT NULL REFERENCES marketing_imports(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    row_number INT NOT NULL,
    row_date DATE,
    dimensions JSONB NOT NULL DEFAULT '{}'::jsonb,
    metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
    raw_row JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_marketing_import_rows_import_row
    ON marketing_import_rows(import_id, row_number);

CREATE INDEX IF NOT EXISTS idx_marketing_import_rows_project_date
    ON marketing_import_rows(project_id, row_date DESC);
