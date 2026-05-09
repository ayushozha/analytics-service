CREATE TABLE IF NOT EXISTS destinations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    destination_type VARCHAR(64) NOT NULL DEFAULT 'webhook',
    endpoint_url TEXT NOT NULL,
    secret TEXT,
    headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    event_types TEXT[] NOT NULL DEFAULT '{}',
    transform JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    failure_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT destinations_name_check CHECK (name <> ''),
    CONSTRAINT destinations_type_check CHECK (destination_type IN ('webhook')),
    CONSTRAINT destinations_url_check CHECK (endpoint_url <> '')
);

CREATE INDEX IF NOT EXISTS idx_destinations_project
    ON destinations(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_destinations_active
    ON destinations(project_id, is_active)
    WHERE is_active = true;

CREATE TABLE IF NOT EXISTS destination_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    destination_id UUID NOT NULL REFERENCES destinations(id) ON DELETE CASCADE,
    event_type VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    payload JSONB NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    response_status INT,
    response_body TEXT,
    error_message TEXT,
    next_retry_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT destination_deliveries_status_check
        CHECK (status IN ('pending', 'retry', 'delivered', 'dead_letter'))
);

CREATE INDEX IF NOT EXISTS idx_destination_deliveries_pending
    ON destination_deliveries(status, next_retry_at)
    WHERE status IN ('pending', 'retry');

CREATE INDEX IF NOT EXISTS idx_destination_deliveries_project
    ON destination_deliveries(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_destination_deliveries_destination
    ON destination_deliveries(destination_id, created_at DESC);
