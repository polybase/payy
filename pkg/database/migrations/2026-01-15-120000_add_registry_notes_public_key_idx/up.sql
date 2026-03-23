CREATE INDEX IF NOT EXISTS registry_notes_public_key_added_at_idx
    ON registry_notes (public_key, added_at DESC);
