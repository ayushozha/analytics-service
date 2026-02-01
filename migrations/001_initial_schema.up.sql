-- Projects (tenants)
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    domain VARCHAR(255),
    umami_website_id VARCHAR(100),
    umami_share_url TEXT,
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API Keys (multiple per project)
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    key_hash VARCHAR(64) NOT NULL,
    key_prefix VARCHAR(8) NOT NULL,
    name VARCHAR(255) NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{ingest}',
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT true
);
CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON api_keys(key_hash) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_api_keys_project ON api_keys(project_id);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    hostname VARCHAR(255),
    browser VARCHAR(50),
    os VARCHAR(50),
    device VARCHAR(20),
    screen VARCHAR(20),
    language VARCHAR(10),
    country CHAR(2),
    region VARCHAR(100),
    city VARCHAR(100),
    first_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_bounce BOOLEAN NOT NULL DEFAULT true,
    entry_page VARCHAR(2048),
    exit_page VARCHAR(2048),
    pageview_count INT NOT NULL DEFAULT 0,
    event_count INT NOT NULL DEFAULT 0,
    duration_ms BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sessions_project_time ON sessions(project_id, first_at);
CREATE INDEX IF NOT EXISTS idx_sessions_visitor ON sessions(project_id, visitor_id);

-- Pageviews (partitioned by month)
CREATE TABLE IF NOT EXISTS pageviews (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    session_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    path VARCHAR(2048) NOT NULL,
    title VARCHAR(500),
    referrer VARCHAR(2048),
    referrer_domain VARCHAR(255),
    query_params JSONB,
    duration_ms INT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create partitions for 2026
CREATE TABLE IF NOT EXISTS pageviews_2026_01 PARTITION OF pageviews
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS pageviews_2026_02 PARTITION OF pageviews
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS pageviews_2026_03 PARTITION OF pageviews
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS pageviews_2026_04 PARTITION OF pageviews
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS pageviews_2026_05 PARTITION OF pageviews
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS pageviews_2026_06 PARTITION OF pageviews
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_pageviews_project_time ON pageviews(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_pageviews_path ON pageviews(project_id, path, created_at);

-- Custom Events (partitioned by month)
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    session_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    event_name VARCHAR(255) NOT NULL,
    event_data JSONB,
    path VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS events_2026_01 PARTITION OF events
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS events_2026_02 PARTITION OF events
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS events_2026_03 PARTITION OF events
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS events_2026_04 PARTITION OF events
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS events_2026_05 PARTITION OF events
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS events_2026_06 PARTITION OF events
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_events_project_time ON events(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_events_name ON events(project_id, event_name, created_at);
