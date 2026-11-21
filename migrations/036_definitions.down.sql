DELETE FROM data_dictionary_entries WHERE entry_type IN ('person_property', 'session_property');
ALTER TABLE data_dictionary_entries DROP CONSTRAINT IF EXISTS data_dictionary_entry_type_check;
ALTER TABLE data_dictionary_entries ADD CONSTRAINT data_dictionary_entry_type_check
    CHECK (entry_type IN ('event', 'property', 'metric', 'dimension'));
ALTER TABLE data_dictionary_entries
    DROP COLUMN IF EXISTS tags,
    DROP COLUMN IF EXISTS verified,
    DROP COLUMN IF EXISTS hidden;
