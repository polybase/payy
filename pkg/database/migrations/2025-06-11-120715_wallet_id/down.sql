ALTER TABLE diagnostics DROP CONSTRAINT IF EXISTS fk_diagnostics_wallet_id;
ALTER TABLE rewards DROP CONSTRAINT IF EXISTS fk_rewards_wallet_id;
ALTER TABLE rewards_invites DROP CONSTRAINT IF EXISTS fk_rewards_invites_from_wallet_id;
ALTER TABLE rewards_invites DROP CONSTRAINT IF EXISTS fk_rewards_invites_to_wallet_id;
ALTER TABLE rewards_points DROP CONSTRAINT IF EXISTS fk_rewards_points_wallet_id;

-- Remove wallet_id column from diagnostics
ALTER TABLE diagnostics DROP COLUMN wallet_id;

-- Drop wallets_addresses table
DROP TABLE wallets_addresses;

-- Revert rewards table changes
-- Drop current primary key and restore address as primary key
ALTER TABLE rewards DROP CONSTRAINT rewards_pkey;
ALTER TABLE rewards ADD PRIMARY KEY (address);
ALTER TABLE rewards DROP COLUMN wallet_id;

-- Revert rewards_invites table changes
-- Drop current primary key and restore address-based primary key
ALTER TABLE rewards_invites DROP CONSTRAINT rewards_invites_pkey;
ALTER TABLE rewards_invites ADD PRIMARY KEY (from_address, to_address);
ALTER TABLE rewards_invites DROP COLUMN from_wallet_id;
ALTER TABLE rewards_invites DROP COLUMN to_wallet_id;

-- Remove wallet_id column from rewards_points
ALTER TABLE rewards_points DROP COLUMN wallet_id;