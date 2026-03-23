DROP INDEX IF EXISTS idx_support_issues_auto_close_at;
ALTER TABLE support_issues DROP COLUMN IF EXISTS auto_close_at;
ALTER TABLE support_issues DROP COLUMN IF EXISTS auto_close_minutes;

ALTER TABLE support_tags DROP COLUMN IF EXISTS auto_close_minutes;
