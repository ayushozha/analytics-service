CREATE TABLE IF NOT EXISTS in_app_guides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    guide_type VARCHAR(64) NOT NULL DEFAULT 'tour',
    steps JSONB NOT NULL DEFAULT '[]'::jsonb,
    targeting JSONB NOT NULL DEFAULT '{}'::jsonb,
    appearance JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    priority INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT in_app_guides_name_check CHECK (name <> ''),
    CONSTRAINT in_app_guides_type_check CHECK (guide_type IN ('tour', 'tooltip', 'onboarding', 'announcement', 'checklist')),
    CONSTRAINT in_app_guides_status_check CHECK (status IN ('draft', 'active', 'paused', 'archived'))
);

CREATE INDEX IF NOT EXISTS idx_in_app_guides_project
    ON in_app_guides(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_in_app_guides_active
    ON in_app_guides(project_id, priority DESC, created_at DESC)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS guide_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    guide_id UUID NOT NULL REFERENCES in_app_guides(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    step_id VARCHAR(128),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    path VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT guide_events_type_check CHECK (event_type IN ('shown', 'started', 'step_viewed', 'completed', 'dismissed', 'converted'))
);

CREATE INDEX IF NOT EXISTS idx_guide_events_guide
    ON guide_events(project_id, guide_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_guide_events_visitor
    ON guide_events(project_id, visitor_id, created_at DESC);
