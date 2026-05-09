CREATE TABLE IF NOT EXISTS visual_event_labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    event_name VARCHAR(255) NOT NULL,
    path_pattern VARCHAR(2048) NOT NULL DEFAULT '*',
    element_selector VARCHAR(500) NOT NULL,
    properties JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT visual_event_labels_name_check CHECK (name <> ''),
    CONSTRAINT visual_event_labels_event_name_check CHECK (event_name <> ''),
    CONSTRAINT visual_event_labels_path_pattern_check CHECK (path_pattern <> ''),
    CONSTRAINT visual_event_labels_selector_check CHECK (element_selector <> ''),
    CONSTRAINT visual_event_labels_status_check CHECK (status IN ('active', 'paused', 'archived'))
);

CREATE INDEX IF NOT EXISTS idx_visual_event_labels_project
    ON visual_event_labels(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_visual_event_labels_active
    ON visual_event_labels(project_id, event_name)
    WHERE status = 'active';
