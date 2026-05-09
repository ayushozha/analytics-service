-- ============================================================================
-- Migration 006: Session Replay Upsert Support
-- ============================================================================

CREATE UNIQUE INDEX IF NOT EXISTS idx_session_recordings_project_session_unique
    ON session_recordings(project_id, session_id);
