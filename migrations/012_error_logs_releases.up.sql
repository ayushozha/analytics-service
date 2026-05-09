CREATE TABLE IF NOT EXISTS app_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version VARCHAR(128) NOT NULL,
    environment VARCHAR(64) NOT NULL DEFAULT 'production',
    commit_sha VARCHAR(128),
    deployed_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, version, environment)
);

CREATE INDEX IF NOT EXISTS idx_app_releases_project_time
    ON app_releases(project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS source_maps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    release_id UUID REFERENCES app_releases(id) ON DELETE SET NULL,
    release_version VARCHAR(128) NOT NULL,
    environment VARCHAR(64) NOT NULL DEFAULT 'production',
    minified_url VARCHAR(2048) NOT NULL,
    source_map_url VARCHAR(2048),
    artifacts JSONB NOT NULL DEFAULT '{}'::jsonb,
    uploaded_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, release_version, environment, minified_url)
);

CREATE INDEX IF NOT EXISTS idx_source_maps_project_release
    ON source_maps(project_id, release_version, environment);

ALTER TABLE js_errors
    ADD COLUMN IF NOT EXISTS release VARCHAR(128),
    ADD COLUMN IF NOT EXISTS environment VARCHAR(64),
    ADD COLUMN IF NOT EXISTS fingerprint VARCHAR(64);

CREATE INDEX IF NOT EXISTS idx_js_errors_fingerprint
    ON js_errors(project_id, fingerprint, created_at);
CREATE INDEX IF NOT EXISTS idx_js_errors_release
    ON js_errors(project_id, release, environment, created_at);

CREATE TABLE IF NOT EXISTS log_entries (
    id BIGSERIAL PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64),
    session_id UUID,
    level VARCHAR(16) NOT NULL,
    message TEXT NOT NULL,
    body JSONB NOT NULL DEFAULT '{}'::jsonb,
    path VARCHAR(2048),
    release VARCHAR(128),
    environment VARCHAR(64),
    browser VARCHAR(50),
    os VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_log_entries_project_time
    ON log_entries(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_log_entries_level
    ON log_entries(project_id, level, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_log_entries_release
    ON log_entries(project_id, release, environment, created_at DESC);
