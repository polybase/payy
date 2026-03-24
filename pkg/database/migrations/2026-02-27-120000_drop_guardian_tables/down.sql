CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE ciphertexts (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "ciphertext" TEXT NOT NULL,
    "address" TEXT NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "data_to_encrypt_hash" TEXT NOT NULL,
    UNIQUE("address")
);

CREATE TABLE guardians (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "name" TEXT UNIQUE NOT NULL,
    "display_name" TEXT NOT NULL,
    "ipfs_cid" TEXT NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE guardian_to_ciphertext (
    "guardian_id" UUID NOT NULL,
    "ciphertext_id" UUID NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "guardian_auth_value" TEXT NOT NULL,
    PRIMARY KEY ("guardian_id", "ciphertext_id"),
    CONSTRAINT fk_guardian_ciphertext FOREIGN KEY ("guardian_id") REFERENCES guardians("id") ON DELETE CASCADE,
    CONSTRAINT fk_gc_ciphertext FOREIGN KEY ("ciphertext_id") REFERENCES ciphertexts("id") ON DELETE CASCADE
);

CREATE INDEX idx_guardians_is_active ON guardians (is_active);
CREATE INDEX idx_guardian_to_ciphertext_ciphertext_id ON guardian_to_ciphertext (ciphertext_id);
CREATE UNIQUE INDEX idx_guardian_to_ciphertext_guardian_auth_value
    ON guardian_to_ciphertext (guardian_auth_value);
