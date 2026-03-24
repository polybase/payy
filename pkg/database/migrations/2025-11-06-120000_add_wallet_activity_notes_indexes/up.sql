CREATE INDEX idx_wallet_activity_address_updated_at
    ON wallet_activity(address, updated_at DESC);

CREATE INDEX idx_wallet_notes_address_added_at
    ON wallet_notes(address, added_at DESC);
