-- Group analytics (PostHog-style): up to 5 group types per project, each
-- mapped to a fixed index. Events carry the group key of each type in
-- `$group_<index>`, so queries need no join to resolve type names.
CREATE TABLE IF NOT EXISTS group_type_mappings (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_type_index SMALLINT NOT NULL CHECK (group_type_index BETWEEN 0 AND 4),
    group_type VARCHAR(100) NOT NULL,
    name_singular VARCHAR(100),
    name_plural VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, group_type_index),
    UNIQUE (project_id, group_type)
);

CREATE TABLE IF NOT EXISTS groups (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_type_index SMALLINT NOT NULL CHECK (group_type_index BETWEEN 0 AND 4),
    group_key VARCHAR(400) NOT NULL,
    properties JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, group_type_index, group_key)
);

CREATE INDEX IF NOT EXISTS idx_groups_updated
    ON groups(project_id, group_type_index, updated_at DESC);

-- Pageviews gain free-form properties (group keys, custom pageview props).
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS properties JSONB;

-- The legacy single `account` type becomes group type 0 where it is in use.
INSERT INTO group_type_mappings (project_id, group_type_index, group_type, name_singular, name_plural)
SELECT DISTINCT project_id, 0, 'account', 'account', 'accounts' FROM account_profiles
ON CONFLICT DO NOTHING;

INSERT INTO groups (project_id, group_type_index, group_key, properties, created_at, updated_at)
SELECT ap.project_id, m.group_type_index, ap.account_id,
       ap.traits || jsonb_strip_nulls(jsonb_build_object('name', ap.name)),
       ap.first_seen_at, ap.updated_at
FROM account_profiles ap
JOIN group_type_mappings m ON m.project_id = ap.project_id AND m.group_type = 'account'
ON CONFLICT DO NOTHING;
