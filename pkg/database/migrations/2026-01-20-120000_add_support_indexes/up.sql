CREATE INDEX IF NOT EXISTS idx_support_issues_status_updated_at
    ON support_issues (status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_support_issues_wallet_added_at
    ON support_issues (wallet_id, added_at DESC);

CREATE INDEX IF NOT EXISTS idx_support_messages_issue_added_at
    ON support_messages (support_issue_id, added_at ASC);
