CREATE TABLE "wallet_auths"(
	"id" UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
	"wallet_id" UUID NOT NULL,
	"kind" TEXT NOT NULL,
	"value" TEXT NOT NULL,
	"enabled" BOOLEAN NOT NULL DEFAULT true,
	"added_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
	"updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

	CONSTRAINT fk_wallet_auths_wallet_id FOREIGN KEY ("wallet_id") REFERENCES "wallets"("id") ON DELETE CASCADE
);

-- Create index for efficient wallet_id lookups
CREATE INDEX idx_wallet_auths_wallet_id ON "wallet_auths"("wallet_id");

-- Create index for enabled auths
CREATE INDEX idx_wallet_auths_wallet_id_enabled ON "wallet_auths"("wallet_id", "enabled") WHERE enabled = true;

-- Create index for lookup
CREATE INDEX idx_kind_value_enabled ON "wallet_auths"("kind", "value", "enabled") WHERE enabled = true;

-- trigger to automatically update the `update_at` column when a row changes
CREATE TRIGGER set_updated_at_for_wallet_auths
BEFORE UPDATE ON wallet_auths
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
