DROP INDEX IF EXISTS unique_address_provider_country;

CREATE UNIQUE INDEX unique_wallet_id_provider_country
ON ramps_accounts(wallet_id, provider, (COALESCE(country, 'NULL')));

