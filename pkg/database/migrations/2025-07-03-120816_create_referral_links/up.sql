CREATE TABLE "udh_referral_links"(
	"posthog_id" TEXT NOT NULL PRIMARY KEY,
	"user_device_hash" TEXT NOT NULL,
	"referrer_url" TEXT NOT NULL,
	"claimed_at" TIMESTAMP WITH TIME ZONE,
	"wallet_id" UUID,
	"added_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
	"updated_at" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION update_udh_referral_links_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
   NEW.updated_at = now();
   RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_updated_at_for_udh_referral_links
BEFORE UPDATE ON udh_referral_links
FOR EACH ROW
EXECUTE FUNCTION update_udh_referral_links_updated_at_column();
