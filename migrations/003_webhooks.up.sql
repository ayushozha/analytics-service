-- Webhooks for alert notifications
CREATE TABLE IF NOT EXISTS webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    events TEXT[] NOT NULL DEFAULT '{}',
    secret VARCHAR(64),
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_webhooks_project ON webhooks(project_id) WHERE is_active = true;

-- Baseline hourly pageview counts for spike detection
CREATE TABLE IF NOT EXISTS webhook_baselines (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    hour_of_day SMALLINT NOT NULL,
    day_of_week SMALLINT NOT NULL,
    avg_pageviews DOUBLE PRECISION NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, hour_of_day, day_of_week)
);
