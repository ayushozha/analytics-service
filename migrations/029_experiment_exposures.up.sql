-- One row per (experiment, visitor): the first exposure is the only one that
-- counts. experiment_assignments is partitioned by created_at, so it cannot
-- carry a UNIQUE (experiment_id, visitor_id) constraint itself.
CREATE TABLE IF NOT EXISTS experiment_exposures (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    experiment_id UUID NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    visitor_id VARCHAR(64) NOT NULL,
    variant VARCHAR(100) NOT NULL,
    first_exposed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (experiment_id, visitor_id)
);

CREATE INDEX IF NOT EXISTS idx_experiment_exposures_time
    ON experiment_exposures(experiment_id, first_exposed_at);

-- Backfill: keep each visitor's earliest assignment.
INSERT INTO experiment_exposures (project_id, experiment_id, visitor_id, variant, first_exposed_at)
SELECT DISTINCT ON (ea.experiment_id, ea.visitor_id)
       ea.project_id, ea.experiment_id, ea.visitor_id, ea.variant, ea.created_at
FROM experiment_assignments ea
JOIN experiments e ON e.id = ea.experiment_id
ORDER BY ea.experiment_id, ea.visitor_id, ea.created_at ASC
ON CONFLICT (experiment_id, visitor_id) DO NOTHING;
