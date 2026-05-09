CREATE TABLE IF NOT EXISTS feature_flags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT false,
    flag_type VARCHAR(32) NOT NULL DEFAULT 'boolean',
    default_value JSONB NOT NULL DEFAULT 'false',
    variants JSONB NOT NULL DEFAULT '[]',
    rollout_percentage DOUBLE PRECISION NOT NULL DEFAULT 100,
    targeting_rules JSONB NOT NULL DEFAULT '{"match":"all","conditions":[]}',
    remote_config JSONB NOT NULL DEFAULT '{}',
    experiment_id UUID REFERENCES experiments(id) ON DELETE SET NULL,
    guardrail_metrics JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT feature_flags_key_check CHECK (key <> ''),
    CONSTRAINT feature_flags_rollout_check CHECK (rollout_percentage >= 0 AND rollout_percentage <= 100),
    CONSTRAINT feature_flags_type_check CHECK (flag_type IN ('boolean', 'string', 'number', 'json'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_feature_flags_project_key
    ON feature_flags(project_id, key);

CREATE INDEX IF NOT EXISTS idx_feature_flags_project_enabled
    ON feature_flags(project_id, enabled, created_at DESC);

CREATE TABLE IF NOT EXISTS feature_flag_evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    flag_id UUID NOT NULL REFERENCES feature_flags(id) ON DELETE CASCADE,
    visitor_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255),
    matched BOOLEAN NOT NULL,
    enabled BOOLEAN NOT NULL,
    variant VARCHAR(255),
    value JSONB NOT NULL DEFAULT 'null',
    reason VARCHAR(64) NOT NULL,
    context JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_feature_flag_evaluations_project_time
    ON feature_flag_evaluations(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_feature_flag_evaluations_flag_visitor
    ON feature_flag_evaluations(flag_id, visitor_id, created_at DESC);

CREATE TABLE IF NOT EXISTS remote_config_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    description TEXT,
    value JSONB NOT NULL DEFAULT '{}',
    targeting_rules JSONB NOT NULL DEFAULT '{"match":"all","conditions":[]}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT remote_config_key_check CHECK (key <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_remote_config_project_key
    ON remote_config_entries(project_id, key);

CREATE INDEX IF NOT EXISTS idx_remote_config_project_active
    ON remote_config_entries(project_id, is_active, created_at DESC);
