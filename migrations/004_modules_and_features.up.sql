-- ============================================================================
-- Migration 004: Module System + All Feature Tables
-- ============================================================================

-- ============================================================================
-- 1. UTM TRACKING — Add columns to pageviews
-- ============================================================================
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS utm_source VARCHAR(255);
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS utm_medium VARCHAR(255);
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS utm_campaign VARCHAR(255);
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS utm_content VARCHAR(255);
ALTER TABLE pageviews ADD COLUMN IF NOT EXISTS utm_term VARCHAR(255);

CREATE INDEX IF NOT EXISTS idx_pageviews_utm ON pageviews(project_id, utm_source, created_at)
    WHERE utm_source IS NOT NULL;

-- Daily campaign rollup
CREATE TABLE IF NOT EXISTS daily_campaigns (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    utm_source VARCHAR(255) NOT NULL DEFAULT '',
    utm_medium VARCHAR(255) NOT NULL DEFAULT '',
    utm_campaign VARCHAR(255) NOT NULL DEFAULT '',
    visitors INT NOT NULL DEFAULT 0,
    sessions INT NOT NULL DEFAULT 0,
    pageviews INT NOT NULL DEFAULT 0,
    bounces INT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, utm_source, utm_medium, utm_campaign)
);

-- ============================================================================
-- 2. REVENUE TRACKING — Add columns to events
-- ============================================================================
ALTER TABLE events ADD COLUMN IF NOT EXISTS revenue_amount DECIMAL(12,2);
ALTER TABLE events ADD COLUMN IF NOT EXISTS revenue_currency VARCHAR(3) DEFAULT 'USD';

CREATE INDEX IF NOT EXISTS idx_events_revenue ON events(project_id, created_at)
    WHERE revenue_amount IS NOT NULL;

-- ============================================================================
-- 3. FUNNELS
-- ============================================================================
CREATE TABLE IF NOT EXISTS funnels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    steps JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_funnels_project ON funnels(project_id);

-- ============================================================================
-- 4. GOALS
-- ============================================================================
CREATE TABLE IF NOT EXISTS goals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    goal_type VARCHAR(20) NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_goals_project ON goals(project_id);

CREATE TABLE IF NOT EXISTS goal_conversions (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    goal_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    revenue_amount DECIMAL(12,2),
    revenue_currency VARCHAR(3) DEFAULT 'USD',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Initial partitions for goal_conversions
CREATE TABLE IF NOT EXISTS goal_conversions_2026_01 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS goal_conversions_2026_02 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS goal_conversions_2026_03 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS goal_conversions_2026_04 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS goal_conversions_2026_05 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS goal_conversions_2026_06 PARTITION OF goal_conversions
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_goal_conversions_project ON goal_conversions(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_goal_conversions_goal ON goal_conversions(goal_id, created_at);

-- ============================================================================
-- 5. WEB VITALS
-- ============================================================================
CREATE TABLE IF NOT EXISTS web_vitals (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    path VARCHAR(2048),
    metric_name VARCHAR(10) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    rating VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS web_vitals_2026_01 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS web_vitals_2026_02 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS web_vitals_2026_03 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS web_vitals_2026_04 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS web_vitals_2026_05 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS web_vitals_2026_06 PARTITION OF web_vitals
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_web_vitals_project ON web_vitals(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_web_vitals_metric ON web_vitals(project_id, metric_name, created_at);

-- ============================================================================
-- 6. SCROLL DEPTH
-- ============================================================================
CREATE TABLE IF NOT EXISTS scroll_depths (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    path VARCHAR(2048) NOT NULL,
    max_depth SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS scroll_depths_2026_01 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS scroll_depths_2026_02 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS scroll_depths_2026_03 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS scroll_depths_2026_04 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS scroll_depths_2026_05 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS scroll_depths_2026_06 PARTITION OF scroll_depths
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_scroll_depths_project ON scroll_depths(project_id, created_at);

-- ============================================================================
-- 7. SITE SEARCH
-- ============================================================================
CREATE TABLE IF NOT EXISTS search_queries (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    query VARCHAR(500) NOT NULL,
    results_count INT,
    path VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS search_queries_2026_01 PARTITION OF search_queries
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS search_queries_2026_02 PARTITION OF search_queries
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS search_queries_2026_03 PARTITION OF search_queries
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS search_queries_2026_04 PARTITION OF search_queries
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS search_queries_2026_05 PARTITION OF search_queries
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS search_queries_2026_06 PARTITION OF search_queries
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_search_queries_project ON search_queries(project_id, created_at);

-- ============================================================================
-- 8. OUTLINKS / DOWNLOADS
-- ============================================================================
CREATE TABLE IF NOT EXISTS outlinks (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    url VARCHAR(2048) NOT NULL,
    link_type VARCHAR(10) NOT NULL DEFAULT 'outlink',
    path VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS outlinks_2026_01 PARTITION OF outlinks
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS outlinks_2026_02 PARTITION OF outlinks
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS outlinks_2026_03 PARTITION OF outlinks
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS outlinks_2026_04 PARTITION OF outlinks
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS outlinks_2026_05 PARTITION OF outlinks
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS outlinks_2026_06 PARTITION OF outlinks
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_outlinks_project ON outlinks(project_id, created_at);

-- ============================================================================
-- 9. JS ERROR TRACKING
-- ============================================================================
CREATE TABLE IF NOT EXISTS js_errors (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    message TEXT NOT NULL,
    stack TEXT,
    filename VARCHAR(2048),
    lineno INT,
    colno INT,
    path VARCHAR(2048),
    browser VARCHAR(50),
    os VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS js_errors_2026_01 PARTITION OF js_errors
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS js_errors_2026_02 PARTITION OF js_errors
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS js_errors_2026_03 PARTITION OF js_errors
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS js_errors_2026_04 PARTITION OF js_errors
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS js_errors_2026_05 PARTITION OF js_errors
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS js_errors_2026_06 PARTITION OF js_errors
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_js_errors_project ON js_errors(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_js_errors_message ON js_errors(project_id, message, created_at);

-- ============================================================================
-- 10. HEATMAPS (click events)
-- ============================================================================
CREATE TABLE IF NOT EXISTS click_events (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID NOT NULL,
    path VARCHAR(2048) NOT NULL,
    x DOUBLE PRECISION NOT NULL,
    y DOUBLE PRECISION NOT NULL,
    element_selector VARCHAR(500),
    viewport_width INT,
    viewport_height INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS click_events_2026_01 PARTITION OF click_events
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS click_events_2026_02 PARTITION OF click_events
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS click_events_2026_03 PARTITION OF click_events
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS click_events_2026_04 PARTITION OF click_events
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS click_events_2026_05 PARTITION OF click_events
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS click_events_2026_06 PARTITION OF click_events
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_click_events_project ON click_events(project_id, path, created_at);

-- ============================================================================
-- 11. SESSION REPLAY
-- ============================================================================
CREATE TABLE IF NOT EXISTS session_recordings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    session_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    events_data JSONB NOT NULL DEFAULT '[]',
    events_count INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL,
    duration_ms BIGINT DEFAULT 0,
    entry_page VARCHAR(2048),
    browser VARCHAR(50),
    os VARCHAR(50),
    device VARCHAR(20),
    country CHAR(2),
    screen VARCHAR(20),
    is_complete BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_session_recordings_project ON session_recordings(project_id, created_at);
CREATE INDEX IF NOT EXISTS idx_session_recordings_session ON session_recordings(session_id);

-- ============================================================================
-- 12. SHARED DASHBOARDS
-- ============================================================================
CREATE TABLE IF NOT EXISTS shared_dashboards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL DEFAULT 'Shared Dashboard',
    token VARCHAR(64) NOT NULL UNIQUE,
    password_hash VARCHAR(128),
    modules TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_shared_dashboards_token ON shared_dashboards(token);

-- ============================================================================
-- 13. EMAIL REPORTS
-- ============================================================================
CREATE TABLE IF NOT EXISTS email_report_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    recipients TEXT[] NOT NULL,
    schedule VARCHAR(20) NOT NULL DEFAULT 'weekly',
    modules TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    last_sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_email_reports_project ON email_report_configs(project_id);

-- ============================================================================
-- 14. ALERT RULES
-- ============================================================================
CREATE TABLE IF NOT EXISTS alert_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    module VARCHAR(50) NOT NULL,
    metric VARCHAR(100) NOT NULL,
    operator VARCHAR(10) NOT NULL,
    threshold DOUBLE PRECISION NOT NULL,
    window_minutes INT NOT NULL DEFAULT 60,
    cooldown_minutes INT NOT NULL DEFAULT 360,
    notify_channels JSONB NOT NULL DEFAULT '[]',
    is_active BOOLEAN DEFAULT true,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_alert_rules_project ON alert_rules(project_id);

-- ============================================================================
-- 15. A/B TESTING
-- ============================================================================
CREATE TABLE IF NOT EXISTS experiments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    variants JSONB NOT NULL DEFAULT '[]',
    goal_id UUID,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_experiments_project ON experiments(project_id);

CREATE TABLE IF NOT EXISTS experiment_assignments (
    id BIGSERIAL,
    project_id UUID NOT NULL,
    experiment_id UUID NOT NULL,
    visitor_id VARCHAR(64) NOT NULL,
    variant VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE IF NOT EXISTS experiment_assignments_2026_01 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS experiment_assignments_2026_02 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS experiment_assignments_2026_03 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS experiment_assignments_2026_04 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS experiment_assignments_2026_05 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS experiment_assignments_2026_06 PARTITION OF experiment_assignments
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS idx_experiment_assignments_exp ON experiment_assignments(experiment_id, created_at);
CREATE INDEX IF NOT EXISTS idx_experiment_assignments_visitor ON experiment_assignments(experiment_id, visitor_id);

-- ============================================================================
-- 16. SURVEYS
-- ============================================================================
CREATE TABLE IF NOT EXISTS surveys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    questions JSONB NOT NULL DEFAULT '[]',
    trigger_config JSONB NOT NULL DEFAULT '{}',
    appearance JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    response_limit INT,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_surveys_project ON surveys(project_id);

CREATE TABLE IF NOT EXISTS survey_responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL,
    survey_id UUID NOT NULL REFERENCES surveys(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    session_id UUID,
    answers JSONB NOT NULL DEFAULT '[]',
    completed BOOLEAN DEFAULT true,
    path VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_survey_responses_survey ON survey_responses(survey_id, created_at);

-- ============================================================================
-- 17. API KEY MODULE RESTRICTIONS
-- ============================================================================
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS allowed_modules TEXT[];
