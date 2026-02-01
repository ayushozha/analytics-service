-- Pre-computed daily rollups for fast dashboard queries

CREATE TABLE IF NOT EXISTS daily_stats (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    pageviews BIGINT NOT NULL DEFAULT 0,
    visitors BIGINT NOT NULL DEFAULT 0,
    sessions BIGINT NOT NULL DEFAULT 0,
    bounces BIGINT NOT NULL DEFAULT 0,
    total_duration_ms BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date)
);

CREATE TABLE IF NOT EXISTS daily_pages (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    path VARCHAR(2048) NOT NULL,
    views BIGINT NOT NULL DEFAULT 0,
    unique_views BIGINT NOT NULL DEFAULT 0,
    avg_duration_ms INT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, path)
);

CREATE TABLE IF NOT EXISTS daily_referrers (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    referrer_domain VARCHAR(255) NOT NULL,
    visits BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, referrer_domain)
);

CREATE TABLE IF NOT EXISTS daily_events (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    event_name VARCHAR(255) NOT NULL,
    count BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, event_name)
);

CREATE TABLE IF NOT EXISTS daily_geo (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    country CHAR(2) NOT NULL,
    visitors BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, country)
);

CREATE TABLE IF NOT EXISTS daily_devices (
    project_id UUID NOT NULL,
    date DATE NOT NULL,
    browser VARCHAR(50) NOT NULL DEFAULT 'Unknown',
    os VARCHAR(50) NOT NULL DEFAULT 'Unknown',
    device VARCHAR(20) NOT NULL DEFAULT 'desktop',
    visitors BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (project_id, date, browser, os, device)
);
