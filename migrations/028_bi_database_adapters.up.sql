ALTER TABLE bi_database_connections
    DROP CONSTRAINT IF EXISTS bi_database_connections_type_check;

ALTER TABLE bi_database_connections
    ADD CONSTRAINT bi_database_connections_type_check
    CHECK (database_type IN ('postgres', 'clickhouse', 'http_json'));
