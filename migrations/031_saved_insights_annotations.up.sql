-- Saved insights: any insight query (trends, funnels, retention, stickiness,
-- lifecycle, paths) stored with its result cache and version history.
CREATE TABLE IF NOT EXISTS insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    kind VARCHAR(20) NOT NULL,
    query JSONB NOT NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    favorited BOOLEAN NOT NULL DEFAULT false,
    deleted BOOLEAN NOT NULL DEFAULT false,
    version INT NOT NULL DEFAULT 1,
    cached_result JSONB,
    last_refresh_at TIMESTAMPTZ,
    last_viewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT insights_kind_check CHECK (kind IN ('trends','funnels','retention','stickiness','lifecycle','paths'))
);
CREATE INDEX IF NOT EXISTS idx_insights_project ON insights(project_id, updated_at DESC) WHERE NOT deleted;
CREATE INDEX IF NOT EXISTS idx_insights_tags ON insights USING GIN (tags);

CREATE TABLE IF NOT EXISTS insight_versions (
    insight_id UUID NOT NULL REFERENCES insights(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    query JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (insight_id, version)
);

-- Annotations: dated notes shown on charts (project-wide or for one insight).
CREATE TABLE IF NOT EXISTS annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    date_marker TIMESTAMPTZ NOT NULL,
    scope VARCHAR(20) NOT NULL DEFAULT 'project',
    insight_id UUID REFERENCES insights(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT annotations_scope_check CHECK (
        (scope = 'project' AND insight_id IS NULL) OR (scope = 'insight' AND insight_id IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS idx_annotations_project_date ON annotations(project_id, date_marker);
