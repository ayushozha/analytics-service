-- Monthly partitions are only pre-created from 2026-01 forward (plus 3 months
-- ahead by the partition task). Rows outside those ranges — historical imports,
-- clock-skewed clients — failed to insert with "no partition found". A DEFAULT
-- partition catches them instead of dropping them.
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'pageviews', 'events', 'goal_conversions', 'web_vitals', 'scroll_depths',
        'search_queries', 'outlinks', 'js_errors', 'click_events', 'experiment_assignments'
    ] LOOP
        IF EXISTS (
            SELECT 1 FROM pg_partitioned_table pt
            JOIN pg_class c ON c.oid = pt.partrelid
            WHERE c.relname = t
        ) AND NOT EXISTS (
            SELECT 1 FROM pg_class WHERE relname = t || '_default'
        ) THEN
            EXECUTE format('CREATE TABLE %I PARTITION OF %I DEFAULT', t || '_default', t);
        END IF;
    END LOOP;
END $$;
