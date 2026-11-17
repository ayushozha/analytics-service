-- Per-person lookups (person event feed, merge, GDPR delete) scan by visitor.
CREATE INDEX IF NOT EXISTS idx_pageviews_visitor ON pageviews(project_id, visitor_id, created_at);
CREATE INDEX IF NOT EXISTS idx_events_visitor ON events(project_id, visitor_id, created_at);

-- Person merges: events still arriving under a merged-away visitor id are
-- attributed to the person they were merged into.
CREATE TABLE IF NOT EXISTS person_redirects (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    from_visitor_id VARCHAR(64) NOT NULL,
    to_visitor_id VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (project_id, from_visitor_id)
);
