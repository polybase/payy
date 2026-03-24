CREATE TABLE "wallets"(
	"id" UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
	"address" BPCHAR(64) NOT NULL,
	"expo_push_token" TEXT NOT NULL,
	"deposit_address" BPCHAR(64) NOT NULL,
	"added_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
	"updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

	CONSTRAINT unique_address_expo_push_token UNIQUE ("address", "expo_push_token")
);

-- trigger to automatically update the `update_at` column when a row changes (because of a change in expo_push_token, for instance)
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
	NEW.updated_at = NOW();
	RETURN NEW;
END;
$$ language plpgsql;

CREATE TRIGGER set_updated_at_for_wallets
BEFORE UPDATE ON wallets
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
