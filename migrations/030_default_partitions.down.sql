DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'pageviews', 'events', 'goal_conversions', 'web_vitals', 'scroll_depths',
        'search_queries', 'outlinks', 'js_errors', 'click_events', 'experiment_assignments'
    ] LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I', t || '_default');
    END LOOP;
END $$;
