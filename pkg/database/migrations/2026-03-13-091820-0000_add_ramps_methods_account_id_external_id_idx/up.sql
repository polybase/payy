CREATE INDEX IF NOT EXISTS idx_ramps_methods_account_id_external_id
    ON ramps_methods (account_id, external_id);
