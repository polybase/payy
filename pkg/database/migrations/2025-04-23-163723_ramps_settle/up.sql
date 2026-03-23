-- First, insert missing wallet entries for addresses in ramps_accounts
INSERT INTO wallets (address, added_at, updated_at)
SELECT DISTINCT ra.address, NOW(), NOW()
FROM ramps_accounts ra
WHERE NOT EXISTS (
    SELECT 1 FROM wallets w WHERE ra.address = w.address
);

-- Then, insert missing wallet entries for addresses in ramps_transactions
-- that aren't already in ramps_accounts
INSERT INTO wallets (address, added_at, updated_at)
SELECT DISTINCT rt.address, NOW(), NOW()
FROM ramps_transactions rt
WHERE NOT EXISTS (
    SELECT 1 FROM wallets w WHERE rt.address = w.address
)
AND NOT EXISTS (
    SELECT 1 FROM ramps_accounts ra WHERE rt.address = ra.address
);

-- Add new columns to ramps_transactions
ALTER TABLE "ramps_transactions" ADD COLUMN "funding_status" TEXT;
ALTER TABLE "ramps_transactions" ADD COLUMN "funding_due_amount" NUMERIC;
ALTER TABLE "ramps_transactions" ADD COLUMN "wallet_id" UUID;

-- Add wallet_id column to ramps_accounts
ALTER TABLE "ramps_accounts" ADD COLUMN "wallet_id" UUID;

-- Populate ramps_accounts.wallet_id from wallets table (using address)
UPDATE ramps_accounts
SET wallet_id = wallets.id
FROM wallets
WHERE ramps_accounts.address = wallets.address;

-- Populate ramps_transactions.wallet_id from wallets table based on address
UPDATE ramps_transactions
SET wallet_id = wallets.id
FROM wallets
WHERE ramps_transactions.address = wallets.address;

-- Make wallet_id NOT NULL after population
ALTER TABLE "ramps_accounts" ALTER COLUMN "wallet_id" SET NOT NULL;
ALTER TABLE "ramps_transactions" ALTER COLUMN "wallet_id" SET NOT NULL;

-- Remove is_manual column from ramps_transactions
ALTER TABLE "ramps_transactions" DROP COLUMN "is_manual";