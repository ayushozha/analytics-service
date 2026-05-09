CREATE TABLE IF NOT EXISTS event_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    source_type VARCHAR(64) NOT NULL DEFAULT 'webhook',
    description TEXT,
    token_hash VARCHAR(64) NOT NULL,
    token_prefix VARCHAR(16) NOT NULL,
    schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_received_at TIMESTAMPTZ,
    failure_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT event_sources_name_check CHECK (name <> ''),
    CONSTRAINT event_sources_type_check CHECK (source_type ~ '^[a-z0-9_.-]+$')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_event_sources_token_hash
    ON event_sources(token_hash);

CREATE INDEX IF NOT EXISTS idx_event_sources_project
    ON event_sources(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS source_ingestions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_id UUID NOT NULL REFERENCES event_sources(id) ON DELETE CASCADE,
    event_type VARCHAR(128) NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'accepted',
    error_message TEXT,
    destination_deliveries INT NOT NULL DEFAULT 0,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_ingestions_status_check CHECK (status IN ('accepted', 'rejected'))
);

CREATE INDEX IF NOT EXISTS idx_source_ingestions_project
    ON source_ingestions(project_id, received_at DESC);

CREATE INDEX IF NOT EXISTS idx_source_ingestions_source
    ON source_ingestions(source_id, received_at DESC);
