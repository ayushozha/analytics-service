CREATE TABLE IF NOT EXISTS account_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    account_id VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    traits JSONB NOT NULL DEFAULT '{}'::jsonb,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, account_id)
);

CREATE INDEX IF NOT EXISTS idx_account_profiles_project
    ON account_profiles(project_id, last_seen_at DESC);

CREATE TABLE IF NOT EXISTS account_memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    account_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255),
    visitor_id VARCHAR(64) NOT NULL,
    role VARCHAR(128),
    traits JSONB NOT NULL DEFAULT '{}'::jsonb,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, account_id, visitor_id)
);

CREATE INDEX IF NOT EXISTS idx_account_memberships_account
    ON account_memberships(project_id, account_id, last_seen_at DESC);

CREATE INDEX IF NOT EXISTS idx_account_memberships_user
    ON account_memberships(project_id, user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_account_memberships_visitor
    ON account_memberships(project_id, visitor_id);
