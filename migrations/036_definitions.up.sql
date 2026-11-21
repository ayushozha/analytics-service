-- Event and property definitions (PostHog `EventDefinition` / `PropertyDefinition`):
-- observed names come from the event stream; curated metadata lives in the data
-- dictionary, which gains tags, verified and hidden flags and per-type properties.
ALTER TABLE data_dictionary_entries
    ADD COLUMN IF NOT EXISTS tags TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS verified BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS hidden BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE data_dictionary_entries DROP CONSTRAINT IF EXISTS data_dictionary_entry_type_check;
ALTER TABLE data_dictionary_entries ADD CONSTRAINT data_dictionary_entry_type_check
    CHECK (entry_type IN ('event', 'property', 'person_property', 'session_property', 'metric', 'dimension'));
