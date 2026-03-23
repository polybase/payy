ALTER TABLE guardian_id_to_address
ADD COLUMN "guardian_secret" TEXT;

ALTER TABLE guardian_id_to_address
ALTER COLUMN guardian_secret SET NOT NULL;

CREATE UNIQUE INDEX idx_guardian_id_to_address_guardian_secret
ON guardian_id_to_address (guardian_secret);
