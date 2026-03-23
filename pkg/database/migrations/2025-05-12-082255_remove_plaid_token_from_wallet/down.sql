-- This file should undo anything in `up.sql`
ALTER TABLE wallets
ADD COLUMN plaid_access_token TEXT;
