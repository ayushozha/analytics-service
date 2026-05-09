-- ============================================================================
-- Migration 005: Identity Profiles
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(255),
    traits JSONB NOT NULL DEFAULT '{}',
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    identified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, visitor_id)
);

CREATE INDEX IF NOT EXISTS idx_user_profiles_project ON user_profiles(project_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_user_profiles_user_id ON user_profiles(project_id, user_id)
    WHERE user_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS user_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, user_id, visitor_id)
);

CREATE INDEX IF NOT EXISTS idx_user_aliases_user ON user_aliases(project_id, user_id);
CREATE INDEX IF NOT EXISTS idx_user_aliases_visitor ON user_aliases(project_id, visitor_id);
