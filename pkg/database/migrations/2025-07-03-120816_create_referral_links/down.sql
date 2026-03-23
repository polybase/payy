DROP TRIGGER IF EXISTS "set_updated_at_for_udh_referral_links" ON udh_referral_links;
DROP FUNCTION IF EXISTS update_udh_referral_links_updated_at_column();
DROP TABLE IF EXISTS "udh_referral_links";
