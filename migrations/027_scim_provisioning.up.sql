CREATE TABLE IF NOT EXISTS scim_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_name VARCHAR(255) NOT NULL,
    external_id VARCHAR(255),
    active BOOLEAN NOT NULL DEFAULT true,
    display_name VARCHAR(255),
    given_name VARCHAR(255),
    family_name VARCHAR(255),
    emails JSONB NOT NULL DEFAULT '[]'::jsonb,
    traits JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT scim_users_name_check CHECK (user_name <> ''),
    CONSTRAINT scim_users_emails_check CHECK (jsonb_typeof(emails) = 'array'),
    CONSTRAINT scim_users_traits_check CHECK (jsonb_typeof(traits) = 'object'),
    UNIQUE (project_id, user_name)
);

CREATE INDEX IF NOT EXISTS idx_scim_users_project
    ON scim_users(project_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_scim_users_external
    ON scim_users(project_id, external_id)
    WHERE external_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS scim_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    display_name VARCHAR(255) NOT NULL,
    external_id VARCHAR(255),
    traits JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT scim_groups_name_check CHECK (display_name <> ''),
    CONSTRAINT scim_groups_traits_check CHECK (jsonb_typeof(traits) = 'object'),
    UNIQUE (project_id, display_name)
);

CREATE INDEX IF NOT EXISTS idx_scim_groups_project
    ON scim_groups(project_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS scim_group_members (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES scim_groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES scim_users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_scim_group_members_user
    ON scim_group_members(project_id, user_id);
