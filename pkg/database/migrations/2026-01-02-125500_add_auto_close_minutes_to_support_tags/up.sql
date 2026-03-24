ALTER TABLE support_tags ADD COLUMN auto_close_minutes INT;

ALTER TABLE support_issues ADD COLUMN auto_close_at TIMESTAMPTZ;
ALTER TABLE support_issues ADD COLUMN auto_close_minutes INT;

CREATE INDEX idx_support_issues_auto_close_at ON support_issues (auto_close_at)
  WHERE auto_close_at IS NOT NULL AND status != 'CLOSED';
