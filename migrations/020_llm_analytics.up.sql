CREATE TABLE IF NOT EXISTS llm_traces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    trace_key VARCHAR(128) NOT NULL,
    name VARCHAR(255),
    user_id VARCHAR(255),
    visitor_id VARCHAR(64),
    session_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL DEFAULT 'success',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT llm_traces_trace_key_check CHECK (trace_key <> ''),
    CONSTRAINT llm_traces_status_check CHECK (status IN ('started', 'success', 'error', 'cancelled')),
    CONSTRAINT llm_traces_duration_check CHECK (duration_ms IS NULL OR duration_ms >= 0),
    CONSTRAINT llm_traces_project_trace_key_unique UNIQUE (project_id, trace_key)
);

CREATE INDEX IF NOT EXISTS idx_llm_traces_project_time
    ON llm_traces(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_llm_traces_user
    ON llm_traces(project_id, user_id, created_at DESC)
    WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_llm_traces_visitor
    ON llm_traces(project_id, visitor_id, created_at DESC)
    WHERE visitor_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS llm_generations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    trace_id UUID REFERENCES llm_traces(id) ON DELETE SET NULL,
    trace_key VARCHAR(128),
    provider VARCHAR(128) NOT NULL,
    model VARCHAR(255) NOT NULL,
    operation VARCHAR(128) NOT NULL DEFAULT 'chat_completion',
    prompt JSONB NOT NULL DEFAULT '{}'::jsonb,
    completion JSONB NOT NULL DEFAULT '{}'::jsonb,
    input_tokens INT NOT NULL DEFAULT 0,
    output_tokens INT NOT NULL DEFAULT 0,
    total_tokens INT NOT NULL DEFAULT 0,
    latency_ms BIGINT,
    cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'success',
    error_message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT llm_generations_provider_check CHECK (provider <> ''),
    CONSTRAINT llm_generations_model_check CHECK (model <> ''),
    CONSTRAINT llm_generations_operation_check CHECK (operation <> ''),
    CONSTRAINT llm_generations_tokens_check CHECK (input_tokens >= 0 AND output_tokens >= 0 AND total_tokens >= 0),
    CONSTRAINT llm_generations_latency_check CHECK (latency_ms IS NULL OR latency_ms >= 0),
    CONSTRAINT llm_generations_cost_check CHECK (cost_usd >= 0),
    CONSTRAINT llm_generations_status_check CHECK (status IN ('success', 'error', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_llm_generations_project_time
    ON llm_generations(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_llm_generations_trace
    ON llm_generations(project_id, trace_id, created_at DESC)
    WHERE trace_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_llm_generations_trace_key
    ON llm_generations(project_id, trace_key, created_at DESC)
    WHERE trace_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_llm_generations_model
    ON llm_generations(project_id, provider, model, created_at DESC);

CREATE TABLE IF NOT EXISTS llm_evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    generation_id UUID REFERENCES llm_generations(id) ON DELETE CASCADE,
    trace_id UUID REFERENCES llm_traces(id) ON DELETE SET NULL,
    trace_key VARCHAR(128),
    evaluator VARCHAR(128) NOT NULL,
    metric VARCHAR(128) NOT NULL,
    score DOUBLE PRECISION,
    label VARCHAR(128),
    passed BOOLEAN,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT llm_evaluations_evaluator_check CHECK (evaluator <> ''),
    CONSTRAINT llm_evaluations_metric_check CHECK (metric <> '')
);

CREATE INDEX IF NOT EXISTS idx_llm_evaluations_project_time
    ON llm_evaluations(project_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_llm_evaluations_generation
    ON llm_evaluations(project_id, generation_id, created_at DESC)
    WHERE generation_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_llm_evaluations_trace
    ON llm_evaluations(project_id, trace_id, created_at DESC)
    WHERE trace_id IS NOT NULL;
