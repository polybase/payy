CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create guardian_id_to_ciphertext table
CREATE TABLE ciphertexts (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "ciphertext" TEXT NOT NULL,
    "address" BPCHAR(64) NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE("address")
);

-- Create guardian_types table
CREATE TABLE guardians (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "name" TEXT UNIQUE NOT NULL,
    "display_name" TEXT NOT NULL,
    "ipfs_cid" TEXT NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create guardian_id_to_address table
CREATE TABLE guardian_id_to_address (
    "guardian_id" UUID NOT NULL,
    "ciphertext_id" UUID NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("guardian_id", "ciphertext_id"),
    CONSTRAINT fk_guardian_ciphertext FOREIGN KEY ("guardian_id") REFERENCES guardians("id") ON DELETE CASCADE,
    CONSTRAINT fk_gc_ciphertext FOREIGN KEY ("ciphertext_id") REFERENCES ciphertexts("id") ON DELETE CASCADE
);

-- active guardians will be fetched frequently
CREATE INDEX idx_guardians_is_active ON guardians (is_active);
CREATE INDEX idx_guardian_id_to_address_ciphertext_id ON guardian_id_to_address (ciphertext_id);
