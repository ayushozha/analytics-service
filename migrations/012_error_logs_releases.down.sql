DROP TABLE IF EXISTS log_entries;

DROP INDEX IF EXISTS idx_js_errors_release;
DROP INDEX IF EXISTS idx_js_errors_fingerprint;

ALTER TABLE js_errors
    DROP COLUMN IF EXISTS fingerprint,
    DROP COLUMN IF EXISTS environment,
    DROP COLUMN IF EXISTS release;

DROP TABLE IF EXISTS source_maps;
DROP TABLE IF EXISTS app_releases;
