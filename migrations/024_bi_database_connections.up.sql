CREATE TABLE IF NOT EXISTS bi_database_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    database_type VARCHAR(32) NOT NULL DEFAULT 'postgres',
    connection_string TEXT NOT NULL,
    allowed_schemas JSONB NOT NULL DEFAULT '["public"]'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_tested_at TIMESTAMPTZ,
    last_error TEXT,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bi_database_connections_name_check CHECK (name <> ''),
    CONSTRAINT bi_database_connections_string_check CHECK (connection_string <> ''),
    CONSTRAINT bi_database_connections_type_check CHECK (database_type IN ('postgres'))
);

CREATE INDEX IF NOT EXISTS idx_bi_database_connections_project
    ON bi_database_connections(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_bi_database_connections_active
    ON bi_database_connections(project_id, database_type)
    WHERE is_active = true;

ALTER TABLE bi_query_runs
    DROP CONSTRAINT IF EXISTS bi_query_runs_type_check;

ALTER TABLE bi_query_runs
    ADD CONSTRAINT bi_query_runs_type_check
    CHECK (query_type IN ('sql', 'saved_sql', 'visual', 'drill_through', 'external_sql'));
