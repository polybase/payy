CREATE INDEX IF NOT EXISTS idx_ramps_accounts_provider_external
    ON ramps_accounts (provider, external_id);
