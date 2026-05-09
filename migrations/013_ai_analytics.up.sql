CREATE TABLE IF NOT EXISTS ai_query_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    intent VARCHAR(64) NOT NULL,
    answer TEXT NOT NULL,
    result JSONB NOT NULL DEFAULT '{}'::jsonb,
    insights JSONB NOT NULL DEFAULT '[]'::jsonb,
    start_at TIMESTAMPTZ NOT NULL,
    end_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_query_runs_project_time
    ON ai_query_runs(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ai_query_runs_intent
    ON ai_query_runs(project_id, intent, created_at DESC);
