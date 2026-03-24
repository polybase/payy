ALTER TABLE guardian_to_ciphertext RENAME TO guardian_id_to_address;

ALTER TABLE guardian_id_to_address
    RENAME COLUMN guardian_auth_value TO guardian_secret;

ALTER INDEX idx_guardian_to_ciphertext_ciphertext_id
    RENAME TO idx_guardian_id_to_address_ciphertext_id;

ALTER INDEX idx_guardian_to_ciphertext_guardian_auth_value
    RENAME TO idx_guardian_id_to_address_guardian_secret;
