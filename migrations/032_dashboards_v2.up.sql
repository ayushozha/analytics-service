-- Dashboards made of saved-insight and text tiles, with dashboard-wide filters.
CREATE TABLE IF NOT EXISTS dashboards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    tags TEXT[] NOT NULL DEFAULT '{}',
    pinned BOOLEAN NOT NULL DEFAULT false,
    is_home BOOLEAN NOT NULL DEFAULT false,
    -- {date_from, date_to, properties: [...]} applied to every insight tile
    filters JSONB NOT NULL DEFAULT '{}'::jsonb,
    deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_dashboards_project ON dashboards(project_id, updated_at DESC) WHERE NOT deleted;
-- At most one home dashboard per project.
CREATE UNIQUE INDEX IF NOT EXISTS idx_dashboards_home ON dashboards(project_id) WHERE is_home AND NOT deleted;

CREATE TABLE IF NOT EXISTS dashboard_tiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dashboard_id UUID NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    insight_id UUID REFERENCES insights(id) ON DELETE CASCADE,
    text TEXT,
    -- {x, y, w, h} on a 12-column grid
    layout JSONB NOT NULL DEFAULT '{}'::jsonb,
    color VARCHAR(32),
    -- result cache for this tile when dashboard filters modify the insight
    cache_key TEXT,
    cached_result JSONB,
    last_refresh_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT dashboard_tiles_content CHECK ((insight_id IS NULL) <> (text IS NULL))
);
CREATE INDEX IF NOT EXISTS idx_dashboard_tiles_dashboard ON dashboard_tiles(dashboard_id);

-- Share links can now point at a dashboard or a saved insight.
ALTER TABLE shared_dashboards ADD COLUMN IF NOT EXISTS dashboard_id UUID REFERENCES dashboards(id) ON DELETE CASCADE;
ALTER TABLE shared_dashboards ADD COLUMN IF NOT EXISTS insight_id UUID REFERENCES insights(id) ON DELETE CASCADE;
