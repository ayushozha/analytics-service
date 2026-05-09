CREATE TABLE IF NOT EXISTS tracking_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enforcement_mode VARCHAR(32) NOT NULL DEFAULT 'observe',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tracking_plans_enforcement_mode_check
        CHECK (enforcement_mode IN ('observe', 'reject'))
);

CREATE INDEX IF NOT EXISTS idx_tracking_plans_project
    ON tracking_plans(project_id, is_active, created_at DESC);

CREATE TABLE IF NOT EXISTS event_schema_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tracking_plan_id UUID REFERENCES tracking_plans(id) ON DELETE CASCADE,
    event_name VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    required_properties TEXT[] NOT NULL DEFAULT '{}',
    property_schema JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT event_schema_status_check
        CHECK (status IN ('draft', 'approved', 'deprecated'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_event_schema_project_plan_name
    ON event_schema_definitions(project_id, COALESCE(tracking_plan_id, '00000000-0000-0000-0000-000000000000'::uuid), event_name);

CREATE INDEX IF NOT EXISTS idx_event_schema_project_status
    ON event_schema_definitions(project_id, status, event_name);

CREATE TABLE IF NOT EXISTS data_dictionary_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    entry_type VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    data_type VARCHAR(64),
    description TEXT,
    owner VARCHAR(255),
    is_pii BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT data_dictionary_entry_type_check
        CHECK (entry_type IN ('event', 'property', 'metric', 'dimension'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_data_dictionary_project_type_name
    ON data_dictionary_entries(project_id, entry_type, name);

CREATE TABLE IF NOT EXISTS event_quality_violations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tracking_plan_id UUID REFERENCES tracking_plans(id) ON DELETE SET NULL,
    event_schema_id UUID REFERENCES event_schema_definitions(id) ON DELETE SET NULL,
    event_name VARCHAR(255) NOT NULL,
    visitor_id VARCHAR(255),
    violation_type VARCHAR(64) NOT NULL,
    message TEXT NOT NULL,
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_quality_violations_project_time
    ON event_quality_violations(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_event_quality_violations_project_event
    ON event_quality_violations(project_id, event_name, created_at DESC);
