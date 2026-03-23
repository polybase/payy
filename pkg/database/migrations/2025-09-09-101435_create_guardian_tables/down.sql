-- This file should undo anything in `up.sql`

-- Drop tables in reverse order (to handle any potential foreign key dependencies)
DROP TABLE IF EXISTS guardian_types;
DROP TABLE IF EXISTS guardian_id_to_address;
DROP TABLE IF EXISTS guardian_id_to_ciphertext;