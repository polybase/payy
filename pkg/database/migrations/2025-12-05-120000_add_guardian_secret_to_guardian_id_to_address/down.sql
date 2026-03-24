DROP INDEX IF EXISTS idx_guardian_id_to_address_guardian_secret;
ALTER TABLE guardian_id_to_address DROP COLUMN "guardian_secret";
