CREATE INDEX IF NOT EXISTS idx_support_issues_wallet_updated_desc
    ON support_issues (wallet_id, updated_at DESC);
