CREATE TABLE IF NOT EXISTS privacy_settings (
    project_id UUID PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    anonymize_ip BOOLEAN NOT NULL DEFAULT true,
    respect_dnt BOOLEAN NOT NULL DEFAULT true,
    bot_filtering BOOLEAN NOT NULL DEFAULT true,
    consent_required BOOLEAN NOT NULL DEFAULT false,
    allowed_consent_modes TEXT[] NOT NULL DEFAULT ARRAY['analytics', 'measurement', 'all'],
    blocked_user_agents TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
