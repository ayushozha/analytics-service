ALTER TABLE shared_dashboards DROP COLUMN IF EXISTS allow_person_data;
ALTER TABLE shared_dashboards DROP COLUMN IF EXISTS insight_id;
ALTER TABLE shared_dashboards DROP COLUMN IF EXISTS dashboard_id;
DROP TABLE IF EXISTS dashboard_tiles;
DROP TABLE IF EXISTS dashboards;
