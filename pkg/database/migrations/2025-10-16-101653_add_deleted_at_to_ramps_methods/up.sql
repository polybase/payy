ALTER TABLE ramps_methods
    ADD COLUMN deleted_at TIMESTAMPTZ DEFAULT NULL;

CREATE INDEX idx_ramps_methods_deleted_at ON ramps_methods(deleted_at);
