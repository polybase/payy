-- Your SQL goes here
CREATE TABLE "ramps_accounts"(
    "id" UUID NOT NULL PRIMARY KEY,
    "address" BPCHAR(64) NOT NULL,
    "provider" TEXT NOT NULL,
    "external_id" TEXT,
    "kyc_status" TEXT NOT NULL,
	"kyc_update_required_fields" JSONB,
    "kyc_external_id" TEXT,
    "country" TEXT,
    "deposit_evm_address" TEXT,
    "withdraw_evm_address" TEXT,
    "metadata" JSONB,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX unique_address_provider_country ON ramps_accounts(address bpchar_ops,provider text_ops,(COALESCE(country, 'NULL'::text)) text_ops);

CREATE TABLE "ramps_transactions" (
    "id" UUID NOT NULL PRIMARY KEY,
    "address" BPCHAR(64) NOT NULL,
    "provider" TEXT NOT NULL,
    "account_id" UUID NOT NULL,
    "external_id" TEXT,
    "external_fund_id" TEXT,
    "local_id" TEXT UNIQUE,
    "quote_id" UUID,
    "status" TEXT NOT NULL,
    "from_currency" TEXT NOT NULL,
    "from_amount" NUMERIC NOT NULL,
    "from_network" TEXT NOT NULL,
    "from_network_identifier" JSONB,
    "to_currency" TEXT NOT NULL,
    "to_amount" NUMERIC NOT NULL,
    "to_network" TEXT NOT NULL,
    "to_network_identifier" JSONB,
    "evm_address" TEXT,
	"name" TEXT,
    "memo" TEXT,
    "desc" TEXT,
    "emoji" TEXT,
    "category" TEXT NOT NULL,
    "metadata" JSONB,
    "is_manual" BOOLEAN DEFAULT false,
    "transaction_at" TIMESTAMPTZ,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "ramps_methods"(
    "id" UUID NOT NULL PRIMARY KEY,
    "account_id" UUID NOT NULL,
    "external_id" TEXT,
    "local_id" TEXT NOT NULL,
    "network" TEXT NOT NULL,
    "network_identifier" JSONB NOT NULL,
    "preview" JSONB,
    "metadata" JSONB,
    "is_default" BOOLEAN DEFAULT false NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "ramps_quotes"(
    "id" UUID NOT NULL PRIMARY KEY,
    "provider" TEXT NOT NULL,
    "account_id" UUID NOT NULL,
    "external_id" TEXT NOT NULL,
    "from_currency" TEXT NOT NULL,
    "from_amount" NUMERIC NOT NULL,
    "from_network" TEXT NOT NULL,
    "to_currency" TEXT NOT NULL,
    "to_amount" NUMERIC NOT NULL,
    "to_network" TEXT NOT NULL,
    "metadata" JSONB,
    "expires_at" TIMESTAMPTZ NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL
);