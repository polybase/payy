DROP INDEX IF EXISTS unique_wallet_id_provider_country;

CREATE UNIQUE INDEX unique_address_provider_country
ON ramps_accounts(address, provider, (COALESCE(country, 'NULL')));
