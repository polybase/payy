CREATE INDEX IF NOT EXISTS notes_status_owner_version_value_idx
    ON notes(status, owner_id, version, value);
