ALTER TABLE guardian_id_to_address RENAME TO guardian_to_ciphertext;

ALTER TABLE guardian_to_ciphertext
    RENAME COLUMN guardian_secret TO guardian_auth_value;

ALTER INDEX idx_guardian_id_to_address_ciphertext_id
    RENAME TO idx_guardian_to_ciphertext_ciphertext_id;

ALTER INDEX idx_guardian_id_to_address_guardian_secret
    RENAME TO idx_guardian_to_ciphertext_guardian_auth_value;
