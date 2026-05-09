CREATE TABLE IF NOT EXISTS bi_embeds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    resource_type VARCHAR(32) NOT NULL,
    resource_id UUID,
    resource_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    allowed_origins JSONB NOT NULL DEFAULT '[]'::jsonb,
    theme JSONB NOT NULL DEFAULT '{}'::jsonb,
    token_hash VARCHAR(64) NOT NULL UNIQUE,
    token_prefix VARCHAR(16) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    last_accessed_at TIMESTAMPTZ,
    access_count BIGINT NOT NULL DEFAULT 0,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bi_embeds_name_check CHECK (name <> ''),
    CONSTRAINT bi_embeds_resource_type_check CHECK (resource_type IN ('dashboard', 'report', 'sql_query', 'visual_query', 'metric'))
);

CREATE INDEX IF NOT EXISTS idx_bi_embeds_project
    ON bi_embeds(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_bi_embeds_token_hash
    ON bi_embeds(token_hash);

CREATE INDEX IF NOT EXISTS idx_bi_embeds_active
    ON bi_embeds(project_id, resource_type)
    WHERE is_active = true;
