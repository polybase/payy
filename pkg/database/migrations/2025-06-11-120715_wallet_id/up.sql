-- Add wallet_id column to diagnostics table
ALTER TABLE diagnostics ADD COLUMN wallet_id UUID;

-- Create wallets_addresses table
CREATE TABLE wallets_addresses (
    address CHAR(64) PRIMARY KEY,
    wallet_id UUID NOT NULL
);

-- Insert missing addresses into wallets table from diagnostics
INSERT INTO wallets (id, address, added_at, updated_at)
SELECT 
    gen_random_uuid(),
    d.address,
    NOW(),
    NOW()
FROM diagnostics d
LEFT JOIN wallets w ON d.address = w.address
WHERE w.address IS NULL
ON CONFLICT (address) DO NOTHING;

-- Insert missing addresses into wallets table from rewards
INSERT INTO wallets (id, address, added_at, updated_at)
SELECT 
    gen_random_uuid(),
    r.address,
    NOW(),
    NOW()
FROM rewards r
LEFT JOIN wallets w ON r.address = w.address
WHERE w.address IS NULL
ON CONFLICT (address) DO NOTHING;

-- Insert missing addresses into wallets table from rewards_invites (from_address)
INSERT INTO wallets (id, address, added_at, updated_at)
SELECT 
    gen_random_uuid(),
    ri.from_address,
    NOW(),
    NOW()
FROM rewards_invites ri
LEFT JOIN wallets w ON ri.from_address = w.address
WHERE w.address IS NULL
ON CONFLICT (address) DO NOTHING;

-- Insert missing addresses into wallets table from rewards_invites (to_address)
INSERT INTO wallets (id, address, added_at, updated_at)
SELECT 
    gen_random_uuid(),
    ri.to_address,
    NOW(),
    NOW()
FROM rewards_invites ri
LEFT JOIN wallets w ON ri.to_address = w.address
WHERE w.address IS NULL
ON CONFLICT (address) DO NOTHING;

-- Insert missing addresses into wallets table from rewards_points
INSERT INTO wallets (id, address, added_at, updated_at)
SELECT 
    gen_random_uuid(),
    rp.address,
    NOW(),
    NOW()
FROM rewards_points rp
LEFT JOIN wallets w ON rp.address = w.address
WHERE w.address IS NULL
ON CONFLICT (address) DO NOTHING;

-- Populate wallets_addresses with all wallet data (now includes newly created wallets)
INSERT INTO wallets_addresses (address, wallet_id)
SELECT address, id FROM wallets;

-- Update diagnostics.wallet_id based on address lookup
UPDATE diagnostics 
SET wallet_id = w.id 
FROM wallets w 
WHERE diagnostics.address = w.address;


-- Change rewards table primary key from address to wallet_id
-- First, add the wallet_id column
ALTER TABLE rewards ADD COLUMN wallet_id UUID;


-- Populate wallet_id in rewards table
UPDATE rewards 
SET wallet_id = w.id 
FROM wallets w 
WHERE rewards.address = w.address;

-- Make wallet_id NOT NULL after populating
ALTER TABLE rewards ALTER COLUMN wallet_id SET NOT NULL;


-- Drop the old primary key and create new one for rewards
ALTER TABLE rewards DROP CONSTRAINT rewards_pkey;
ALTER TABLE rewards ADD PRIMARY KEY (wallet_id);

-- Add wallet_id columns to rewards_invites
ALTER TABLE rewards_invites ADD COLUMN from_wallet_id UUID;
ALTER TABLE rewards_invites ADD COLUMN to_wallet_id UUID;

-- Populate wallet_id columns in rewards_invites
UPDATE rewards_invites 
SET from_wallet_id = w.id 
FROM wallets w 
WHERE rewards_invites.from_address = w.address;

UPDATE rewards_invites 
SET to_wallet_id = w.id 
FROM wallets w 
WHERE rewards_invites.to_address = w.address;

-- Make wallet_id columns NOT NULL
ALTER TABLE rewards_invites ALTER COLUMN from_wallet_id SET NOT NULL;
ALTER TABLE rewards_invites ALTER COLUMN to_wallet_id SET NOT NULL;

-- Drop the old primary key constraint and unique constraint
ALTER TABLE rewards_invites DROP CONSTRAINT rewards_invites_pkey;
ALTER TABLE rewards_invites DROP CONSTRAINT rewards_invites_to_address_key;

-- Create new composite primary key
ALTER TABLE rewards_invites ADD PRIMARY KEY (from_wallet_id, to_wallet_id);

-- Add wallet_id column to rewards_points
ALTER TABLE rewards_points ADD COLUMN wallet_id UUID;

-- Populate wallet_id in rewards_points
UPDATE rewards_points 
SET wallet_id = w.id 
FROM wallets w 
WHERE rewards_points.address = w.address;

-- Make wallet_id NOT NULL
ALTER TABLE rewards_points ALTER COLUMN wallet_id SET NOT NULL;

-- Drop NOT NULL for address
ALTER TABLE rewards ALTER COLUMN address DROP NOT NULL;
ALTER TABLE rewards_points ALTER COLUMN address DROP NOT NULL;
ALTER TABLE rewards_invites ALTER COLUMN from_address DROP NOT NULL;
ALTER TABLE rewards_invites ALTER COLUMN to_address DROP NOT NULL;
ALTER TABLE ramps_accounts ALTER COLUMN address DROP NOT NULL;
ALTER TABLE ramps_transactions ALTER COLUMN address DROP NOT NULL;


-- Alter the payments table
ALTER TABLE payments 
ALTER COLUMN payment_by TYPE text;

-- Convert from address to wallet_id
UPDATE payments 
SET payment_by = wallets.id::text
FROM wallets 
WHERE payments.payment_by = wallets.address
  AND payments.payment_by IS NOT NULL;