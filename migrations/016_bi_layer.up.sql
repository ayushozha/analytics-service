CREATE TABLE IF NOT EXISTS semantic_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    key VARCHAR(128) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    dataset VARCHAR(64) NOT NULL DEFAULT 'events',
    expression TEXT NOT NULL,
    filters JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT semantic_metrics_key_check CHECK (key <> ''),
    CONSTRAINT semantic_metrics_name_check CHECK (name <> ''),
    CONSTRAINT semantic_metrics_dataset_check CHECK (dataset IN ('pageviews', 'events', 'sessions', 'daily_stats', 'csv_uploads'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_semantic_metrics_project_key
    ON semantic_metrics(project_id, key);

CREATE INDEX IF NOT EXISTS idx_semantic_metrics_project
    ON semantic_metrics(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS saved_sql_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    sql_text TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT saved_sql_queries_name_check CHECK (name <> ''),
    CONSTRAINT saved_sql_queries_sql_check CHECK (sql_text <> '')
);

CREATE INDEX IF NOT EXISTS idx_saved_sql_queries_project
    ON saved_sql_queries(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS bi_query_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    query_id UUID REFERENCES saved_sql_queries(id) ON DELETE SET NULL,
    query_type VARCHAR(64) NOT NULL,
    sql_text TEXT NOT NULL,
    result JSONB NOT NULL DEFAULT '[]'::jsonb,
    row_count INT NOT NULL DEFAULT 0,
    duration_ms INT NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'success',
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bi_query_runs_type_check CHECK (query_type IN ('sql', 'saved_sql', 'visual')),
    CONSTRAINT bi_query_runs_status_check CHECK (status IN ('success', 'error'))
);

CREATE INDEX IF NOT EXISTS idx_bi_query_runs_project
    ON bi_query_runs(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS csv_uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    columns JSONB NOT NULL DEFAULT '[]'::jsonb,
    row_count INT NOT NULL DEFAULT 0,
    uploaded_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT csv_uploads_name_check CHECK (name <> '')
);

CREATE INDEX IF NOT EXISTS idx_csv_uploads_project
    ON csv_uploads(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS csv_upload_rows (
    id BIGSERIAL PRIMARY KEY,
    upload_id UUID NOT NULL REFERENCES csv_uploads(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    row_number INT NOT NULL,
    row_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_csv_upload_rows_upload_row
    ON csv_upload_rows(upload_id, row_number);

CREATE INDEX IF NOT EXISTS idx_csv_upload_rows_project_upload
    ON csv_upload_rows(project_id, upload_id);
