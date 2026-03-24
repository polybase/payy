ALTER TABLE support_issues
    ADD COLUMN last_ai_support_bot_processed_at TIMESTAMPTZ;

CREATE INDEX idx_support_issues_last_ai_support_bot_processed_at
    ON support_issues (last_ai_support_bot_processed_at);
