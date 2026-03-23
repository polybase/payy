DROP INDEX IF EXISTS idx_support_issues_last_ai_support_bot_processed_at;

ALTER TABLE support_issues
    DROP COLUMN last_ai_support_bot_processed_at;
