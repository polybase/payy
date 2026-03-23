ALTER TABLE "wallets" ADD COLUMN "atlas_customer_id" TEXT;

CREATE TABLE "support_messages"(
	"id" UUID NOT NULL PRIMARY KEY,
	"support_issue_id" UUID NOT NULL,
	"external_id" TEXT NOT NULL,
	"emoji" TEXT,
	"role" TEXT NOT NULL,
	"message" TEXT NOT NULL,
	"attachment" JSONB,
	"added_at" TIMESTAMPTZ NOT NULL
);

CREATE TABLE "support_issues"(
	"id" UUID NOT NULL PRIMARY KEY,
	"wallet_id" UUID NOT NULL,
	"external_id" UUID NOT NULL,
	"status" TEXT NOT NULL,
	"channel" TEXT NOT NULL,
	"subject" TEXT,
	"unread_count" INT4 NOT NULL,
	"last_message" TEXT NOT NULL,
	"last_message_at" TIMESTAMPTZ NOT NULL,
	"last_read_at" TIMESTAMPTZ,
	"closed_at" TIMESTAMPTZ,
	"updated_at" TIMESTAMPTZ NOT NULL,
	"added_at" TIMESTAMPTZ NOT NULL
);

CREATE TRIGGER set_updated_at_for_support_issues
BEFORE UPDATE ON support_issues
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();