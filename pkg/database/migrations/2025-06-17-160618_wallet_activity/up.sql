ALTER TABLE "ramps_transactions"
ADD COLUMN "private_key" TEXT;

CREATE TABLE "wallet_activity" (
    "id" UUID NOT NULL PRIMARY KEY,
    "address" BPCHAR (64) NOT NULL,
    "kind" TEXT NOT NULL,
    "data" BYTEA NOT NULL,
    "active" BOOL NOT NULL,
    "completed_at" TIMESTAMPTZ,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "wallet_notes" (
    "commitment" BPCHAR (64) NOT NULL PRIMARY KEY,
    "address" BPCHAR (64) NOT NULL,
    "data" BYTEA NOT NULL,
    "status" TEXT NOT NULL,
    "activity_id" UUID,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "registry_notes" (
    "id" UUID NOT NULL PRIMARY KEY,
    "block" BIGINT NOT NULL,
    "public_key" BPCHAR (64) NOT NULL,
    "encrypted_key" BYTEA NOT NULL,
    "encrypted_note" BYTEA NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "migrate_elements" ("element" BPCHAR (64) NOT NULL PRIMARY KEY);
