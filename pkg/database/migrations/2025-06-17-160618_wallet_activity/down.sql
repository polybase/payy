ALTER TABLE "ramps_transactions" DROP COLUMN "private_key";

DROP TABLE IF EXISTS "wallet_activity";
DROP TABLE IF EXISTS "wallet_notes";
DROP TABLE IF EXISTS "registry_notes";
DROP TABLE IF EXISTS "migrate_elements";
