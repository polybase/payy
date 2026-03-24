DROP INDEX IF EXISTS idx_ramps_methods_deleted_at;
ALTER TABLE ramps_methods DROP COLUMN deleted_at;
