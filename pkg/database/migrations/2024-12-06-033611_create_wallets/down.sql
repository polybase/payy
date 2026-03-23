DROP TRIGGER IF EXISTS "set_updated_at_for_wallets" on "wallets";
DROP FUNCTION IF EXISTS "update_updated_at_column";
ALTER TABLE "wallets" DROP CONSTRAINT IF EXISTS "unique_address_expo_push_token";
DROP TABLE IF EXISTS "wallets";
