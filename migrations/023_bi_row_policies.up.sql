CREATE TABLE IF NOT EXISTS bi_row_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    dataset VARCHAR(64) NOT NULL,
    field VARCHAR(128) NOT NULL,
    operator VARCHAR(32) NOT NULL DEFAULT 'eq',
    values JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bi_row_policies_name_check CHECK (name <> ''),
    CONSTRAINT bi_row_policies_field_check CHECK (field <> ''),
    CONSTRAINT bi_row_policies_dataset_check CHECK (dataset IN ('pageviews', 'events', 'sessions', 'daily_stats', 'csv_uploads')),
    CONSTRAINT bi_row_policies_operator_check CHECK (operator IN ('eq', 'neq', 'in', 'not_in'))
);

CREATE INDEX IF NOT EXISTS idx_bi_row_policies_project
    ON bi_row_policies(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_bi_row_policies_active
    ON bi_row_policies(project_id, dataset)
    WHERE is_active = true;
