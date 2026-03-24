CREATE INDEX IF NOT EXISTS rewards_invites_to_wallet_added_at_idx
    ON rewards_invites (to_wallet_id, added_at ASC)
    INCLUDE (from_wallet_id);
