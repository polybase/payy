CREATE INDEX IF NOT EXISTS ramps_transactions_account_funding_idx 
ON ramps_transactions(account_id, funding_status) 
INCLUDE (funding_due_amount);
