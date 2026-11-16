-- Person cohorts (PostHog-style): dynamic cohorts defined by person-property
-- and behavioural criteria, recalculated into cohort_people; static cohorts
-- whose members are added explicitly.
CREATE TABLE IF NOT EXISTS person_cohorts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_static BOOLEAN NOT NULL DEFAULT false,
    filters JSONB NOT NULL DEFAULT '{}'::jsonb,
    member_count INT NOT NULL DEFAULT 0,
    is_calculating BOOLEAN NOT NULL DEFAULT false,
    -- When the current calculation claim was taken (lease for crash recovery).
    calc_started_at TIMESTAMPTZ,
    last_calculated_at TIMESTAMPTZ,
    last_error TEXT,
    deleted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_person_cohorts_project ON person_cohorts(project_id) WHERE NOT deleted;

CREATE TABLE IF NOT EXISTS cohort_people (
    cohort_id UUID NOT NULL REFERENCES person_cohorts(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cohort_id, visitor_id)
);
CREATE INDEX IF NOT EXISTS idx_cohort_people_visitor ON cohort_people(visitor_id);

CREATE TABLE IF NOT EXISTS cohort_counts (
    cohort_id UUID NOT NULL REFERENCES person_cohorts(id) ON DELETE CASCADE,
    member_count INT NOT NULL,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cohort_id, calculated_at)
);
