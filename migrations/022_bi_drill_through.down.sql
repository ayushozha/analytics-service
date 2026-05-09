ALTER TABLE bi_query_runs
    DROP CONSTRAINT IF EXISTS bi_query_runs_type_check;

ALTER TABLE bi_query_runs
    ADD CONSTRAINT bi_query_runs_type_check
    CHECK (query_type IN ('sql', 'saved_sql', 'visual'));
